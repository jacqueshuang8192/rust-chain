use super::*;

async fn call(
    app: &Router,
    token: Option<&str>,
    method: &str,
    path: &str,
    body: Value,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn timed_deposit_routes_enforce_exact_amount_ownership_and_admin_permissions() {
    let Some(pool) = mysql_pool().await else {
        return;
    };
    let settings = test_settings();
    let user = create_user(&pool).await;
    let other = create_user(&pool).await;
    let (role, admin) = create_admin(&pool).await;
    let (asset, _) = create_asset(&pool).await;
    let symbol: String = sqlx::query_scalar("SELECT symbol FROM assets WHERE id=?")
        .bind(asset)
        .fetch_one(&pool)
        .await
        .unwrap();
    let contract = format!("0x{:040x}", asset + 1_000_000);
    let address = format!("0x{:040x}", asset + 2_000_000);
    let state = AppState::new(settings.clone()).with_mysql(pool.clone());
    let app = Router::new()
        .nest("/api/v1", routes())
        .nest("/admin/api/v1", admin_routes())
        .with_state(state);
    let user_token = issue_token(&settings, format!("user:{user}"), TokenScope::User, 900).unwrap();
    let other_token =
        issue_token(&settings, format!("user:{other}"), TokenScope::User, 900).unwrap();
    let admin_token =
        issue_token(&settings, format!("admin:{admin}"), TokenScope::Admin, 900).unwrap();
    assert_eq!(
        call(
            &app,
            None,
            "GET",
            "/api/v1/wallet/deposit-options",
            Value::Null
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &app,
            Some(&admin_token),
            "GET",
            "/api/v1/wallet/deposit-options",
            Value::Null
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let asset_response=call(&app,Some(&admin_token),"PUT","/admin/api/v1/wallet/deposit-settings/assets",json!({"network":"eth","asset_symbol":symbol,"asset_kind":"token","token_address":contract,"decimals":18,"enabled":true,"expected_revision":0,"reason":"local route test"})).await;
    assert_eq!(
        asset_response.status(),
        StatusCode::NO_CONTENT,
        "{:?}",
        body_json(asset_response).await
    );
    assert_eq!(
        call(
            &app,
            Some(&admin_token),
            "POST",
            "/admin/api/v1/wallet/deposit-settings/addresses",
            json!({"network":"eth","addresses":[address],"reason":"local direct import"})
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    // The route fixture simulates scanner readiness; it never contacts a chain node.
    sqlx::query("UPDATE wallet_deposit_scanners SET enabled=TRUE,endpoint_encrypted='route-fixture',last_scan_at=CURRENT_TIMESTAMP(6),last_error=NULL WHERE network='eth'").execute(&pool).await.unwrap();
    sqlx::query("UPDATE deposit_network_configs SET asset_symbols_json=NULL WHERE network='eth'")
        .execute(&pool)
        .await
        .unwrap();
    let path = "/api/v1/wallet/deposit-orders";
    let body = json!({"asset_symbol":symbol,"network":"eth","amount":"9007199254740993.000000000000000001","idempotency_key":"route-test-exact"});
    let mut numeric = body.clone();
    numeric["amount"] = json!(100);
    assert_eq!(
        call(&app, Some(&user_token), "POST", path, numeric)
            .await
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let response = call(&app, Some(&user_token), "POST", path, body.clone()).await;
    let status = response.status();
    let order = body_json(response).await.unwrap();
    assert_eq!(status, StatusCode::OK, "{order}");
    assert_eq!(order["amount"], "9007199254740993.000000000000000001");
    assert_eq!(order["user_id"], user);
    let repeated = body_json(call(&app, Some(&user_token), "POST", path, body).await)
        .await
        .unwrap();
    assert_eq!(repeated["id"], order["id"]);
    let detail = format!("{path}/{}", order["id"]);
    assert_eq!(
        call(&app, Some(&other_token), "GET", &detail, Value::Null)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    let filtered = format!("{path}?order_no={}", order["order_no"].as_str().unwrap());
    let hidden = body_json(call(&app, Some(&other_token), "GET", &filtered, Value::Null).await)
        .await
        .unwrap();
    assert_eq!(hidden["total"], 0);
    let visible = body_json(call(&app, Some(&user_token), "GET", &filtered, Value::Null).await)
        .await
        .unwrap();
    assert_eq!(visible["total"], 1);
    let cancel_path = format!("{detail}/cancel");
    for (token, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(admin_token.as_str()), StatusCode::FORBIDDEN),
        (Some(other_token.as_str()), StatusCode::NOT_FOUND),
    ] {
        assert_eq!(
            call(&app, token, "POST", &cancel_path, Value::Null)
                .await
                .status(),
            expected
        );
    }
    for _ in 0..2 {
        let response = call(&app, Some(&user_token), "POST", &cancel_path, Value::Null).await;
        assert_eq!(response.status(), StatusCode::OK);
        let cancelled = body_json(response).await.unwrap();
        assert_eq!(cancelled["status"], "cancelled");
        assert!(cancelled["cancelled_at"].is_i64());
        assert_eq!(cancelled["expires_at"], order["expires_at"]);
    }
    // Network-only operators receive the minimal asset catalog without assets.read.
    sqlx::query("UPDATE admin_roles SET permissions=JSON_ARRAY('wallet.networks.read') WHERE id=?")
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    let response = call(
        &app,
        Some(&admin_token),
        "GET",
        "/admin/api/v1/wallet/deposit-settings",
        Value::Null,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let config = body_json(response).await.unwrap();
    let catalog = config["platform_assets"].as_array().unwrap();
    let entry = catalog.iter().find(|row| row["symbol"] == symbol).unwrap();
    assert!(entry["deposit_enabled"].is_boolean());
    assert!(entry["precision_scale"].is_i64());
    assert!(entry["status"].is_string());
    assert_eq!(entry.as_object().unwrap().len(), 4);
    sqlx::query("UPDATE admin_roles SET permissions=JSON_ARRAY('wallet.deposits.read') WHERE id=?")
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        call(
            &app,
            Some(&admin_token),
            "GET",
            "/admin/api/v1/wallet/deposit-orders",
            Value::Null
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Some(&admin_token),
            "POST",
            "/admin/api/v1/wallet/deposit-receipts/1/review",
            json!({"action":"ignore","order_id":null,"reason":"should not be authorized"})
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            Some(&admin_token),
            "GET",
            "/admin/api/v1/wallet/deposit-settings",
            Value::Null
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE admin_roles SET permissions=JSON_ARRAY('wallet.address_pool.read','wallet.address_pool.write') WHERE id=?").bind(role).execute(&pool).await.unwrap();
    assert_eq!(
        call(
            &app,
            Some(&admin_token),
            "GET",
            "/admin/api/v1/wallet/deposit-settings/addresses",
            Value::Null
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            Some(&admin_token),
            "GET",
            "/admin/api/v1/wallet/deposit-settings",
            Value::Null
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(
            &app,
            Some(&admin_token),
            "POST",
            "/admin/api/v1/wallet/deposit-settings/addresses",
            json!({"network":"eth","addresses":[address],"reason":"address role"})
        )
        .await
        .status(),
        StatusCode::NO_CONTENT
    );
    sqlx::query(
        "UPDATE admin_roles SET permissions=JSON_ARRAY('wallet.address_pool.read') WHERE id=?",
    )
    .bind(role)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        call(
            &app,
            Some(&admin_token),
            "POST",
            "/admin/api/v1/wallet/deposit-settings/addresses",
            json!({"network":"eth","addresses":[address],"reason":"read role cannot write"})
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    for (path, token) in [
        ("/api/v1/wallet/deposit-address", &user_token),
        ("/admin/api/v1/wallet/deposits/observe", &admin_token),
        ("/admin/api/v1/deposit-address-pool", &admin_token),
        ("/admin/api/v1/deposit-address-pool/batch", &admin_token),
        ("/admin/api/v1/deposit-address-pool/1/reclaim", &admin_token),
    ] {
        assert_eq!(
            call(&app, Some(token), "POST", path, json!({}))
                .await
                .status(),
            StatusCode::NOT_FOUND,
            "retired path {path}"
        );
    }
    // Recovery is a network-write operation; rejected requests cannot queue or change checkpoints.
    let recovery_path = "/admin/api/v1/wallet/deposit-settings/scanners/eth/recovery";
    sqlx::query("UPDATE wallet_deposit_scanners SET cursor_json=JSON_OBJECT('height',80,'hash_height',80,'hash','b80') WHERE network='eth'").execute(&pool).await.unwrap();
    let revision: u64 =
        sqlx::query_scalar("SELECT revision FROM wallet_deposit_scanners WHERE network='eth'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let recovery = json!({"mode":"skip","expected_revision":revision,"acknowledge_skip":true,"reason":"audited fixture skip"});
    for (token, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some(user_token.as_str()), StatusCode::FORBIDDEN),
        (Some(admin_token.as_str()), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            call(&app, token, "POST", recovery_path, recovery.clone())
                .await
                .status(),
            expected
        );
    }
    sqlx::query(
        "UPDATE admin_roles SET permissions=JSON_ARRAY('wallet.networks.write') WHERE id=?",
    )
    .bind(role)
    .execute(&pool)
    .await
    .unwrap();
    for (field, value, status) in [
        ("mode", json!("automatic"), StatusCode::BAD_REQUEST),
        ("acknowledge_skip", json!(false), StatusCode::BAD_REQUEST),
        ("reason", json!(""), StatusCode::BAD_REQUEST),
        (
            "expected_revision",
            json!(revision + 10),
            StatusCode::CONFLICT,
        ),
        (
            "expected_revision",
            json!("invalid"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let mut invalid = recovery.clone();
        invalid[field] = value;
        assert_eq!(
            call(&app, Some(&admin_token), "POST", recovery_path, invalid)
                .await
                .status(),
            status
        );
    }
    let pending: bool = sqlx::query_scalar(
        "SELECT recovery_request_json IS NOT NULL FROM wallet_deposit_scanners WHERE network='eth'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!pending);
    let response = call(
        &app,
        Some(&admin_token),
        "POST",
        recovery_path,
        recovery.clone(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(body_json(response).await.unwrap(), json!({"queued":true}));
    assert_eq!(
        call(&app, Some(&admin_token), "POST", recovery_path, recovery)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let height: u64 = sqlx::query_scalar("SELECT CAST(JSON_UNQUOTE(JSON_EXTRACT(cursor_json,'$.height')) AS UNSIGNED) FROM wallet_deposit_scanners WHERE network='eth'").fetch_one(&pool).await.unwrap();
    assert_eq!(
        height, 80,
        "only the validated worker may apply the recovery"
    );
}
