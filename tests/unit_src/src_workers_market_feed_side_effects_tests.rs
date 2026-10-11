use super::*;
use crate::modules::market::{
    MarketCacheWriteOutcome, MarketTickerSnapshot, RedisMarketCache, market_ticker_redis_key,
};
use bigdecimal::BigDecimal;
use chrono::{TimeZone, Utc};

#[test]
fn work_lanes_are_distinct_and_stable() {
    let keys = ExternalMarketLane::ALL.map(ExternalMarketLane::key);
    assert_eq!(
        keys.iter().collect::<std::collections::HashSet<_>>().len(),
        4
    );
    assert!(
        keys.iter()
            .all(|key| key.starts_with("market:external-work:v1:"))
    );
}

#[test]
fn queued_event_preserves_full_source_time() {
    let observed_at = Utc.timestamp_micros(1_710_000_000_123_456).unwrap();
    let event = ExternalMarketEvent::Ticker {
        snapshot: MarketTickerSnapshot::new(
            MarketDataProvider::Coinbase,
            "BTCUSDT",
            100.into(),
            1.into(),
            observed_at,
        )
        .unwrap(),
        generation: 3,
    };
    let job = QueuedMarketEvent {
        observed_at: event.observed_at(),
        event,
    };
    let decoded: QueuedMarketEvent =
        serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap();
    assert_eq!(decoded.observed_at, observed_at);
    assert_eq!(
        decoded.event.observed_at().timestamp_millis(),
        observed_at.timestamp_millis()
    );
    assert_ne!(
        decoded.event.observed_at(),
        observed_at,
        "public payload remains millisecond precision"
    );
}

fn ticker(symbol: &str, millis: i64, price: i64) -> ExternalMarketEvent {
    ExternalMarketEvent::Ticker {
        snapshot: MarketTickerSnapshot::new(
            MarketDataProvider::Bitget,
            symbol,
            BigDecimal::from(price),
            BigDecimal::from(1),
            Utc.timestamp_millis_opt(millis).unwrap(),
        )
        .unwrap(),
        generation: 1,
    }
}

