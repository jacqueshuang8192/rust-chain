use super::*;
use crate::modules::market::{
    MarketDataProvider, MarketTickerCacheEntry, MarketTickerSnapshot, RedisMarketCache,
};

#[tokio::test]
async fn live_follow_reads_latest_redis_without_sql_price_history()
-> Result<(), Box<dyn std::error::Error>> {
    let (Ok(db), Ok(redis_url)) = (
        std::env::var("LATEST_PRICE_TEST_DATABASE_URL"),
        std::env::var("LATEST_PRICE_TEST_REDIS_URL"),
    ) else {
        return Ok(());
    };
    for value in [&db, &redis_url] {
        assert_eq!(url::Url::parse(value)?.host_str(), Some("127.0.0.1"));
    }
    assert!(url::Url::parse(&db)?.path().ends_with("_test"));
    let pool = sqlx::mysql::MySqlPoolOptions::new().connect(&db).await?;
    sqlx::migrate!().run(&pool).await?;
    let suffix = uuid::Uuid::new_v4().simple().to_string()[..8].to_uppercase();
    let mut ids = Vec::new();
    for prefix in ["LB", "LQ"] {
        ids.push(sqlx::query("INSERT INTO assets(symbol,name,precision_scale,asset_type,status) VALUES(?,?,18,'coin','active')")
            .bind(format!("{prefix}{suffix}")).bind("latest fixture").execute(&pool).await?.last_insert_id());
    }
    let symbol = format!("LB{suffix}LQ{suffix}");
    let reference = sqlx::query("INSERT INTO trading_pairs(base_asset,quote_asset,symbol,price_precision,qty_precision,min_order_value,status,market_type) VALUES(?,?,?,18,18,1,'active','external')")
        .bind(ids[0]).bind(ids[1]).bind(&symbol).execute(&pool).await?.last_insert_id();
    let redis = redis::Client::open(redis_url)?
        .get_connection_manager()
        .await?;
    let cache = RedisMarketCache::new(redis);
    let now = Utc::now();
    for (offset, price) in [(2, 100), (1, 200)] {
        let snapshot = MarketTickerSnapshot::new(
            MarketDataProvider::Bitget,
            &symbol,
            price.into(),
            1.into(),
            now - chrono::TimeDelta::seconds(offset),
        )?;
        cache
            .save_ticker(MarketTickerCacheEntry::from_snapshot(&snapshot)?)
            .await?;
        let selected =
            load_follow_reference(&pool, &cache, reference + 1, reference, now, 15, None).await?;
        assert_eq!(
            selected.observation.unwrap().price,
            bigdecimal::BigDecimal::from(price)
        );
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM market_price_ticks WHERE symbol=?")
        .bind(&symbol)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0);
    assert!(
        load_follow_reference(
            &pool,
            &cache,
            reference + 1,
            reference,
            now + chrono::TimeDelta::seconds(16),
            15,
            None
        )
        .await?
        .observation
        .is_none()
    );
    sqlx::query("UPDATE trading_pairs SET status='disabled' WHERE id=?")
        .bind(reference)
        .execute(&pool)
        .await?;
    assert!(
        load_follow_reference(&pool, &cache, reference + 1, reference, now, 15, None)
            .await?
            .observation
            .is_none()
    );
    Ok(())
}
