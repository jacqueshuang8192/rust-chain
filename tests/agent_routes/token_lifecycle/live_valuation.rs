use super::*;
use exchange_api::modules::market::{
    MarketDataProvider, MarketTickerSnapshot, RedisMarketCache,
    adapters::{
        DefaultTickerProvenance, MarketIngestionService, PairTickerFence, SyntheticTickerProvenance,
    },
    infrastructure::default_runtime::PairGenerationLock,
};

async fn views(app: &axum::Router, auth: &str, project: u64, asset: u64) -> (Value, Value) {
    let holdings = ok(request(
        app,
        auth,
        "GET",
        &format!("/api/v1/new-coins/holdings?project_id={project}"),
        json!(null),
    )
    .await);
    (
        holdings["holdings"][0].clone(),
        direct_purchase::valuation(app, auth, asset).await,
    )
}

fn valued((holding, account): &(Value, Value), price: &str) {
    let price = BigDecimal::from_str(price).unwrap();
    assert_eq!(decimal(&holding["market_price"]), price);
    assert_eq!(decimal(&holding["market_value"]), &price * 2500);
    assert_eq!(decimal(&holding["profit"]), &price * 2500 - 500);
    assert_eq!(
        holding["eligible"], false,
        "locked holdings must still have valuation"
    );
    assert_eq!(decimal(&account["locked"]), BigDecimal::from(2500));
    assert_eq!(
        decimal(&account["issuance_valuation"]["price_usdt"]),
        price * 2
    );
}

async fn unavailable(app: &axum::Router, auth: &str, project: u64, asset: u64) {
    let (holding, account) = views(app, auth, project, asset).await;
    for field in ["market_price", "market_value", "profit"] {
        assert!(holding[field].is_null(), "{field}: {holding}");
    }
    assert!(
        account["issuance_valuation"]["price_usdt"].is_null(),
        "{account}"
    );
    assert_eq!(decimal(&account["locked"]), BigDecimal::from(2500));
}

