use super::*;
use crate::modules::market::{MarketDataProvider, MarketTickerSnapshot};

#[tokio::test]
async fn latest_cache_has_no_history_dependency_and_keeps_exact_prices()
-> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("LATEST_PRICE_TEST_REDIS_URL") else {
        return Ok(());
    };
    assert_eq!(url::Url::parse(&url)?.host_str(), Some("127.0.0.1"));
    let mut redis = redis::Client::open(url)?.get_connection_manager().await?;
    let symbol =
        format!("L{}USDT", &uuid::Uuid::new_v4().simple().to_string()[..10]).to_uppercase();
    let cache = RedisMarketCache::new(redis.clone());
    let old_history_key = format!("market:price-history:v1:{symbol}");
    let _: () = redis
        .set(&old_history_key, "obsolete wrong-type history")
        .await?;
    assert!(cache.latest_price(&symbol).await?.is_none());
    let now = Utc::now();
    for (offset, value) in [(0, "101.123456789012345678"), (1, "99.000000000000000001")] {
        let price: BigDecimal = value.parse()?;
        let snapshot = MarketTickerSnapshot::new(
            MarketDataProvider::Bitget,
            &symbol,
            price.clone(),
            1.into(),
            now + chrono::TimeDelta::seconds(offset),
        )?;
        cache
            .save_ticker(MarketTickerCacheEntry::from_snapshot(&snapshot)?)
            .await?;
        let selected = cache.latest_price(&symbol).await?.unwrap();
        assert_eq!(selected.price, price);
        assert_eq!(selected.source, "bitget");
        assert!(selected.source_version.starts_with("redis:"));
    }
    assert_eq!(
        redis.get::<_, String>(&old_history_key).await?,
        "obsolete wrong-type history"
    );
    let _: () = redis
        .set(market_ticker_redis_key(&symbol), "bad JSON")
        .await?;
    assert!(cache.latest_price(&symbol).await.is_err());
    Ok(())
}
