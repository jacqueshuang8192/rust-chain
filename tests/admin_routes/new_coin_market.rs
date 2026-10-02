use super::*;

async fn call(
    app: &axum::Router,
    token: &str,
    method: &str,
    path: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1_048_576)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (status, body)
}

#[tokio::test]
async fn new_coin_market_drafts_publish_atomically_and_replay_without_reenabling()
-> Result<(), Box<dyn Error>> {
    let Some(pool) = mysql_pool().await else {
        return Ok(());
    };
    let (role, admin) = create_admin_user(&pool).await;
    let settings = test_settings();
    let token = issue_token(&settings, format!("admin:{admin}"), TokenScope::Admin, 900)?;
    let app = build_router(AppState::new(settings).with_mysql(pool.clone()));
    let suffix = Uuid::now_v7().simple().to_string();
    let symbol = format!("NM{}", &suffix[16..32]).to_uppercase();
    let base = create_asset(&pool, &format!("NMB{}", &suffix[16..24])).await;
    let quote = create_asset(&pool, &format!("NMQ{}", &suffix[24..32])).await;
    let pair = sqlx::query("INSERT INTO trading_pairs (base_asset,quote_asset,symbol,price_precision,qty_precision,min_order_value,status,market_type) VALUES (?,?,?,18,18,1,'active','strategy')")
        .bind(base).bind(quote).bind(&symbol).execute(&pool).await?.last_insert_id();
    let project = sqlx::query("INSERT INTO new_coin_projects (asset_id,quote_asset_id,symbol,total_supply,remaining_supply,issue_price,unlock_type,relative_unlock_seconds) VALUES (?,?,?,100,100,1,'relative_period',60)")
        .bind(base).bind(quote).bind(format!("NP{suffix}")).execute(&pool).await?.last_insert_id();
    let path = format!("/admin/api/v1/new-coins/{project}/market-setup");
    let live = format!("/admin/api/v1/market-pairs/{pair}/default-generator");
    let body = json!({"pair_id":pair,"expected_revision":0,"expected_market_version":0,
        "initial_price":"1.000000000000000001","config":{"volume_min":"0","volume_max":"0"},"reason":"save draft"});
    let (status, read) = call(&app, &token, "GET", &path, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    assert_eq!(read["pairs"][0]["id"], pair);
    assert!(read["draft"].is_null());

    for permissions in [
        json!(["new_coin.projects.read", "new_coin.projects.write"]),
        json!(["market.pairs.read", "market.pairs.write"]),
    ] {
        sqlx::query("UPDATE admin_roles SET permissions=? WHERE id=?")
            .bind(SqlxJson(permissions))
            .bind(role)
            .execute(&pool)
            .await?;
        assert_eq!(
            call(&app, &token, "GET", &path, Value::Null).await.0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            call(&app, &token, "PATCH", &path, body.clone()).await.0,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            call(
                &app,
                &token,
                "POST",
                &format!("{path}/publish"),
                json!({"pair_id":pair,"expected_revision":1,"reason":"publish"})
            )
            .await
            .0,
            StatusCode::FORBIDDEN
        );
    }
    sqlx::query("UPDATE admin_roles SET permissions=JSON_ARRAY('*') WHERE id=?")
        .bind(role)
        .execute(&pool)
        .await?;
    let (status, draft) = call(&app, &token, "PATCH", &path, body.clone()).await;
    assert_eq!(status, StatusCode::OK, "{draft}");
    assert_eq!(draft["revision"], 1);
    assert_eq!(
        decimal(draft["initial_price"].as_str().unwrap()),
        decimal("1.000000000000000001")
    );
    assert_eq!(
        call(&app, &token, "PATCH", &path, body.clone()).await.0,
        StatusCode::CONFLICT
    );
    let (_, runtime) = call(&app, &token, "GET", &live, Value::Null).await;
    assert_eq!(runtime["configured"], false);
    let counts: (i64,i64) = sqlx::query_as("SELECT (SELECT COUNT(*) FROM market_price_ticks WHERE symbol=?),(SELECT COUNT(*) FROM market_pair_generation_runs WHERE pair_id=?)")
        .bind(&symbol).bind(pair).fetch_one(&pool).await?;
    assert_eq!(counts, (0, 0));
    let (status, preview) = call(
        &app,
        &token,
        "POST",
        &format!("{live}/preview"),
        json!({"expected_version":0,"initial_price":body["initial_price"],"config":body["config"]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["samples"].as_array().unwrap().len(), 60);
    // Pair deactivation after preview must block publication without consuming the draft.
    sqlx::query("UPDATE trading_pairs SET status='disabled' WHERE id=?")
        .bind(pair)
        .execute(&pool)
        .await?;
    let publish = json!({"pair_id":pair,"expected_revision":1,"reason":"publish draft"});
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
        StatusCode::BAD_REQUEST
    );
    sqlx::query("UPDATE trading_pairs SET status='active' WHERE id=?")
        .bind(pair)
        .execute(&pool)
        .await?;
    // Failure of the final audit must roll back both the generator and publication receipt.
    let trigger = format!("ncm_audit_{}", &suffix[16..24]);
    sqlx::raw_sql(&format!("CREATE TRIGGER {trigger} BEFORE INSERT ON admin_audit_logs FOR EACH ROW BEGIN IF NEW.admin_id={admin} AND NEW.action='new_coin.market_draft.publish' THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='test audit failure'; END IF; END"))
        .execute(&pool).await?;
    let (status, _) = call(
        &app,
        &token,
        "POST",
        &format!("{path}/publish"),
        publish.clone(),
    )
    .await;
    sqlx::raw_sql(&format!("DROP TRIGGER {trigger}"))
        .execute(&pool)
        .await?;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        call(&app, &token, "GET", &live, Value::Null).await.1["configured"],
        false
    );
    assert!(
        call(&app, &token, "GET", &path, Value::Null).await.1["draft"]["published_version"]
            .is_null()
    );

    let publish_path = format!("{path}/publish");
    let (left, right) = tokio::join!(
        call(&app, &token, "POST", &publish_path, publish.clone()),
        call(&app, &token, "POST", &publish_path, publish.clone())
    );
    assert_eq!(left.0, StatusCode::OK, "{left:?}");
    assert_eq!(right.0, StatusCode::OK, "{right:?}");
    assert_ne!(left.1["replayed"], right.1["replayed"]);
    assert_eq!(left.1["published_version"], 1);
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM market_default_generator_versions WHERE pair_id=?",
    )
    .bind(pair)
    .fetch_one(&pool)
    .await?;
    assert_eq!(count, 1);
    let (status, _) = call(
        &app,
        &token,
        "PATCH",
        &format!("{live}/pause-all"),
        json!({"expected_version":1,"all_market_paused":true,"reason":"pause"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        call(&app, &token, "POST", &format!("{path}/publish"), publish)
            .await
            .1["replayed"],
        true
    );
    let (_, paused) = call(&app, &token, "GET", &live, Value::Null).await;
    assert_eq!(paused["version"], 2);
    assert_eq!(paused["all_market_paused"], true);
    let mut next = body.clone();
    next["expected_revision"] = json!(1);
    next["expected_market_version"] = json!(2);
    assert_eq!(
        call(&app, &token, "PATCH", &path, next).await.0,
        StatusCode::OK
    );
    let (status, result) = call(
        &app,
        &token,
        "POST",
        &format!("{path}/publish"),
        json!({"pair_id":pair,"expected_revision":2,"reason":"publish while paused"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(
        call(&app, &token, "GET", &live, Value::Null).await.1["all_market_paused"],
        true
    );
    let unchanged: (String,bool,Option<u64>) = sqlx::query_as("SELECT lifecycle_status,post_listing_purchase_enabled,post_listing_pair_id FROM new_coin_projects WHERE id=?").bind(project).fetch_one(&pool).await?;
    assert_eq!(unchanged, ("preheat".into(), false, None));
    // Stale live version and wrong asset pairing cannot be hidden by a new draft revision.
    let mut invalid = body.clone();
    invalid["expected_revision"] = json!(2);
    assert_eq!(
        call(&app, &token, "PATCH", &path, invalid.clone()).await.0,
        StatusCode::CONFLICT
    );
    invalid["expected_market_version"] = json!(3);
    invalid["initial_price"] = json!("0.0000000000000000001");
    let (status, message) = call(&app, &token, "PATCH", &path, invalid.clone()).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{message}");
    invalid["initial_price"] = body["initial_price"].clone();
    let other_pair = sqlx::query("INSERT INTO trading_pairs (base_asset,quote_asset,symbol,price_precision,qty_precision,min_order_value,status,market_type) VALUES (?,?,?,18,18,1,'active','strategy')")
        .bind(quote).bind(base).bind(format!("X{symbol}")).execute(&pool).await?.last_insert_id();
    invalid["pair_id"] = json!(other_pair);
    assert_eq!(
        call(&app, &token, "PATCH", &path, invalid.clone()).await.0,
        StatusCode::BAD_REQUEST
    );
    invalid["pair_id"] = json!(pair);
    sqlx::query("UPDATE trading_pairs SET market_type='external' WHERE id=?")
        .bind(pair)
        .execute(&pool)
        .await?;
    assert_eq!(
        call(&app, &token, "PATCH", &path, invalid.clone()).await.0,
        StatusCode::BAD_REQUEST
    );
    sqlx::query("UPDATE trading_pairs SET market_type='strategy' WHERE id=?")
        .bind(pair)
        .execute(&pool)
        .await?;
    invalid["unexpected"] = json!(true);
    assert_eq!(
        call(&app, &token, "PATCH", &path, invalid).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let read = call(&app, &token, "GET", &path, Value::Null).await.1;
    assert_eq!(read["draft"]["revision"], 2);
    assert_eq!(read["pairs"].as_array().unwrap().len(), 1);
    Ok(())
}