#[tokio::test]
async fn durable_pipeline_real_redis_mysql() -> Result<(), Box<dyn std::error::Error>> {
    let (Ok(redis_url), Ok(mysql_url)) = (
        std::env::var("EXTERNAL_MARKET_TEST_REDIS_URL"),
        std::env::var("EXTERNAL_MARKET_TEST_DATABASE_URL"),
    ) else {
        eprintln!("skipping durable pipeline: isolated EXTERNAL_MARKET_TEST_* URLs not configured");
        return Ok(());
    };
    for raw in [&redis_url, &mysql_url] {
        let url = url::Url::parse(raw)?;
        assert!(matches!(
            url.host_str(),
            Some("127.0.0.1" | "localhost" | "::1")
        ));
    }
    assert!(url::Url::parse(&mysql_url)?.path().ends_with("_test"));
    let mut redis = redis::Client::open(redis_url.as_str())?
        .get_connection_manager()
        .await?;
    for lane in ExternalMarketLane::ALL {
        assert_eq!(
            redis.llen::<_, usize>(lane.key()).await?,
            0,
            "use an empty isolated Redis database"
        );
    }
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(8)
        .connect(&mysql_url)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    verify_creation_time_cutoffs(&pool).await?;
    let mongo = mongodb::Client::with_uri_str(
        "mongodb://127.0.0.1:1/?serverSelectionTimeoutMS=100&connectTimeoutMS=100",
    )
    .await?
    .database("market_pipeline_test");
    let state = AppState::new(super::super::tests::test_settings())
        .with_mysql(pool.clone())
        .with_redis(redis.clone())
        .with_mongo(mongo)
        .with_event_broadcast_hub(crate::modules::events::EventBroadcastHub::new(64));
    let cache = RedisMarketCache::new(redis.clone());
    let symbol =
        format!("Q{}USDT", &uuid::Uuid::new_v4().simple().to_string()[..12]).to_uppercase();
    let base = Utc::now().timestamp_millis() - 10_000;
    let first = ticker(&symbol, base, 100);
    assert!(
        cache
            .admit_external(&ticker(&symbol, 0, 100))
            .await
            .is_err()
    );
    assert!(
        cache
            .admit_external(&ticker(&symbol, base, 0))
            .await
            .is_err()
    );
    let _: () = redis
        .set(ExternalMarketLane::Margin.key(), "wrong-type-fixture")
        .await?;
    assert!(cache.admit_external(&first).await.is_err());
    assert_eq!(
        redis
            .llen::<_, usize>(ExternalMarketLane::Archive.key())
            .await?,
        0
    );
    assert_eq!(
        redis
            .llen::<_, usize>(ExternalMarketLane::Spot.key())
            .await?,
        0
    );
    assert!(
        !redis
            .exists::<_, bool>(market_ticker_redis_key(&symbol))
            .await?
    );
    let _: usize = redis.del(ExternalMarketLane::Margin.key()).await?;
    assert_eq!(
        cache.admit_external_with_capacity(&first, 2).await?,
        MarketCacheWriteOutcome::Accepted
    );
    assert_eq!(
        cache.admit_external(&first).await?,
        MarketCacheWriteOutcome::ReplayedIdentical
    );
    assert_eq!(
        cache.admit_external(&ticker(&symbol, base - 1, 99)).await?,
        MarketCacheWriteOutcome::RejectedStale
    );
    assert_eq!(
        cache.admit_external(&ticker(&symbol, base, 99)).await?,
        MarketCacheWriteOutcome::RejectedStale
    );
    let fence = super::super::MarketFeedGenerationFence::default();
    fence.activate(1).await;
    let worker = super::super::market_feed_worker_for_generation(&state, 1, fence.clone())?;
    let channel = crate::modules::events::WebSocketChannel::public("ticker", &symbol)?;
    let mut messages = state
        .event_broadcast_hub
        .as_ref()
        .unwrap()
        .subscribe(&channel);
    let frame = |time: i64, price: i64| {
        crate::modules::market::adapters::MarketFeedFrame::bitget_ticker(
            serde_json::json!({"arg":{"instId":symbol},"data":[{
                "lastPr":price.to_string(), "baseVolume":"1", "ts":time.to_string()
            }]})
            .to_string(),
        )
    };
    worker.ingest_frame(&frame(base, 100)).await?;
    assert_eq!(
        timeout(Duration::from_millis(100), messages.recv())
            .await??
            .channel(),
        &channel,
        "another API instance must be able to publish a snapshot already accepted into shared Redis"
    );
    worker.ingest_frame(&frame(base - 1, 99)).await?;
    assert!(
        timeout(Duration::from_millis(20), messages.recv())
            .await
            .is_err(),
        "stale prices must not be published"
    );
    let _: () = redis
        .set(
            ExternalMarketLane::Archive.key(),
            "obsolete wrong-type archive",
        )
        .await?;
    let second = ticker(&symbol, base + 1, 101);
    assert_eq!(
        cache.admit_external_with_capacity(&second, 7).await?,
        MarketCacheWriteOutcome::Accepted
    );
    let _: usize = redis.del(ExternalMarketLane::Archive.key()).await?;
    fence.activate(2).await;
    assert!(worker.ingest_frame(&frame(base + 2, 102)).await.is_err());
    let third = ticker(&symbol, base + 2, 102);
    assert!(cache.admit_external_with_capacity(&third, 2).await.is_err());
    let value: String = redis.get(market_ticker_redis_key(&symbol)).await?;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&value)?["observed_at"],
        base + 1
    );
    for lane in [
        ExternalMarketLane::Archive,
        ExternalMarketLane::Spot,
        ExternalMarketLane::Margin,
    ] {
        assert_eq!(
            redis.llen::<_, usize>(lane.key()).await?,
            if matches!(lane, ExternalMarketLane::Archive) {
                0
            } else {
                2
            }
        );
    }

    // 资金队列仍持久化；遗留归档工作被安全确认，不再依赖行情历史表。
    let head: Vec<String> = redis.lrange(ExternalMarketLane::Spot.key(), 0, -1).await?;
    let _: usize = redis
        .rpush(ExternalMarketLane::Archive.key(), &head)
        .await?;
    assert!(
        acknowledge(
            &mut redis,
            ExternalMarketLane::Archive,
            &["wrong-head".into()]
        )
        .await
        .is_err()
    );
    let events = head
        .iter()
        .map(|v| serde_json::from_str(v))
        .collect::<Result<Vec<_>, _>>()?;
    sqlx::query("RENAME TABLE market_price_ticks TO market_price_ticks_held")
        .execute(&pool)
        .await?;
    retire_archive(&events).await?;
    retire_archive(&events).await?;
    sqlx::query("RENAME TABLE market_price_ticks_held TO market_price_ticks")
        .execute(&pool)
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM market_price_ticks WHERE symbol = ?")
        .bind(&symbol)
        .fetch_one(&pool)
        .await?;
    assert_eq!(
        count, 0,
        "legacy archive retirement must not persist prices"
    );
    verify_auxiliary_admission(&cache, &mut redis, &symbol, base).await?;

    // 故意持有现货通道锁；归档、杠杆和实时写入必须仍能推进。
    let mut blocker = MySqlConnection::connect_with(&pool.connect_options()).await?;
    let acquired: i64 =
        sqlx::query_scalar("SELECT GET_LOCK(SHA2(CONCAT(DATABASE(), ':', ?),256),0)")
            .bind(ExternalMarketLane::Spot.key())
            .fetch_one(&mut blocker)
            .await?;
    assert_eq!(acquired, 1);
    verify_lock(&mut blocker, ExternalMarketLane::Spot).await?;
    let cancellation = CancellationToken::new();
    let consumer = tokio::spawn(run(state.clone(), cancellation.clone()));
    let worker = super::super::market_feed_worker_for_generation(&state, 2, fence)?;
    let burst_started = Instant::now();
    for index in 2..502 {
        worker
            .ingest_frame(&frame(base + index, 100 + index))
            .await?;
        assert_eq!(
            timeout(Duration::from_millis(100), messages.recv())
                .await??
                .channel(),
            &channel
        );
    }
    assert!(
        burst_started.elapsed() < Duration::from_secs(5),
        "slow spot lane blocked realtime admission"
    );
    timeout(Duration::from_secs(20), async {
        loop {
            let archived: usize = redis.llen(ExternalMarketLane::Archive.key()).await.unwrap();
            let margin: usize = redis.llen(ExternalMarketLane::Margin.key()).await.unwrap();
            if archived == 0 && margin == 0 {
                break;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await?;
    assert_eq!(
        redis
            .llen::<_, usize>(ExternalMarketLane::Spot.key())
            .await?,
        503
    );
    cancellation.cancel();
    timeout(Duration::from_secs(5), consumer).await???;
    blocker.close().await?;
    // 取消后队列不丢失，下一代消费者继续按原 FIFO 完成。
    let cancellation = CancellationToken::new();
    let consumer = tokio::spawn(run(state.clone(), cancellation.clone()));
    timeout(Duration::from_secs(20), async {
        loop {
            if redis
                .llen::<_, usize>(ExternalMarketLane::Spot.key())
                .await
                .unwrap()
                == 0
            {
                break;
            }
            sleep(Duration::from_millis(25)).await;
        }
    })
    .await?;
    cancellation.cancel();
    timeout(Duration::from_secs(5), consumer).await???;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM market_price_ticks WHERE symbol = ?")
        .bind(&symbol)
        .fetch_one(&pool)
        .await?;
    assert_eq!(count, 0, "live consumption must not archive ticker history");
    let precise_time =
        Utc.timestamp_millis_opt(base + 501).unwrap() + chrono::Duration::microseconds(123);
    let precise = ExternalMarketEvent::Ticker {
        snapshot: MarketTickerSnapshot::new(
            MarketDataProvider::Bitget,
            &symbol,
            601.into(),
            1.into(),
            precise_time,
        )?,
        generation: 2,
    };
    assert_eq!(
        cache.admit_external(&precise).await?,
        MarketCacheWriteOutcome::ReplayedIdentical
    );
    assert_eq!(
        redis
            .llen::<_, usize>(ExternalMarketLane::Spot.key())
            .await?,
        0
    );
    assert_eq!(
        redis
            .llen::<_, usize>(ExternalMarketLane::Margin.key())
            .await?,
        0
    );
    let pending_archive: Vec<String> = redis
        .lrange(ExternalMarketLane::Archive.key(), 0, -1)
        .await?;
    assert!(
        pending_archive.is_empty(),
        "replay must not create archive jobs"
    );
    let history_exists: bool = redis
        .exists(format!("market:price-history:v1:{symbol}"))
        .await?;
    assert!(!history_exists, "no Redis ticker history is maintained");
    assert_eq!(
        redis
            .llen::<_, usize>(ExternalMarketLane::Kline.key())
            .await?,
        1,
        "Mongo failure must retain the history job"
    );
    // Mongo 在本夹具中故意不可用，以下仅清理测试事件，不是生产确认路径。
    let pending: Vec<String> = redis.lrange(ExternalMarketLane::Kline.key(), 0, -1).await?;
    acknowledge(&mut redis, ExternalMarketLane::Kline, &pending).await?;
    let _: usize = redis
        .del(vec![
            crate::modules::market::market_depth_redis_key(&symbol),
            crate::modules::market::market_kline_redis_key(&symbol, "1m"),
            format!("market:kline-sequence:{symbol}:1m"),
        ])
        .await?;
    let _: usize = redis.del(market_ticker_redis_key(&symbol)).await?;
    sqlx::query("DELETE FROM market_price_ticks WHERE symbol = ?")
        .bind(&symbol)
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}

async fn verify_auxiliary_admission(
    cache: &RedisMarketCache,
    redis: &mut redis::aio::ConnectionManager,
    symbol: &str,
    base: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::modules::market::{
        MarketDepthLevel, MarketDepthSnapshot, MarketKlineSnapshot, MarketKlineValues,
        market_kline_redis_key,
    };
    let depth = |millis| {
        ExternalMarketEvent::Depth(
            MarketDepthSnapshot::new(
                MarketDataProvider::Bitget,
                symbol,
                vec![MarketDepthLevel::new(99.into(), 1.into())],
                vec![MarketDepthLevel::new(101.into(), 1.into())],
                Utc.timestamp_millis_opt(millis).unwrap(),
            )
            .unwrap(),
        )
    };
    assert_eq!(
        cache.admit_external(&depth(base)).await?,
        MarketCacheWriteOutcome::Accepted
    );
    assert_eq!(
        cache.admit_external(&depth(base)).await?,
        MarketCacheWriteOutcome::ReplayedIdentical
    );
    assert_eq!(
        cache.admit_external(&depth(base - 1)).await?,
        MarketCacheWriteOutcome::RejectedStale
    );
    let open = Utc.timestamp_millis_opt(base / 60_000 * 60_000).unwrap();
    let candle = |millis| {
        ExternalMarketEvent::Kline(
            MarketKlineSnapshot::new(
                MarketDataProvider::Bitget,
                symbol,
                "1m",
                open,
                MarketKlineValues {
                    open: 100.into(),
                    high: 101.into(),
                    low: 99.into(),
                    close: 100.into(),
                    volume: 1.into(),
                },
                Utc.timestamp_millis_opt(millis).unwrap(),
            )
            .unwrap(),
        )
    };
    assert_eq!(
        cache.admit_external(&candle(base)).await?,
        MarketCacheWriteOutcome::Accepted
    );
    assert_eq!(
        cache.admit_external(&candle(base)).await?,
        MarketCacheWriteOutcome::ReplayedIdentical
    );
    assert_eq!(
        cache.admit_external(&candle(base - 1)).await?,
        MarketCacheWriteOutcome::RejectedStale
    );
    let payload: String = redis.get(market_kline_redis_key(symbol, "1m")).await?;
    let _: usize = redis.del(market_kline_redis_key(symbol, "1m")).await?;
    assert_eq!(
        cache.admit_external(&candle(base - 1)).await?,
        MarketCacheWriteOutcome::RejectedStale,
        "the sequence fence must survive a missing public cache key"
    );
    let _: () = redis
        .set(market_kline_redis_key(symbol, "1m"), payload)
        .await?;
    Ok(())
}

async fn verify_creation_time_cutoffs(
    pool: &sqlx::MySqlPool,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::modules::{
        margin::infrastructure::triggered_margin_limit_position_ids, spot::infrastructure::*,
    };
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let user = sqlx::query("INSERT INTO users (password_hash) VALUES ('test-only')")
        .execute(pool)
        .await?
        .last_insert_id();
    let mut assets = Vec::new();
    for prefix in ["QB", "QQ"] {
        assets.push(sqlx::query("INSERT INTO assets (symbol, name, precision_scale, asset_type, status) VALUES (?, 'test', 8, 'coin', 'active')")
            .bind(format!("{prefix}{}", &suffix[..10])).execute(pool).await?.last_insert_id());
    }
    let symbol = format!("QC{}USDT", &suffix[..10]).to_uppercase();
    let pair = sqlx::query("INSERT INTO trading_pairs (base_asset, quote_asset, symbol, price_precision, qty_precision, min_order_value, status, market_type) VALUES (?, ?, ?, 8, 8, 1, 'active', 'spot')")
        .bind(assets[0]).bind(assets[1]).bind(&symbol).execute(pool).await?.last_insert_id();
    let at = Utc::now().naive_utc();
    for side in ["buy", "sell"] {
        for kind in ["limit", "stop_limit"] {
            for offset in [-1, 0, 1] {
                sqlx::query("INSERT INTO spot_orders (user_id, pair_id, side, order_type, price, trigger_price, quantity, created_at) VALUES (?, ?, ?, ?, 100, 100, 1, ?)")
                    .bind(user).bind(pair).bind(side).bind(kind)
                    .bind(at + chrono::Duration::seconds(offset)).execute(pool).await?;
            }
        }
    }
    let price = BigDecimal::from(100);
    assert_eq!(
        triggered_limit_buy_order_ids(pool, &symbol, &price, 20, Some(at))
            .await?
            .len(),
        1
    );
    assert_eq!(
        triggered_limit_sell_order_ids(pool, &symbol, &price, 20, Some(at))
            .await?
            .len(),
        1
    );
    assert_eq!(
        triggered_stop_limit_buy_order_ids(pool, &symbol, &price, 20, Some(at))
            .await?
            .len(),
        1
    );
    assert_eq!(
        triggered_stop_limit_sell_order_ids(pool, &symbol, &price, 20, Some(at))
            .await?
            .len(),
        1
    );
    assert_eq!(
        triggered_limit_buy_order_ids(pool, &symbol, &price, 20, None)
            .await?
            .len(),
        3
    );
    assert_eq!(
        execute_triggered_spot_limit_orders_with_hub_at(
            pool,
            &symbol,
            &price,
            None,
            Some(at - chrono::Duration::seconds(2))
        )
        .await?,
        0
    );
    assert_eq!(
        execute_triggered_spot_limit_orders_with_hub_at(pool, &symbol, &price, None, Some(at))
            .await?,
        0,
        "unfunded fixtures must preserve existing per-order failure isolation"
    );
    let product = sqlx::query(r#"INSERT INTO margin_products (pair_id, margin_asset, margin_modes, leverage_levels, max_leverage, min_margin, maintenance_margin_rate)
        VALUES (?, ?, '["isolated"]', '[2]', 2, 1, 0.01)"#)
        .bind(pair).bind(assets[1]).execute(pool).await?.last_insert_id();
    for direction in ["long", "short"] {
        for offset in [-1, 0, 1] {
            sqlx::query("INSERT INTO margin_positions (user_id, product_id, pair_id, margin_asset, direction, order_type, margin_amount, leverage, notional_amount, limit_price, idempotency_key, created_at) VALUES (?, ?, ?, ?, ?, 'limit', 10, 2, 20, 100, ?, ?)")
                .bind(user).bind(product).bind(pair).bind(assets[1]).bind(direction)
                .bind(format!("{direction}{offset}")).bind(at + chrono::Duration::seconds(offset))
                .execute(pool).await?;
        }
    }
    assert_eq!(
        triggered_margin_limit_position_ids(pool, &symbol, &price, 500, Some(at))
            .await?
            .len(),
        2
    );
    assert_eq!(
        triggered_margin_limit_position_ids(pool, &symbol, &price, 500, None)
            .await?
            .len(),
        6
    );
    assert_eq!(
        execute_triggered_margin_limit_orders_with_hub_at(pool, &symbol, &price, None, Some(at))
            .await?,
        2
    );
    assert_eq!(
        execute_triggered_margin_limit_orders_with_hub_at(pool, &symbol, &price, None, Some(at))
            .await?,
        0
    );
    let filled: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM margin_positions WHERE user_id = ? AND entry_price IS NOT NULL",
    )
    .bind(user)
    .fetch_one(pool)
    .await?;
    assert_eq!(
        filled, 2,
        "replayed quote must neither fill newer orders nor fill an order twice"
    );
    // 失败的现货事务已回滚；只清理本夹具的挂单和外键树，不触碰其他测试账户。
    sqlx::query("DELETE FROM margin_positions WHERE user_id = ?")
        .bind(user)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM margin_products WHERE id = ?")
        .bind(product)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM spot_orders WHERE user_id = ?")
        .bind(user)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM trading_pairs WHERE id = ?")
        .bind(pair)
        .execute(pool)
        .await?;
    for asset in assets {
        sqlx::query("DELETE FROM assets WHERE id = ?")
            .bind(asset)
            .execute(pool)
            .await?;
    }
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user)
        .execute(pool)
        .await?;
    Ok(())
}
