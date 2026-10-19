use super::new_coin_market::call;
use super::*;

fn policy() -> Value {
    json!({"version":2,"timezone":"Asia/Hong_Kong","purchase_initial_price":"0.3","listing_reference_price":"9.99",
      "subscription":{"automatic":true,"at":1800000000000i64},"distribution":{"automatic":false,"at":1800001000000i64},
      "purchase":{"automatic":true,"at":1800002000000i64},"listing":{"automatic":false,"at":1800003000000i64},
      "lock":{"kind":"on_purchase"},"fee":{"kind":"profit","rate":"0.2"}})
}
async fn input(pool: &MySqlPool) -> Value {
    let suffix = Uuid::now_v7().simple().to_string();
    let mut assets = Vec::new();
    for prefix in ["B", "Q"] {
        assets.push(sqlx::query("INSERT INTO assets(symbol,name,precision_scale,status) VALUES (?,'admin issuance test',8,'active')")
            .bind(format!("{prefix}{}", &suffix[20..])).execute(pool).await.unwrap().last_insert_id());
    }
    json!({"asset_id":assets[0],"quote_asset_id":assets[1],"total_supply":"1000000","issue_price":"0.2",
        "issuance_policy":policy(),"reason":"admin direct issuance","idempotency_key":Uuid::now_v7().to_string()})
}
fn ok(result: (StatusCode, Value)) -> Value {
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    result.1
}
async fn role(pool: &MySqlPool, id: u64, permissions: Value) {
    sqlx::query("UPDATE admin_roles SET permissions=? WHERE id=?")
        .bind(SqlxJson(permissions))
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn admin_token_creation_is_atomic_canonical_and_recoverable() -> Result<(), Box<dyn Error>> {
    let Some(pool) = mysql_pool().await else {
        return Ok(());
    };
    let (role_id, admin) = create_admin_user(&pool).await;
    let user = create_user(&pool).await;
    let token = issue_token(
        &test_settings(),
        format!("admin:{admin}"),
        TokenScope::Admin,
        900,
    )?;
    let app = build_router(AppState::new(test_settings()).with_mysql(pool.clone()));
    let body = input(&pool).await;
    let path = "/admin/api/v1/new-coins/issuance";
    role(&pool, role_id, json!(["new_coin.projects.read"])).await;
    assert_eq!(
        call(&app, &token, "POST", path, body.clone()).await.0,
        StatusCode::FORBIDDEN
    );
    role(&pool, role_id, json!(["new_coin.projects.write"])).await;
    for invalid in [
        json!({"total_supply":1}),
        json!({"issue_price":"0.000000001"}),
        json!({"lifecycle_status":"listed"}),
        json!({"quote_asset_id":body["asset_id"]}),
    ] {
        let mut bad = body.clone();
        bad.as_object_mut()
            .unwrap()
            .extend(invalid.as_object().unwrap().clone());
        assert!(matches!(
            call(&app, &token, "POST", path, bad).await.0,
            StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
        ));
    }
    let trigger = format!("admin_token_audit_{admin}");
    sqlx::raw_sql(&format!("CREATE TRIGGER {trigger} BEFORE INSERT ON admin_audit_logs FOR EACH ROW BEGIN IF NEW.admin_id={admin} AND NEW.action='new_coin.issuance.create' THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='test rollback'; END IF; END")).execute(&pool).await?;
    let failed = call(&app, &token, "POST", path, body.clone()).await;
    sqlx::raw_sql(&format!("DROP TRIGGER {trigger}"))
        .execute(&pool)
        .await?;
    assert_eq!(failed.0, StatusCode::INTERNAL_SERVER_ERROR);
    let base = body["asset_id"].as_u64().unwrap();
    let counts: (i64,i64,i64,i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM new_coin_projects WHERE asset_id=?),(SELECT COUNT(*) FROM trading_pairs WHERE base_asset=?),(SELECT COUNT(*) FROM wallet_accounts WHERE asset_id=?),(SELECT COUNT(*) FROM admin_token_issuance_receipts WHERE admin_id=?)")
        .bind(base).bind(base).bind(base).bind(admin).fetch_one(&pool).await?;
    assert_eq!(counts, (0, 0, 0, 0));
    let (a, b) = tokio::join!(
        call(&app, &token, "POST", path, body.clone()),
        call(&app, &token, "POST", path, body.clone())
    );
    let created = ok(a);
    assert_eq!(ok(b), created);
    assert_eq!(created["lifecycle_status"], "preheat");
    assert_eq!(
        created["issuance_policy"]["distribution"]["automatic"],
        false
    );
    assert_eq!(created["next_transition_at"], 1800000000000i64);
    assert!(created["actual_purchase_at"].is_null());
    assert!(created["issuer_agent_id"].is_null());
    assert!(created["quote_symbol"].is_string());
    let project = created["id"].as_u64().unwrap();
    let pair = created["post_listing_pair_id"].as_u64().unwrap();
    let state: (String,bool,BigDecimal,i64) = sqlx::query_as("SELECT p.status,g.enabled,g.initial_price,(SELECT COUNT(*) FROM market_default_generator_versions WHERE pair_id=p.id) FROM trading_pairs p JOIN market_default_generators g ON g.pair_id=p.id WHERE p.id=?")
        .bind(pair).fetch_one(&pool).await?;
    assert_eq!(state, ("disabled".into(), false, decimal("0.3"), 1));
    let balances: (BigDecimal, BigDecimal, BigDecimal) = sqlx::query_as(
        "SELECT available,frozen,locked FROM wallet_accounts WHERE user_id=? AND asset_id=?",
    )
    .bind(user)
    .bind(base)
    .fetch_one(&pool)
    .await?;
    assert_eq!(balances, (decimal("0"), decimal("0"), decimal("0")));
    let audits: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin_audit_logs WHERE admin_id=? AND action='new_coin.issuance.create'").bind(admin).fetch_one(&pool).await?;
    assert_eq!(audits, 1);
    let mut canonical = body.clone();
    canonical["issue_price"] = json!("0.20000000");
    canonical["issuance_policy"]["fee"]["rate"] = json!("0.2000");
    sqlx::query("UPDATE new_coin_projects SET lifecycle_status='subscription' WHERE id=?")
        .bind(project)
        .execute(&pool)
        .await?;
    assert_eq!(
        ok(call(&app, &token, "POST", path, canonical).await)["id"],
        project
    );
    let mut changed = body.clone();
    changed["total_supply"] = json!("2000000");
    assert_eq!(
        call(&app, &token, "POST", path, changed).await.0,
        StatusCode::CONFLICT
    );
    let mut duplicate = body.clone();
    duplicate["idempotency_key"] = json!(Uuid::now_v7().to_string());
    assert_eq!(
        call(&app, &token, "POST", path, duplicate).await.0,
        StatusCode::CONFLICT
    );
    let receipt_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM admin_token_issuance_receipts WHERE admin_id=?")
            .bind(admin)
            .fetch_one(&pool)
            .await?;
    assert_eq!(receipt_count, 1);
    Ok(())
}

#[tokio::test]
async fn admin_token_records_separate_ownership_and_only_show_executed_buybacks()
-> Result<(), Box<dyn Error>> {
    let Some(pool) = mysql_pool().await else {
        return Ok(());
    };
    let (role_id, admin) = create_admin_user(&pool).await;
    let token = issue_token(
        &test_settings(),
        format!("admin:{admin}"),
        TokenScope::Admin,
        900,
    )?;
    let app = build_router(AppState::new(test_settings()).with_mysql(pool.clone()));
    let body = input(&pool).await;
    let created = ok(call(
        &app,
        &token,
        "POST",
        "/admin/api/v1/new-coins/issuance",
        body.clone(),
    )
    .await);
    let project = created["id"].as_u64().unwrap();
    let mut agents = Vec::new();
    for _ in 0..2 {
        let user = create_user(&pool).await;
        agents.push(
            sqlx::query("INSERT INTO agents(user_id,agent_code,path) VALUES (?,?,'/')")
                .bind(user)
                .bind(Uuid::now_v7().to_string())
                .execute(&pool)
                .await?
                .last_insert_id(),
        );
    }
    sqlx::query("UPDATE new_coin_projects SET issuer_agent_id=?,actual_purchase_at='2026-10-19 00:00:00' WHERE id=?").bind(agents[0]).bind(project).execute(&pool).await?;
    let buyer = create_user(&pool).await;
    sqlx::query("INSERT INTO user_referrals(user_id,root_agent_id,path) VALUES (?,?,'/')")
        .bind(buyer)
        .bind(agents[1])
        .execute(&pool)
        .await?;
    let mut ids = Vec::new();
    for state in ["held", "sold"] {
        ids.push(sqlx::query("INSERT INTO new_coin_holdings(project_id,user_id,source_kind,source_id,quantity,cost,policy_json,acquired_at,unlock_at,status,sold_at) VALUES (?,?,'purchase',?,2500,500,?,'2026-10-18 00:00:00',NULL,?,IF(?='sold','2026-10-19 01:00:00',NULL))")
            .bind(project).bind(buyer).bind(buyer*10+ids.len() as u64).bind(SqlxJson(policy())).bind(state).bind(state).execute(&pool).await?.last_insert_id());
    }
    let mut executed_id = String::new();
    for (side, status) in [
        ("sell", "quoted"),
        ("buy", "executed"),
        ("sell", "executed"),
    ] {
        let id = Uuid::now_v7().to_string();
        if side == "sell" && status == "executed" {
            executed_id = id.clone();
        }
        sqlx::query("INSERT INTO new_coin_trade_quotes(id,user_id,project_id,holding_id,side,quantity,price,gross,cost,fee,net,source,source_version,observed_at,expires_at,idempotency_key,intent_json,status,executed_at) VALUES (?,?,?,?,?,2500,0.4,1000,500,100,900,'default','default:2','2026-10-19 01:00:00','2026-10-19 01:00:15',?,JSON_OBJECT(),?,IF(?='executed','2026-10-19 01:00:01',NULL))")
            .bind(&id).bind(buyer).bind(project).bind(ids[1]).bind(side).bind(id.as_bytes()).bind(status).bind(status).execute(&pool).await?;
    }
    for (actor, id) in [
        ("agent", Some(agents[1])),
        ("scheduler", None),
        ("user", Some(buyer)),
    ] {
        sqlx::query("INSERT INTO new_coin_operations(project_id,actor_type,actor_id,action,payload_json,reason) VALUES (?,?,?,'new_coin.stage',JSON_OBJECT(),'fixture')").bind(project).bind(actor).bind(id).execute(&pool).await?;
    }
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallet_ledger")
        .fetch_one(&pool)
        .await?;
    for (resource, permission) in [
        ("holdings", "new_coin.locks.read"),
        ("buybacks", "new_coin.unlocks.read"),
        ("operations", "audit.logs.read"),
    ] {
        let url = format!("/admin/api/v1/new-coins/{resource}?project_id={project}");
        role(&pool, role_id, json!(["new_coin.projects.read"])).await;
        assert_eq!(
            call(&app, &token, "GET", &url, Value::Null).await.0,
            StatusCode::FORBIDDEN
        );
        role(&pool, role_id, json!([permission])).await;
        let rows = ok(call(&app, &token, "GET", &url, Value::Null).await);
        assert_eq!(
            rows["total"],
            match resource {
                "holdings" => 2,
                "buybacks" => 1,
                _ => 4,
            }
        );
        assert_eq!(
            call(
                &app,
                &token,
                "GET",
                &format!("{url}&limit=101"),
                Value::Null
            )
            .await
            .0,
            StatusCode::BAD_REQUEST
        );
    }
    role(
        &pool,
        role_id,
        json!([
            "new_coin.locks.read",
            "new_coin.unlocks.read",
            "audit.logs.read"
        ]),
    )
    .await;
    let url = format!(
        "/admin/api/v1/new-coins/holdings?project_id={project}&agent_id={}&issuer_agent_id={}&user_id={buyer}&limit=1",
        agents[1], agents[0]
    );
    let first = ok(call(&app, &token, "GET", &url, Value::Null).await);
    assert_eq!(first["total"], 2);
    assert_eq!(first["holdings"][0]["id"], ids[1]);
    assert_eq!(first["holdings"][0]["unlock_at"], 1792368000000i64);
    assert_eq!(first["holdings"][0]["owner_agent_id"], agents[1]);
    let second = ok(call(&app, &token, "GET", &format!("{url}&offset=1"), Value::Null).await);
    assert_eq!(second["holdings"][0]["id"], ids[0]);
    assert_eq!(second["total"], 2);
    let empty = ok(call(
        &app,
        &token,
        "GET",
        &format!("{url}&status=held&offset=1"),
        Value::Null,
    )
    .await);
    assert_eq!(empty["total"], 1);
    assert!(empty["holdings"].as_array().unwrap().is_empty());
    let wrong_owner = ok(call(
        &app,
        &token,
        "GET",
        &format!(
            "/admin/api/v1/new-coins/holdings?project_id={project}&agent_id={}",
            agents[0]
        ),
        Value::Null,
    )
    .await);
    assert_eq!(wrong_owner["total"], 0);
    let sold = ok(call(
        &app,
        &token,
        "GET",
        &format!("/admin/api/v1/new-coins/buybacks?project_id={project}&user_id={buyer}"),
        Value::Null,
    )
    .await);
    assert_eq!(sold["buybacks"][0]["id"], executed_id);
    for (field, amount) in [
        ("cost", "500"),
        ("gross", "1000"),
        ("fee", "100"),
        ("net", "900"),
    ] {
        assert_eq!(
            decimal(sold["buybacks"][0][field].as_str().unwrap()),
            decimal(amount)
        );
    }
    let ops=ok(call(&app,&token,"GET",&format!("/admin/api/v1/new-coins/operations?project_id={project}&actor_type=scheduler&action=new_coin.stage"),Value::Null).await);
    assert_eq!(ops["total"], 1);
    assert!(ops["operations"][0]["actor_id"].is_null());
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallet_ledger")
        .fetch_one(&pool)
        .await?;
    assert_eq!(before, after);
    Ok(())
}

#[tokio::test]
async fn admin_token_market_is_pinned_to_contract_and_actual_purchase() -> Result<(), Box<dyn Error>>
{
    let Some(pool) = mysql_pool().await else {
        return Ok(());
    };
    let (_, admin) = create_admin_user(&pool).await;
    let token = issue_token(
        &test_settings(),
        format!("admin:{admin}"),
        TokenScope::Admin,
        900,
    )?;
    let app = build_router(AppState::new(test_settings()).with_mysql(pool.clone()));
    let created = ok(call(
        &app,
        &token,
        "POST",
        "/admin/api/v1/new-coins/issuance",
        input(&pool).await,
    )
    .await);
    let project = created["id"].as_u64().unwrap();
    let pair = created["post_listing_pair_id"].as_u64().unwrap();
    let path = format!("/admin/api/v1/new-coins/{project}/market-setup");
    let read = ok(call(&app, &token, "GET", &path, Value::Null).await);
    assert_eq!(read["pairs"].as_array().unwrap().len(), 1);
    assert_eq!(read["pairs"][0]["id"], pair);
    let mut draft = json!({"pair_id":pair,"expected_revision":0,"expected_market_version":1,"initial_price":"0.4","config":{"volume_min":"0","volume_max":"0"},"reason":"draft"});
    assert_eq!(
        call(&app, &token, "PATCH", &path, draft.clone()).await.0,
        StatusCode::BAD_REQUEST
    );
    draft["initial_price"] = json!("0.3");
    ok(call(&app, &token, "PATCH", &path, draft.clone()).await);
    let publish = json!({"pair_id":pair,"expected_revision":1,"reason":"publish"});
    assert_eq!(
        call(
            &app,
            &token,
            "POST",
            &format!("{path}/publish"),
            publish.clone()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE new_coin_projects SET lifecycle_status='purchase' WHERE id=?")
        .bind(project)
        .execute(&pool)
        .await?;
    assert_eq!(
        call(
            &app,
            &token,
            "POST",
            &format!("{path}/publish"),
            publish.clone()
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE new_coin_projects SET actual_purchase_at=UTC_TIMESTAMP(6) WHERE id=?")
        .bind(project)
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE trading_pairs SET status='active' WHERE id=?")
        .bind(pair)
        .execute(&pool)
        .await?;
    let published = ok(call(
        &app,
        &token,
        "POST",
        &format!("{path}/publish"),
        publish.clone(),
    )
    .await);
    assert_eq!(published["published_version"], 2);
    assert_eq!(
        ok(call(&app, &token, "POST", &format!("{path}/publish"), publish).await)["replayed"],
        true
    );
    Ok(())
}
