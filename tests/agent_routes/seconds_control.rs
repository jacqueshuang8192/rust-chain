use super::*;
use axum::http::Method;

/// 以代理身份发送 JSON 写请求，复用全局测试连接池与令牌签发方式。
async fn agent_write_json(
    app: axum::Router,
    token: &str,
    method: Method,
    uri: impl AsRef<str>,
    body: Value,
) -> Result<(StatusCode, Value), Box<dyn Error>> {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri.as_ref())
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body)?))?,
        )
        .await?;
    let status = response.status();
    let payload = response_json(response).await?;
    Ok((status, payload))
}

async fn create_seconds_pair(pool: &MySqlPool) -> (u64, u64) {
    let base = create_asset(pool, "scbase").await;
    let quote = create_asset(pool, "scquote").await;
    let symbol = format!("SC{}-USD", Uuid::now_v7().simple());
    let pair = sqlx::query(
        "INSERT INTO trading_pairs (base_asset, quote_asset, symbol, price_precision, qty_precision, min_order_value, status, market_type) VALUES (?, ?, ?, 18, 18, 1, 'active', 'external')",
    )
    .bind(base)
    .bind(quote)
    .bind(&symbol)
    .execute(pool)
    .await
    .unwrap()
    .last_insert_id();
    (pair, quote)
}

async fn insert_seconds_order(
    pool: &MySqlPool,
    user: u64,
    pair: u64,
    asset: u64,
    product: u64,
    status: &str,
) -> u64 {
    sqlx::query(
        r#"INSERT INTO seconds_contract_orders
           (user_id, product_id, pair_id, stake_asset, direction, stake_amount, duration_seconds,
            payout_rate, entry_price, status, result, settlement_price, idempotency_key,
            expires_at, settled_at, created_at)
           VALUES (?, ?, ?, ?, 'up', 10, 60, 0.8, 100, ?, ?, ?, ?,
                   '2035-01-01 00:01:00', '2035-01-01 00:01:00', '2025-01-01 00:00:00')"#,
    )
    .bind(user)
    .bind(product)
    .bind(pair)
    .bind(asset)
    .bind(status)
    .bind((status == "settled").then_some("win"))
    .bind((status == "settled").then_some(101))
    .bind(Uuid::now_v7().to_string())
    .execute(pool)
    .await
    .unwrap()
    .last_insert_id()
}

