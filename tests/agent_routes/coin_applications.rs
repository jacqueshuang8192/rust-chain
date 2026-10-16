use super::*;

fn token(agent: AgentFixture) -> String {
    issue_token(
        &test_settings(),
        format!("agent:{}", agent.admin_user_id),
        TokenScope::Agent,
        900,
    )
    .unwrap()
}
async fn request(
    app: &axum::Router,
    token: &str,
    method: &str,
    uri: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| json!({"body":String::from_utf8_lossy(&bytes)})),
    )
}
fn ok(result: (StatusCode, Value)) -> Value {
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    result.1
}
async fn count(pool: &MySqlPool, table: &str, symbol: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE symbol=?"))
        .bind(symbol)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn coin_applications_enforce_ownership_and_atomic_publication() {
    let Some(pool) = mysql_pool().await else {
        return;
    };
    let a = create_agent(&pool, "coin-a").await;
    let b = create_agent(&pool, "coin-b").await;
    // A second portal account on the same agent is not the original applicant.
    let peer=sqlx::query("INSERT INTO agent_admin_users (agent_id,username,password_hash,status) VALUES (?,?,'test','active')")
        .bind(a.agent_id).bind(format!("coin-peer-{}",Uuid::now_v7())).execute(&pool).await.unwrap().last_insert_id();
    let (role, admin, _) = create_admin_with_password(&pool, "coin-review", "test-password").await;
    let admin_token = issue_token(
        &test_settings(),
        format!("admin:{admin}"),
        TokenScope::Admin,
        900,
    )
    .unwrap();
    let app = build_router(AppState::new(test_settings()).with_mysql(pool.clone()));
    let quote = sqlx::query(
        "INSERT INTO assets(symbol,name,precision_scale,status) VALUES (?,'quote',18,'active')",
    )
    .bind(format!("Q{}", &Uuid::now_v7().simple().to_string()[16..]))
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();
    let symbol = format!("C{}", &Uuid::now_v7().simple().to_string()[16..]);
    let input = json!({"symbol":symbol,"name":"Test issuance","precision_scale":18,"total_supply":"100.000000000000000001","issue_price":"0.000000000000000001","quote_asset_id":quote,"listed_at":1900000000000i64,"reason":"review this project","idempotency_key":"coin-intent"});
    let route = "/agent/api/v1/coin-applications";
    let at = token(a);
    let pending = ok(request(&app, &at, "POST", route, input.clone()).await);
    assert_eq!(pending["status"], "pending");
    assert_eq!(pending["total_supply"], input["total_supply"]);
    assert_eq!(
        BigDecimal::from_str(pending["issue_price"].as_str().unwrap()).unwrap(),
        BigDecimal::from_str(input["issue_price"].as_str().unwrap()).unwrap()
    );
    assert!(pending["project_id"].is_null());
    assert!(pending.get("idempotency_key").is_none());
    assert_eq!(count(&pool, "assets", &symbol).await, 0);
    assert_eq!(count(&pool, "new_coin_projects", &symbol).await, 0);
    let id = pending["id"].as_u64().unwrap();
    let detail = format!("{route}/{id}");
    assert_eq!(
        ok(request(&app, &at, "POST", route, input.clone()).await)["id"],
        id
    );
    let mut changed = input.clone();
    changed["name"] = json!("changed");
    assert_eq!(
        request(&app, &at, "POST", route, changed).await.0,
        StatusCode::CONFLICT
    );
    let own = ok(request(&app, &at, "GET", route, json!(null)).await);
    assert_eq!(own["total"], 1);
    assert_eq!(own["applications"][0]["id"], id);
    for denied in [
        token(b),
        issue_token(
            &test_settings(),
            format!("agent:{peer}"),
            TokenScope::Agent,
            900,
        )
        .unwrap(),
    ] {
        assert_eq!(
            request(&app, &denied, "GET", &detail, json!(null)).await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ok(request(&app, &denied, "GET", route, json!(null)).await)["total"],
            0
        );
    }
    assert_eq!(
        request(
            &app,
            &at,
            "GET",
            &format!("{route}?agent_id={}", b.agent_id),
            json!(null)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let review = format!("/admin/api/v1/agent-coin-applications/{id}/review");
    assert_eq!(
        request(
            &app,
            &at,
            "POST",
            &review,
            json!({"decision":"approve","reason":"ok"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE admin_roles SET permissions=JSON_ARRAY('new_coin.applications.read','new_coin.projects.write') WHERE id=?").bind(role).execute(&pool).await.unwrap();
    assert_eq!(
        request(
            &app,
            &admin_token,
            "POST",
            &review,
            json!({"decision":"approve","reason":"ok"})
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE admin_roles SET permissions=JSON_ARRAY('new_coin.applications.read','new_coin.applications.review') WHERE id=?").bind(role).execute(&pool).await.unwrap();
    let decision = json!({"decision":"approve","reason":"checked issuance"});
    let (r1, r2) = tokio::join!(
        request(&app, &admin_token, "POST", &review, decision.clone()),
        request(&app, &admin_token, "POST", &review, decision.clone())
    );
    let first = ok(r1);
    let second = ok(r2);
    assert_eq!(first["project_id"], second["project_id"]);
    assert_eq!(first["status"], "approved");
    assert_eq!(first["reviewed_by"], admin);
    assert!(first["reviewed_at"].is_i64());
    assert_eq!(count(&pool, "assets", &symbol).await, 1);
    assert_eq!(count(&pool, "new_coin_projects", &symbol).await, 1);
    let state:(String,bool,bool,bool)=sqlx::query_as("SELECT p.lifecycle_status,a.deposit_enabled,a.withdraw_enabled,a.margin_transfer_enabled FROM new_coin_projects p JOIN assets a ON a.id=p.asset_id WHERE p.id=?").bind(first["project_id"].as_u64().unwrap()).fetch_one(&pool).await.unwrap();
    assert_eq!(state, ("preheat".into(), false, false, false));
    assert_eq!(
        request(
            &app,
            &admin_token,
            "POST",
            &review,
            json!({"decision":"reject","reason":"late"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let receipt = ok(request(&app, &at, "GET", &detail, json!(null)).await);
    assert_eq!(receipt["review_reason"], "checked issuance");
    assert_eq!(receipt["status"], "approved");
    assert_eq!(
        ok(request(
            &app,
            &at,
            "GET",
            &format!("{route}?status=pending"),
            json!(null)
        )
        .await)["total"],
        0
    );
    assert_eq!(
        ok(request(
            &app,
            &at,
            "GET",
            &format!("{route}?status=approved"),
            json!(null)
        )
        .await)["total"],
        1
    );
    // Rejection leaves no asset or project, with the reason visible to the applicant.
    let mut rejected = input.clone();
    rejected["symbol"] = json!(format!("R{symbol}"));
    rejected["idempotency_key"] = json!("rejected");
    let row = ok(request(&app, &at, "POST", route, rejected.clone()).await);
    let review2 = format!("/admin/api/v1/agent-coin-applications/{}/review", row["id"]);
    let result = ok(request(
        &app,
        &admin_token,
        "POST",
        &review2,
        json!({"decision":"reject","reason":"资料不完整"}),
    )
    .await);
    assert_eq!(result["status"], "rejected");
    assert_eq!(result["review_reason"], "资料不完整");
    assert_eq!(
        count(&pool, "assets", rejected["symbol"].as_str().unwrap()).await,
        0
    );
    // Exact raw submission types, bounded pagination and privilege fields fail closed.
    for patch in [
        json!({"total_supply":1}),
        json!({"agent_id":b.agent_id}),
        json!({"status":"approved"}),
        json!({"issue_price":"1e100"}),
    ] {
        let mut invalid = input.clone();
        invalid
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        assert_eq!(
            request(&app, &at, "POST", route, invalid).await.0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    for query in ["limit=0", "limit=101", "offset=100001", "status=listed"] {
        assert_eq!(
            request(&app, &at, "GET", &format!("{route}?{query}"), json!(null))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    // Approval audit failure must roll back the new asset, project and receipt.
    let mut rollback = input.clone();
    rollback["symbol"] = json!(format!("F{symbol}"));
    rollback["idempotency_key"] = json!("rollback");
    let row = ok(request(&app, &at, "POST", route, rollback.clone()).await);
    sqlx::raw_sql("CREATE TRIGGER fail_coin_review BEFORE INSERT ON admin_audit_logs FOR EACH ROW BEGIN IF NEW.action='new_coin_application.review' THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='test audit failure'; END IF; END").execute(&pool).await.unwrap();
    let failed = request(
        &app,
        &admin_token,
        "POST",
        &format!("/admin/api/v1/agent-coin-applications/{}/review", row["id"]),
        decision,
    )
    .await;
    sqlx::raw_sql("DROP TRIGGER fail_coin_review")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(failed.0, StatusCode::INTERNAL_SERVER_ERROR, "{}", failed.1);
    assert_eq!(
        count(&pool, "assets", rollback["symbol"].as_str().unwrap()).await,
        0
    );
    assert_eq!(
        count(
            &pool,
            "new_coin_projects",
            rollback["symbol"].as_str().unwrap()
        )
        .await,
        0
    );
    assert_eq!(
        ok(request(
            &app,
            &at,
            "GET",
            &format!("{route}/{}", row["id"]),
            json!(null)
        )
        .await)["status"],
        "pending"
    );
    // Disabling the issuing agent blocks approval; rejection can still close the application.
    sqlx::query("UPDATE agents SET status='disabled' WHERE id=?")
        .bind(a.agent_id)
        .execute(&pool)
        .await
        .unwrap();
    let review3 = format!("/admin/api/v1/agent-coin-applications/{}/review", row["id"]);
    assert_eq!(
        request(
            &app,
            &admin_token,
            "POST",
            &review3,
            json!({"decision":"approve","reason":"blocked"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        ok(request(
            &app,
            &admin_token,
            "POST",
            &review3,
            json!({"decision":"reject","reason":"代理停用"})
        )
        .await)["status"],
        "rejected"
    );
}
