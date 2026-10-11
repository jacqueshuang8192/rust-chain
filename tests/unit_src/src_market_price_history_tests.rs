use super::*;
use crate::modules::market::infrastructure::cache::external::{
    ExternalMarketEvent, ExternalMarketLane,
};
use crate::modules::market::{MarketDataProvider, MarketTickerSnapshot};
use redis::AsyncCommands;

#[tokio::test]
async fn redis_price_history_is_atomic_bounded_and_windowed()
-> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("MARKET_RETENTION_TEST_REDIS_URL") else {
        return Ok(());
    };
    assert_eq!(url::Url::parse(&url)?.host_str(), Some("127.0.0.1"));
    let mut redis = redis::Client::open(url)?.get_connection_manager().await?;
    let cache = RedisMarketCache::new(redis.clone());
    let symbol =
        format!("H{}USDT", &uuid::Uuid::new_v4().simple().to_string()[..10]).to_uppercase();
    let now = Utc::now() - chrono::TimeDelta::seconds(30);
    let snapshot = |at, price| {
        MarketTickerSnapshot::new(
            MarketDataProvider::Bitget,
            &symbol,
            BigDecimal::from(price),
            1.into(),
            at,
        )
        .unwrap()
    };
    let event = ExternalMarketEvent::Ticker {
        snapshot: snapshot(now, 101),
        generation: 1,
    };
    let key = price_history_key(&symbol);
    let _: () = redis.set(&key, "invalid").await?;
    assert!(cache.admit_external(&event).await.is_err());
    assert!(
        !redis
            .exists::<_, bool>(market_ticker_redis_key(&symbol))
            .await?
    );
    let _: usize = redis.del(&key).await?;
    assert!(cache.admit_external(&event).await?.is_accepted());
    assert!(cache.admit_external(&event).await?.is_identical_replay());
    assert_eq!(redis.zcard::<_, usize>(&key).await?, 1);
    let deadline = now - chrono::TimeDelta::milliseconds(1);
    let first = load_price_window(
        &redis,
        &symbol,
        deadline,
        deadline + chrono::TimeDelta::seconds(5),
    )
    .await?
    .unwrap();
    assert_eq!(first.price, BigDecimal::from(101));
    assert_eq!(first.observed_at, now);
    let later =
        MarketTickerCacheEntry::from_snapshot(&snapshot(now + chrono::TimeDelta::seconds(10), 1))?;
    cache.save_ticker(later).await?;
    assert_eq!(
        load_price_window(
            &redis,
            &symbol,
            deadline,
            deadline + chrono::TimeDelta::seconds(5)
        )
        .await?
        .unwrap()
        .price,
        first.price
    );
    assert!(
        load_price_window(&redis, &symbol, now - chrono::TimeDelta::seconds(1), now)
            .await?
            .is_none()
    );
    let _: usize = redis
        .zadd(
            &key,
            "expired",
            (now - chrono::TimeDelta::days(2)).timestamp_micros(),
        )
        .await?;
    cache
        .save_ticker(MarketTickerCacheEntry::from_snapshot(&snapshot(
            now + chrono::TimeDelta::seconds(11),
            2,
        ))?)
        .await?;
    assert!(
        redis
            .zscore::<_, _, Option<f64>>(&key, "expired")
            .await?
            .is_none()
    );
    let ttl: i64 = redis.ttl(&key).await?;
    assert!((86_390..=86_400).contains(&ttl));
    for lane in ExternalMarketLane::ALL {
        let _: usize = redis.del(lane.key()).await?;
    }
    Ok(())
}
