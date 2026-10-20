use super::*;

pub(super) async fn valuation(app: &axum::Router, token: &str, asset: u64) -> Value {
    let result = ok(request(app, token, "GET", "/api/v1/wallet/accounts", json!(null)).await);
    result["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["asset_id"] == asset)
        .unwrap()
        .clone()
}

pub(super) async fn verify(
    app: &axum::Router,
    pool: &MySqlPool,
    redis: &redis::aio::ConnectionManager,
    ids: (u64, u64, u64, u64),
    symbols: (&str, &str),
) -> (String, Value) {
    let (project, pair, asset, quote) = ids;
    let (symbol, pair_symbol) = symbols;
    let buyer = create_user(pool, "direct-buyer").await;
    let auth = utoken(buyer);
    sqlx::query("INSERT INTO wallet_accounts(user_id,asset_id,available) VALUES (?,?,100)")
        .bind(buyer)
        .bind(quote)
        .execute(pool)
        .await
        .unwrap();
    let path = format!("/api/v1/new-coins/{symbol}/market-purchases");
    for input in [
        json!({"quantity":1,"idempotency_key":"bad"}),
        json!({"quantity":"1","price":"0.1","idempotency_key":"bad"}),
    ] {
        assert_eq!(
            request(app, &auth, "POST", &path, input).await.0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    ticker(redis, pool, pair, pair_symbol, "0.5", 0).await;
    for (quantity, status) in [
        ("0", StatusCode::BAD_REQUEST),
        ("0.000000001", StatusCode::BAD_REQUEST),
        ("1000000", StatusCode::BAD_REQUEST),
        ("1000", StatusCode::BAD_REQUEST),
    ] {
        let result = request(
            app,
            &auth,
            "POST",
            &path,
            json!({"quantity":quantity,"idempotency_key":quantity}),
        )
        .await;
        assert_eq!(result.0, status, "{result:?}");
    }
    for age in [120, -30] {
        ticker(redis, pool, pair, pair_symbol, "0.5", age).await;
        assert_eq!(
            request(
                app,
                &auth,
                "POST",
                &path,
                json!({"quantity":"1","idempotency_key":"bad-price"})
            )
            .await
            .0,
            StatusCode::CONFLICT
        );
    }
    ticker(redis, pool, pair, pair_symbol, "0.5", 0).await;
    sqlx::query("UPDATE market_default_generators SET all_market_paused=1 WHERE pair_id=?")
        .bind(pair)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            app,
            &auth,
            "POST",
            &path,
            json!({"quantity":"1","idempotency_key":"paused"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE market_default_generators SET all_market_paused=0 WHERE pair_id=?")
        .bind(pair)
        .execute(pool)
        .await
        .unwrap();
    let intent = json!({"quantity":"2.00","idempotency_key":"direct-live"});
    sqlx::raw_sql(&format!("CREATE TRIGGER direct_buy_fail BEFORE INSERT ON new_coin_operations FOR EACH ROW BEGIN IF NEW.project_id={project} AND NEW.action='new_coin.trade' THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='test direct buy rollback'; END IF; END")).execute(pool).await.unwrap();
    assert_eq!(
        request(app, &auth, "POST", &path, intent.clone()).await.0,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    sqlx::raw_sql("DROP TRIGGER direct_buy_fail")
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(wallet(pool, buyer, quote).await.0, BigDecimal::from(100));
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM new_coin_trade_quotes WHERE user_id=?")
            .bind(buyer)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    let (a, b) = tokio::join!(
        request(app, &auth, "POST", &path, intent.clone()),
        request(app, &auth, "POST", &path, intent.clone())
    );
    let receipt = ok(a);
    assert_eq!(receipt, ok(b));
    assert_eq!(
        decimal(&receipt["receipt"]["price"]),
        BigDecimal::from_str("0.5").unwrap()
    );
    assert_eq!(wallet(pool, buyer, quote).await.0, BigDecimal::from(99));
    assert_eq!(wallet(pool, buyer, asset).await.2, BigDecimal::from(2));
    ticker(redis, pool, pair, pair_symbol, "0.8", 0).await;
    assert_eq!(
        ok(request(
            app,
            &auth,
            "POST",
            &path,
            json!({"quantity":"2","idempotency_key":"direct-live"})
        )
        .await),
        receipt
    );
    assert_eq!(
        request(
            app,
            &auth,
            "POST",
            &path,
            json!({"quantity":"3","idempotency_key":"direct-live"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            app,
            &auth,
            "POST",
            &format!("/api/v1/new-coins/{symbol}/quotes"),
            json!({"side":"buy","quantity":"2","idempotency_key":"direct-live"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM new_coin_holdings WHERE user_id=? AND project_id=?",
    )
    .bind(buyer)
    .bind(project)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(count, 1);
    let unbalanced:i64=sqlx::query_scalar("SELECT COUNT(*) FROM (SELECT transaction_key,asset_id FROM platform_financial_journal WHERE context='new_coin_issuance' GROUP BY transaction_key,asset_id HAVING SUM(amount)<>0) x").fetch_one(pool).await.unwrap();
    assert_eq!(unbalanced, 0);
    (auth, receipt)
}
