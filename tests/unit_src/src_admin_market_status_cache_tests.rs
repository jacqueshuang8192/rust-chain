use super::*;
use crate::modules::admin::application::get_admin_market_feed_status;
use crate::workers::market_feed::MarketFeedRuntimeStatus;

#[tokio::test]
async fn status_reads_redis_without_mysql_and_rejects_stale_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("MARKET_RETENTION_TEST_REDIS_URL") else {
        return Ok(());
    };
    assert_eq!(url::Url::parse(&url)?.host_str(), Some("127.0.0.1"));
    let mut redis = redis::Client::open(url)?.get_connection_manager().await?;
    let _: usize = redis.del(STATUS_KEY).await?;
    assert!(
        get_admin_market_feed_status(Some(redis.clone()), MarketFeedRuntimeStatus::default())
            .await
            .is_err()
    );
    let symbol = "STATUSCHECKUSDT";
    let now = Utc::now();
    let _: () = redis.set(market_ticker_redis_key(symbol), serde_json::json!({"symbol":symbol,"provider":"bitget","observed_at":now.timestamp_millis()}).to_string()).await?;
    let _: () = redis
        .set(
            format!("market:received:v1:{symbol}"),
            now.timestamp_millis(),
        )
        .await?;
    let runtime = || MarketFeedRuntimeStatus {
        symbols: vec![symbol.into()],
        providers: vec!["bitget".into()],
        ready: true,
        ..Default::default()
    };
    let mut context = CachedMarketStatus {
        saved_config: None,
        kline_gap_count: 0,
        kline_recovery_failed_count: 0,
        checked_at: now,
    };
    store_market_status(&redis, &context).await?;
    let result = get_admin_market_feed_status(Some(redis.clone()), runtime()).await?;
    assert_eq!(result.health.healthy_symbols, 1);
    assert_eq!(
        result.health.last_ingested_at.unwrap().timestamp_millis(),
        now.timestamp_millis()
    );
    let mut wrong_source = runtime();
    wrong_source.providers = vec!["htx".into()];
    assert_eq!(
        get_admin_market_feed_status(Some(redis.clone()), wrong_source)
            .await?
            .health
            .healthy_symbols,
        0
    );
    context.checked_at = now - chrono::TimeDelta::seconds(31);
    store_market_status(&redis, &context).await?;
    let error = get_admin_market_feed_status(Some(redis.clone()), runtime())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AppError::Api {
            code: "MARKET_STATUS_NOT_READY",
            ..
        }
    ));
    Ok(())
}