pub(super) async fn verify(
    app: &axum::Router,
    pool: &MySqlPool,
    redis: &redis::aio::ConnectionManager,
    auth: &str,
    (project, pair, asset, holding_id): (u64, u64, u64, u64),
    symbol: &str,
) {
    // Exercise the real publisher without Mongo I/O: ticker ingestion only uses MySQL/Redis.
    let mongo = mongodb::Client::with_uri_str("mongodb://127.0.0.1:1/?serverSelectionTimeoutMS=50")
        .await
        .unwrap()
        .database("unused_token_valuation");
    let ingestion = MarketIngestionService::new(RedisMarketCache::new(redis.clone()), mongo)
        .with_mysql(Some(pool.clone()));
    let owner = "valuation-producer";
    let version: u32 =
        sqlx::query_scalar("SELECT version FROM market_default_generators WHERE pair_id=?")
            .bind(pair)
            .fetch_one(pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO market_pair_generation_runs(pair_id,generation,active_source,default_version,lease_owner,lease_expires_at) VALUES (?,1,'default',?,?,DATE_ADD(UTC_TIMESTAMP(6),INTERVAL 5 MINUTE)) ON DUPLICATE KEY UPDATE active_source='default',default_version=VALUES(default_version),lease_owner=VALUES(lease_owner),lease_expires_at=VALUES(lease_expires_at)")
        .bind(pair).bind(version).bind(owner).execute(pool).await.unwrap();
    let generation: u64 =
        sqlx::query_scalar("SELECT generation FROM market_pair_generation_runs WHERE pair_id=?")
            .bind(pair)
            .fetch_one(pool)
            .await
            .unwrap();
    let original_policy: sqlx::types::Json<Value> =
        sqlx::query_scalar("SELECT policy_json FROM new_coin_holdings WHERE id=?")
            .bind(holding_id)
            .fetch_one(pool)
            .await
            .unwrap();
    sqlx::query("UPDATE new_coin_holdings SET policy_json=JSON_SET(policy_json,'$.lock',JSON_OBJECT('kind','on_listing')) WHERE id=?")
        .bind(holding_id).execute(pool).await.unwrap();
    let lock = PairGenerationLock::acquire(pool, pair, 0)
        .await
        .unwrap()
        .unwrap();
    let fence = PairTickerFence {
        pair_id: pair,
        generation,
        lock_name: lock.name().into(),
        connection_id: lock.connection_id(),
        owner: owner.into(),
    };
    let observed_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar("SELECT UTC_TIMESTAMP(6)")
        .fetch_one(pool)
        .await
        .unwrap();
    let snapshot = MarketTickerSnapshot::new(
        MarketDataProvider::Strategy,
        symbol,
        BigDecimal::from_str("0.4").unwrap(),
        1.into(),
        observed_at,
    )
    .unwrap();
    assert!(
        ingestion
            .ingest_and_publish_default_ticker(
                &snapshot,
                &DefaultTickerProvenance {
                    fence,
                    config_version: version
                }
            )
            .await
            .unwrap()
            .is_accepted()
    );
    lock.release().await.unwrap();
    let key = exchange_api::modules::market::market_ticker_redis_key(symbol);
    let raw: String = redis.clone().get(&key).await.unwrap();
    let wire: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(wire["source"], "generated");
    assert_eq!(wire["provider"], "strategy");
    let default_views = views(app, auth, project, asset).await;
    valued(&default_views, "0.4");
    let token_symbol = default_views.0["symbol"].as_str().unwrap();
    let quote_path = format!("/api/v1/new-coins/{token_symbol}/quotes");
    let quote = ok(request(
        app,
        auth,
        "POST",
        &quote_path,
        json!({"side":"buy","quantity":"1","idempotency_key":"real-default-source"}),
    )
    .await);
    assert_eq!(quote["source"], "default");
    assert_spot_discovery(app, auth, pair, false).await;
    if let Ok(path) = std::env::var("TOKEN_VALUATION_EVIDENCE_PATH") {
        let wallets = ok(request(app, auth, "GET", "/api/v1/wallet/accounts", json!(null)).await);
        std::fs::write(
            path,
            serde_json::to_vec_pretty(
                &json!({"holding":default_views.0,"wallets":wallets,"ticker":wire}),
            )
            .unwrap(),
        )
        .unwrap();
    }
    // Public metadata cannot authorize a different cached price, timestamp, or engine generation.
    for (field, value) in [
        ("last_price", json!("0.41")),
        ("observed_at", json!(observed_at.timestamp_millis() - 1)),
        ("provider", json!("bitget")),
    ] {
        let mut bad = wire.clone();
        bad[field] = value;
        redis
            .clone()
            .set::<_, _, ()>(&key, bad.to_string())
            .await
            .unwrap();
        unavailable(app, auth, project, asset).await;
    }
    redis.clone().set::<_, _, ()>(&key, &raw).await.unwrap();
    for (set, restore) in [
        ("generation=generation+1", "generation=generation-1"),
        (
            "default_version=default_version+1",
            "default_version=default_version-1",
        ),
        (
            "lease_expires_at=DATE_SUB(UTC_TIMESTAMP(6),INTERVAL 1 SECOND)",
            "lease_expires_at=DATE_ADD(UTC_TIMESTAMP(6),INTERVAL 5 MINUTE)",
        ),
    ] {
        sqlx::query(&format!(
            "UPDATE market_pair_generation_runs SET {set} WHERE pair_id=?"
        ))
        .bind(pair)
        .execute(pool)
        .await
        .unwrap();
        unavailable(app, auth, project, asset).await;
        sqlx::query(&format!(
            "UPDATE market_pair_generation_runs SET {restore} WHERE pair_id=?"
        ))
        .bind(pair)
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE market_default_generators SET all_market_paused=1 WHERE pair_id=?")
        .bind(pair)
        .execute(pool)
        .await
        .unwrap();
    unavailable(app, auth, project, asset).await;
    sqlx::query("UPDATE market_default_generators SET all_market_paused=0 WHERE pair_id=?")
        .bind(pair)
        .execute(pool)
        .await
        .unwrap();
    redis.clone().del::<_, ()>(&key).await.unwrap();
    unavailable(app, auth, project, asset).await; // Never fall back to checkpoint price.
    redis.clone().set::<_, _, ()>(&key, &raw).await.unwrap();
    sqlx::query(
        "UPDATE market_generation_price_checkpoints SET symbol=CONCAT(symbol,'X') WHERE symbol=?",
    )
    .bind(exchange_api::modules::market::sanitize_symbol(symbol))
    .execute(pool)
    .await
    .unwrap();
    unavailable(app, auth, project, asset).await;
    sqlx::query("UPDATE market_generation_price_checkpoints SET symbol=? WHERE symbol=?")
        .bind(exchange_api::modules::market::sanitize_symbol(symbol))
        .bind(format!(
            "{}X",
            exchange_api::modules::market::sanitize_symbol(symbol)
        ))
        .execute(pool)
        .await
        .unwrap();
    valued(&views(app, auth, project, asset).await, "0.4");

    // The same generic Redis provenance is also used by a real scheduled strategy.
    let strategy = sqlx::query("INSERT INTO market_strategies(pair_id,strategy_type,start_price,target_price,start_time,end_time,volatility,volume_min,volume_max,status) VALUES(?,'price_path',0.4,0.5,DATE_SUB(UTC_TIMESTAMP(6),INTERVAL 1 MINUTE),DATE_ADD(UTC_TIMESTAMP(6),INTERVAL 5 MINUTE),0.01,1,2,'active')")
        .bind(pair).execute(pool).await.unwrap().last_insert_id();
    for version in [1, 2] {
        sqlx::query("INSERT INTO strategy_versions(strategy_id,version,effective_time,config_json,seed) VALUES(?,?,UTC_TIMESTAMP(6),JSON_OBJECT(),'valuation')")
            .bind(strategy).bind(version).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO strategy_runs(strategy_id,active_version,run_status,lease_owner,lease_expires_at) VALUES(?,1,'running',?,DATE_ADD(UTC_TIMESTAMP(6),INTERVAL 5 MINUTE))")
        .bind(strategy).bind(owner).execute(pool).await.unwrap();
    sqlx::query("UPDATE market_pair_generation_runs SET generation=generation+1,active_source='strategy',default_version=NULL,strategy_id=?,strategy_version=1 WHERE pair_id=?")
        .bind(strategy).bind(pair).execute(pool).await.unwrap();
    unavailable(app, auth, project, asset).await; // Old default price cannot borrow the new live strategy.
    let lock = PairGenerationLock::acquire(pool, pair, 0)
        .await
        .unwrap()
        .unwrap();
    let fence = PairTickerFence {
        pair_id: pair,
        generation: generation + 1,
        lock_name: lock.name().into(),
        connection_id: lock.connection_id(),
        owner: owner.into(),
    };
    let observed_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar("SELECT UTC_TIMESTAMP(6)")
        .fetch_one(pool)
        .await
        .unwrap();
    let snapshot = MarketTickerSnapshot::new(
        MarketDataProvider::Strategy,
        symbol,
        BigDecimal::from_str("0.5").unwrap(),
        1.into(),
        observed_at,
    )
    .unwrap();
    let provenance = SyntheticTickerProvenance::new(strategy, 1, owner).with_pair_fence(fence);
    assert!(
        ingestion
            .ingest_and_publish_synthetic_ticker(&snapshot, &provenance)
            .await
            .unwrap()
            .is_accepted()
    );
    lock.release().await.unwrap();
    valued(&views(app, auth, project, asset).await, "0.5");
    let quote = ok(request(
        app,
        auth,
        "POST",
        &quote_path,
        json!({"side":"buy","quantity":"1","idempotency_key":"real-strategy-source"}),
    )
    .await);
    assert_eq!(quote["source"], "strategy");
    sqlx::query("UPDATE strategy_runs SET active_version=2 WHERE strategy_id=?")
        .bind(strategy)
        .execute(pool)
        .await
        .unwrap();
    unavailable(app, auth, project, asset).await;
    sqlx::query("UPDATE market_strategies SET status='ended' WHERE id=?")
        .bind(strategy)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE market_pair_generation_runs SET generation=generation+1,active_source='default',default_version=?,strategy_id=NULL,strategy_version=NULL WHERE pair_id=?")
        .bind(version).bind(pair).execute(pool).await.unwrap();
    unavailable(app, auth, project, asset).await; // Old strategy price cannot borrow default readiness.
    for age in [120, -30] {
        ticker(redis, pool, pair, symbol, "0.4", age).await;
        unavailable(app, auth, project, asset).await;
    }
    sqlx::query("UPDATE new_coin_holdings SET policy_json=? WHERE id=?")
        .bind(original_policy)
        .bind(holding_id)
        .execute(pool)
        .await
        .unwrap();
}