#[tokio::test]
async fn agent_seconds_control_requires_subtree_and_writes_audit_trail()
-> Result<(), Box<dyn Error>> {
    let Some(pool) = mysql_pool().await else {
        return Ok(());
    };
    let settings = test_settings();
    let root = create_agent(&pool, "sc-root").await;
    let child = create_child_agent(&pool, root, "sc-child").await;
    let other = create_agent(&pool, "sc-other").await;
    let user = create_user(&pool, "sc-user").await;
    refer_user_to_agent(&pool, user, child.agent_id, 1).await;
    let outsider = create_user(&pool, "sc-outsider").await;
    refer_user_to_agent(&pool, outsider, other.agent_id, 1).await;

    let (pair, asset) = create_seconds_pair(&pool).await;
    let product = sqlx::query(
        "INSERT INTO seconds_contract_products (pair_id, stake_asset, duration_seconds, payout_rate, min_stake, max_stake, status) VALUES (?, ?, 60, 0.8, 1, 1000, 'active')",
    )
    .bind(pair)
    .bind(asset)
    .execute(&pool)
    .await?
    .last_insert_id();
    let opened = insert_seconds_order(&pool, user, pair, asset, product, "opened").await;
    let settled = insert_seconds_order(&pool, user, pair, asset, product, "settled").await;
    let foreign = insert_seconds_order(&pool, outsider, pair, asset, product, "opened").await;

    let token = issue_token(
        &settings,
        format!("agent:{}", child.admin_user_id),
        TokenScope::Agent,
        900,
    )
    .unwrap();
    let app = build_router(AppState::new(settings).with_mysql(pool.clone()));

    // 设置订单级控赢后，订单列与控制流水同时落库。
    let (status, payload) = agent_write_json(
        app.clone(),
        &token,
        Method::POST,
        format!("/agent/api/v1/seconds-contract-orders/{opened}/control"),
        json!({ "result": "win", "reason": "代理控赢" }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{payload}");
    assert_eq!(payload["control_result"], "win");
    assert_eq!(payload["scope"], "order");
    assert_eq!(payload["user_id"].as_u64(), Some(user));
    let control: (Option<String>, Option<String>, Option<u64>) = sqlx::query_as(
        "SELECT control_result, control_actor_type, control_actor_id FROM seconds_contract_orders WHERE id = ?",
    )
    .bind(opened)
    .fetch_one(&pool)
    .await?;
    assert_eq!(control.0.as_deref(), Some("win"));
    assert_eq!(control.1.as_deref(), Some("agent"));
    assert_eq!(control.2, Some(child.admin_user_id));
    let events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM seconds_contract_result_control_events WHERE order_id = ? AND scope = 'order'",
    )
    .bind(opened)
    .fetch_one(&pool)
    .await?;
    assert_eq!(events, 1);

    // 团队订单列表回显控制状态。
    let (status, payload) = agent_get_json(
        app.clone(),
        &token,
        "/agent/api/v1/orders/seconds?limit=100",
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{payload}");
    let row = payload["orders"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"].as_u64() == Some(opened))
        .unwrap();
    assert_eq!(row["control_result"], "win");
    assert_eq!(row["control_actor_type"], "agent");

    // 取消控制后订单列恢复为空，并追加一条取消流水。
    let (status, payload) = agent_write_json(
        app.clone(),
        &token,
        Method::POST,
        format!("/agent/api/v1/seconds-contract-orders/{opened}/control"),
        json!({ "result": "clear", "reason": "恢复自然结算" }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{payload}");
    assert!(payload["control_result"].is_null());
    let cleared: Option<String> =
        sqlx::query_scalar("SELECT control_result FROM seconds_contract_orders WHERE id = ?")
            .bind(opened)
            .fetch_one(&pool)
            .await?;
    assert_eq!(cleared, None);
    let events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM seconds_contract_result_control_events WHERE order_id = ?",
    )
    .bind(opened)
    .fetch_one(&pool)
    .await?;
    assert_eq!(events, 2);

    // 已结算订单、子树外订单与非法结果分别返回冲突、未找到与参数错误。
    let (status, _) = agent_write_json(
        app.clone(),
        &token,
        Method::POST,
        format!("/agent/api/v1/seconds-contract-orders/{settled}/control"),
        json!({ "result": "loss", "reason": "尝试覆盖" }),
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = agent_write_json(
        app.clone(),
        &token,
        Method::POST,
        format!("/agent/api/v1/seconds-contract-orders/{foreign}/control"),
        json!({ "result": "loss", "reason": "跨团队" }),
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = agent_write_json(
        app.clone(),
        &token,
        Method::POST,
        format!("/agent/api/v1/seconds-contract-orders/{opened}/control"),
        json!({ "result": "draw", "reason": "非法结果" }),
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = agent_write_json(
        app.clone(),
        &token,
        Method::POST,
        format!("/agent/api/v1/seconds-contract-orders/{opened}/control"),
        json!({ "result": "win", "reason": "   " }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let blank_reason: Option<String> =
        sqlx::query_scalar("SELECT control_reason FROM seconds_contract_orders WHERE id = ?")
            .bind(opened)
            .fetch_one(&pool)
            .await?;
    assert_eq!(blank_reason, None);
    let (status, payload) = agent_write_json(
        app.clone(),
        &token,
        Method::POST,
        format!("/agent/api/v1/seconds-contract-orders/{opened}/control"),
        json!({ "result": "loss" }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{payload}");
    assert!(payload["reason"].is_null());

    // 用户级控制：设置、读取、清除，子树外用户不可见。
    let (status, payload) = agent_write_json(
        app.clone(),
        &token,
        Method::PUT,
        format!("/agent/api/v1/users/{user}/seconds-contract-control"),
        json!({ "result": "loss", "reason": "默认控输" }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{payload}");
    assert_eq!(payload["control_result"], "loss");
    assert_eq!(payload["scope"], "user");
    let (status, payload) = agent_get_json(
        app.clone(),
        &token,
        format!("/agent/api/v1/users/{user}/seconds-contract-control"),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{payload}");
    assert_eq!(payload["active"], true);
    assert_eq!(payload["control_result"], "loss");
    let (status, payload) = agent_write_json(
        app.clone(),
        &token,
        Method::PUT,
        format!("/agent/api/v1/users/{user}/seconds-contract-control"),
        json!({ "result": "clear", "reason": "取消默认控制" }),
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{payload}");
    assert!(payload["control_result"].is_null());
    let (status, payload) = agent_get_json(
        app.clone(),
        &token,
        format!("/agent/api/v1/users/{user}/seconds-contract-control"),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(payload["active"], false);
    let (status, _) = agent_get_json(
        app.clone(),
        &token,
        format!("/agent/api/v1/users/{outsider}/seconds-contract-control"),
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    Ok(())
}
