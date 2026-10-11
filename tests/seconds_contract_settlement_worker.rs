use bigdecimal::BigDecimal;
use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use exchange_api::workers::seconds_contract_settlement::{
    run_once_with_cached_prices, run_once_with_pool, run_once_with_pool_and_max_wait,
    seconds_contract_settlement_result,
};
use sqlx::{MySql, MySqlPool, Transaction, mysql::MySqlPoolOptions};
use std::{error::Error, str::FromStr};
use tokio::sync::Mutex;
use uuid::Uuid;

static TEST_LOCK: Mutex<()> = Mutex::const_new(());

fn decimal(value: &str) -> BigDecimal {
    BigDecimal::from_str(value).unwrap()
}

async fn cached_test_redis() -> Result<Option<redis::aio::ConnectionManager>, Box<dyn Error>> {
    let Ok(url) = std::env::var("REDIS_URL") else {
        eprintln!("skipping cached-price settlement test because REDIS_URL is not set");
        return Ok(None);
    };
    Ok(Some(
        redis::aio::ConnectionManager::new(redis::Client::open(url)?).await?,
    ))
}

async fn seed_cached_price(
    redis: &redis::aio::ConnectionManager,
    symbol: &str,
    price: &str,
    observed_at: DateTime<Utc>,
) -> Result<(), Box<dyn Error>> {
    use redis::AsyncCommands;
    let mut connection = redis.clone();
    let _: () = connection
        .set(
            exchange_api::modules::market::market_ticker_redis_key(symbol),
            serde_json::json!({
                "symbol": symbol.replace('-', ""), "last_price": price,
                "observed_at": observed_at.timestamp_millis(),
            })
            .to_string(),
        )
        .await?;
    Ok(())
}

#[tokio::test]
async fn redis_settlement_is_due_immediately_and_replay_retains_the_selected_old_quote()
-> Result<(), Box<dyn Error>> {
    let _guard = TEST_LOCK.lock().await;
    let (Some(pool), Some(redis)) = (mysql_pool_or_skip().await?, cached_test_redis().await?)
    else {
        return Ok(());
    };
    sqlx::query("UPDATE seconds_contract_orders SET status='settled' WHERE status='opened' AND idempotency_key LIKE 'event-price-%'")
        .execute(&pool).await?;
    let fixture = seed_fixture(&pool).await?;
    let expires_at = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
    let observed_at = expires_at - TimeDelta::hours(2);
    let order_id = seed_order(&pool, &fixture, expires_at, "up").await?;
    // A conflicting historical price proves the new path takes Redis, not the event window.
    insert_tick(&pool, &fixture.pair_symbol, "90", "bitget", expires_at, 1).await?;
    seed_cached_price(
        &redis,
        &fixture.pair_symbol,
        "101.123456789012345678",
        observed_at,
    )
    .await?;
    let early = run_once_with_cached_prices(
        &pool,
        &redis,
        None,
        expires_at - TimeDelta::milliseconds(1),
        1,
    )
    .await?;
    assert_eq!(early.scanned, 0);
    use exchange_api::modules::events::{EventBroadcastHub, WebSocketChannel};
    let user_id: u64 = sqlx::query_scalar("SELECT user_id FROM seconds_contract_orders WHERE id=?")
        .bind(order_id)
        .fetch_one(&pool)
        .await?;
    let hub = EventBroadcastHub::new(16);
    let mut events = hub.subscribe(&WebSocketChannel::private_user(user_id));
    let (first, second) = tokio::join!(
        run_once_with_cached_prices(&pool, &redis, Some(&hub), expires_at, 1),
        run_once_with_cached_prices(&pool, &redis, Some(&hub), expires_at, 1),
    );
    assert_eq!(first?.settled + second?.settled, 1);
    let event =
        tokio::time::timeout(std::time::Duration::from_millis(100), events.recv()).await??;
    let event: serde_json::Value = serde_json::from_str(event.payload())?;
    assert_eq!(event["status"], "settled");
    assert_eq!(event["settlement_price_source"], "redis");
    assert!(event["settlement_price_tick_id"].is_null());
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), events.recv())
            .await
            .is_err()
    );
    let snapshot = order_snapshot(&pool, order_id).await?;
    assert_eq!(snapshot.0, "settled");
    assert_eq!(snapshot.1, Some(decimal("101.123456789012345678")));
    assert_eq!(snapshot.2, None);
    assert_eq!(snapshot.3.as_deref(), Some("redis"));
    assert_eq!(snapshot.4, Some(observed_at));
    assert_eq!(snapshot.5, None);
    assert!(snapshot.6.as_deref().unwrap().starts_with("redis:"));
    let funds: (BigDecimal, i64) = sqlx::query_as(
        "SELECT wallet.available, (SELECT COUNT(*) FROM wallet_ledger WHERE ref_type='seconds_contract_order' AND ref_id=CAST(orders.id AS CHAR)) FROM seconds_contract_orders orders JOIN wallet_accounts wallet ON wallet.user_id=orders.user_id AND wallet.asset_id=orders.stake_asset WHERE orders.id=?",
    ).bind(order_id).fetch_one(&pool).await?;
    assert_eq!(funds, (decimal("58"), 1));
    seed_cached_price(&redis, &fixture.pair_symbol, "1", expires_at).await?;
    let replay =
        run_once_with_cached_prices(&pool, &redis, None, expires_at + TimeDelta::seconds(1), 1)
            .await?;
    assert_eq!(replay.settled, 0);
    assert_eq!(order_snapshot(&pool, order_id).await?, snapshot);
    let journal: (i64, BigDecimal) = sqlx::query_as(
        "SELECT COUNT(*),COALESCE(SUM(amount),0) FROM platform_financial_journal WHERE transaction_key=?",
    ).bind(format!("seconds_contract:{order_id}:settle")).fetch_one(&pool).await?;
    assert_eq!(journal, (3, decimal("0")));
    Ok(())
}

#[tokio::test]
async fn redis_settlement_transaction_failure_rolls_back_price_funds_and_event()
-> Result<(), Box<dyn Error>> {
    let _guard = TEST_LOCK.lock().await;
    let (Some(pool), Some(redis)) = (mysql_pool_or_skip().await?, cached_test_redis().await?)
    else {
        return Ok(());
    };
    sqlx::query("UPDATE seconds_contract_orders SET status='settled' WHERE status='opened' AND idempotency_key LIKE 'event-price-%'")
        .execute(&pool).await?;
    let fixture = seed_fixture(&pool).await?;
    let expires_at = Utc.with_ymd_and_hms(2020, 3, 1, 0, 0, 0).unwrap();
    let order_id = seed_order(&pool, &fixture, expires_at, "up").await?;
    seed_cached_price(&redis, &fixture.pair_symbol, "101", expires_at).await?;
    sqlx::raw_sql(&format!(
        "CREATE TRIGGER seconds_cache_reject_ledger BEFORE INSERT ON wallet_ledger FOR EACH ROW BEGIN IF NEW.ref_type='seconds_contract_order' AND NEW.ref_id='{order_id}' THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='seconds cache rollback probe'; END IF; END"
    )).execute(&pool).await?;
    let result = run_once_with_cached_prices(&pool, &redis, None, expires_at, 1).await;
    sqlx::raw_sql("DROP TRIGGER seconds_cache_reject_ledger")
        .execute(&pool)
        .await?;
    assert_eq!(result?.failed, 1);
    let snapshot = order_snapshot(&pool, order_id).await?;
    assert_eq!(snapshot.0, "opened");
    assert!(snapshot.1.is_none() && snapshot.2.is_none() && snapshot.3.is_none());
    let available: BigDecimal = sqlx::query_scalar(
        "SELECT available FROM wallet_accounts WHERE (user_id,asset_id)=(SELECT user_id,stake_asset FROM seconds_contract_orders WHERE id=?)",
    ).bind(order_id).fetch_one(&pool).await?;
    assert_eq!(available, decimal("40"));
    let journal_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM platform_financial_journal WHERE transaction_key=?",
    )
    .bind(format!("seconds_contract:{order_id}:settle"))
    .fetch_one(&pool)
    .await?;
    assert_eq!(journal_count, 0);
    let settled =
        run_once_with_cached_prices(&pool, &redis, None, expires_at + TimeDelta::seconds(1), 1)
            .await?;
    assert_eq!(settled.settled, 1);
    Ok(())
}
#[tokio::test]
async fn redis_settlement_retries_missing_and_invalid_cache_after_one_second()
-> Result<(), Box<dyn Error>> {
    let _guard = TEST_LOCK.lock().await;
    let (Some(pool), Some(redis)) = (mysql_pool_or_skip().await?, cached_test_redis().await?)
    else {
        return Ok(());
    };
    sqlx::query("UPDATE seconds_contract_orders SET status='settled' WHERE status='opened' AND idempotency_key LIKE 'event-price-%'")
        .execute(&pool).await?;
    let fixture = seed_fixture(&pool).await?;
    let expires_at = Utc.with_ymd_and_hms(2020, 2, 1, 0, 0, 0).unwrap();
    let order_id = seed_order(&pool, &fixture, expires_at, "down").await?;
    let pending = run_once_with_cached_prices(&pool, &redis, None, expires_at, 1).await?;
    assert_eq!(pending.failed, 1);
    let retry_at: DateTime<Utc> = sqlx::query_scalar(
        "SELECT next_settlement_attempt_at FROM seconds_contract_orders WHERE id=?",
    )
    .bind(order_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(retry_at, expires_at + TimeDelta::seconds(1));
    let skipped = run_once_with_cached_prices(
        &pool,
        &redis,
        None,
        expires_at + TimeDelta::milliseconds(500),
        1,
    )
    .await?;
    assert_eq!(skipped.scanned, 0);
    seed_cached_price(&redis, &fixture.pair_symbol, "0", expires_at).await?;
    let invalid = run_once_with_cached_prices(&pool, &redis, None, retry_at, 1).await?;
    assert_eq!(invalid.failed, 1);
    let unchanged: (String, BigDecimal, i64) = sqlx::query_as(
        "SELECT orders.status,wallet.available,(SELECT COUNT(*) FROM wallet_ledger WHERE ref_type='seconds_contract_order' AND ref_id=CAST(orders.id AS CHAR)) FROM seconds_contract_orders orders JOIN wallet_accounts wallet ON wallet.user_id=orders.user_id AND wallet.asset_id=orders.stake_asset WHERE orders.id=?",
    ).bind(order_id).fetch_one(&pool).await?;
    assert_eq!(unchanged, ("opened".to_owned(), decimal("40"), 0));
    seed_cached_price(
        &redis,
        &fixture.pair_symbol,
        "99",
        expires_at - TimeDelta::days(1),
    )
    .await?;
    let settled =
        run_once_with_cached_prices(&pool, &redis, None, expires_at + TimeDelta::seconds(2), 1)
            .await?;
    assert_eq!(settled.settled, 1);
    Ok(())
}

async fn mysql_pool_or_skip() -> Result<Option<MySqlPool>, Box<dyn Error>> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        eprintln!("skipping seconds event-price tests because DATABASE_URL is not set");
        return Ok(None);
    };
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(Some(pool))
}

#[tokio::test]
async fn cache_failures_rotate_behind_untouched_due_orders() -> Result<(), Box<dyn Error>> {
    let _guard = TEST_LOCK.lock().await;
    let (Some(pool), Some(redis)) = (mysql_pool_or_skip().await?, cached_test_redis().await?)
    else {
        return Ok(());
    };
    sqlx::query("UPDATE seconds_contract_orders SET status='settled' WHERE status='opened' AND idempotency_key LIKE 'event-price-%'")
        .execute(&pool).await?;
    let broken = seed_fixture(&pool).await?;
    let healthy = seed_fixture(&pool).await?;
    let expires_at = Utc.with_ymd_and_hms(2020, 4, 1, 0, 0, 0).unwrap();
    let bad_order = seed_order(&pool, &broken, expires_at, "up").await?;
    let good_order = seed_order(&pool, &healthy, expires_at + TimeDelta::seconds(1), "up").await?;
    seed_cached_price(&redis, &healthy.pair_symbol, "101", expires_at).await?;
    let now = expires_at + TimeDelta::seconds(10);
    assert_eq!(
        run_once_with_cached_prices(&pool, &redis, None, now, 1)
            .await?
            .failed,
        1
    );
    // The failed retry is eligible again, but must not occupy LIMIT 1 forever.
    let next =
        run_once_with_cached_prices(&pool, &redis, None, now + TimeDelta::seconds(1), 1).await?;
    assert_eq!(next.settled, 1);
    assert_eq!(order_snapshot(&pool, good_order).await?.0, "settled");
    assert_eq!(order_snapshot(&pool, bad_order).await?.0, "opened");
    seed_cached_price(&redis, &broken.pair_symbol, "101", now).await?;
    assert_eq!(
        run_once_with_cached_prices(&pool, &redis, None, now + TimeDelta::seconds(2), 1)
            .await?
            .settled,
        1
    );
    Ok(())
}

#[tokio::test]
async fn locked_order_does_not_hold_up_other_users_on_the_same_pair() -> Result<(), Box<dyn Error>>
{
    let _guard = TEST_LOCK.lock().await;
    let (Some(pool), Some(redis)) = (mysql_pool_or_skip().await?, cached_test_redis().await?)
    else {
        return Ok(());
    };
    sqlx::query("UPDATE seconds_contract_orders SET status='settled' WHERE status='opened' AND idempotency_key LIKE 'event-price-%'")
        .execute(&pool).await?;
    let fixture = seed_fixture(&pool).await?;
    let expires_at = Utc.with_ymd_and_hms(2020, 5, 1, 0, 0, 0).unwrap();
    let held_order = seed_order(&pool, &fixture, expires_at, "up").await?;
    let free_order = seed_order(&pool, &fixture, expires_at, "up").await?;
    seed_cached_price(&redis, &fixture.pair_symbol, "101", expires_at).await?;
    let mut held = pool.begin().await?;
    sqlx::query("SELECT id FROM seconds_contract_orders WHERE id=? FOR UPDATE")
        .bind(held_order)
        .fetch_one(&mut *held)
        .await?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        run_once_with_cached_prices(&pool, &redis, None, expires_at, 1),
    )
    .await;
    held.rollback().await?;
    let summary = result??;
    assert_eq!(summary.settled, 1);
    assert_eq!(summary.skipped, 0);
    assert_eq!(order_snapshot(&pool, free_order).await?.0, "settled");
    assert_eq!(
        run_once_with_cached_prices(&pool, &redis, None, expires_at, 10)
            .await?
            .settled,
        1
    );
    Ok(())
}

async fn create_user(tx: &mut Transaction<'_, MySql>) -> Result<u64, Box<dyn Error>> {
    Ok(
        sqlx::query("INSERT INTO users (email, password_hash) VALUES (?, 'test')")
            .bind(format!("event-price-{}@test.invalid", Uuid::now_v7()))
            .execute(&mut **tx)
            .await?
            .last_insert_id(),
    )
}

async fn create_asset(
    tx: &mut Transaction<'_, MySql>,
    prefix: &str,
) -> Result<(u64, String), Box<dyn Error>> {
    let suffix = Uuid::now_v7().simple().to_string();
    let symbol = format!("{}{}", prefix, &suffix[24..]).to_ascii_uppercase();
    let id = sqlx::query(
        "INSERT INTO assets (symbol, name, precision_scale, asset_type, status) VALUES (?, ?, 18, 'coin', 'active')",
    )
    .bind(&symbol)
    .bind(format!("{symbol} asset"))
    .execute(&mut **tx)
    .await?
    .last_insert_id();
    Ok((id, symbol))
}

struct SecondsFixture {
    pair_symbol: String,
    product_id: u64,
    pair_id: u64,
    stake_asset: u64,
}

async fn seed_fixture(pool: &MySqlPool) -> Result<SecondsFixture, Box<dyn Error>> {
    let mut tx = pool.begin().await?;
    let (base_asset, base_symbol) = create_asset(&mut tx, "EB").await?;
    let (quote_asset, quote_symbol) = create_asset(&mut tx, "EQ").await?;
    let pair_symbol = format!("{base_symbol}-{quote_symbol}");
    let pair_id = sqlx::query(
        r#"INSERT INTO trading_pairs
           (base_asset, quote_asset, symbol, price_precision, qty_precision,
            min_order_value, status, market_type)
           VALUES (?, ?, ?, 18, 18, 1, 'active', 'external')"#,
    )
    .bind(base_asset)
    .bind(quote_asset)
    .bind(&pair_symbol)
    .execute(&mut *tx)
    .await?
    .last_insert_id();
    let product_id = sqlx::query(
        r#"INSERT INTO seconds_contract_products
           (pair_id, stake_asset, duration_seconds, payout_rate, min_stake, max_stake, status)
           VALUES (?, ?, 60, 0.8, 1, 100, 'active')"#,
    )
    .bind(pair_id)
    .bind(quote_asset)
    .execute(&mut *tx)
    .await?
    .last_insert_id();
    tx.commit().await?;
    Ok(SecondsFixture {
        pair_symbol,
        product_id,
        pair_id,
        stake_asset: quote_asset,
    })
}

async fn seed_order(
    pool: &MySqlPool,
    fixture: &SecondsFixture,
    expires_at: DateTime<Utc>,
    direction: &str,
) -> Result<u64, Box<dyn Error>> {
    let mut tx = pool.begin().await?;
    let user_id = create_user(&mut tx).await?;
    sqlx::query("INSERT INTO wallet_accounts (user_id, asset_id, available) VALUES (?, ?, 40)")
        .bind(user_id)
        .bind(fixture.stake_asset)
        .execute(&mut *tx)
        .await?;
    let order_id = sqlx::query(
        r#"INSERT INTO seconds_contract_orders
           (user_id, product_id, pair_id, stake_asset, direction, stake_amount,
            payout_rate, entry_price, status, idempotency_key, opened_at, expires_at)
           VALUES (?, ?, ?, ?, ?, 10, 0.8, 100, 'opened', ?, DATE_SUB(?, INTERVAL 60 SECOND), ?)"#,
    )
    .bind(user_id)
    .bind(fixture.product_id)
    .bind(fixture.pair_id)
    .bind(fixture.stake_asset)
    .bind(direction)
    .bind(format!("event-price-{}", Uuid::now_v7()))
    .bind(expires_at.naive_utc())
    .bind(expires_at.naive_utc())
    .execute(&mut *tx)
    .await?
    .last_insert_id();
    tx.commit().await?;
    Ok(order_id)
}

async fn insert_tick(
    pool: &MySqlPool,
    symbol: &str,
    price: &str,
    source: &str,
    observed_at: DateTime<Utc>,
    generation: u64,
) -> Result<u64, Box<dyn Error>> {
    let version = Uuid::now_v7().simple().to_string();
    Ok(sqlx::query(
        r#"INSERT INTO market_price_ticks
           (event_key, symbol, price, source, observed_at, generation, source_version)
           VALUES (?, REPLACE(UPPER(?), '-', ''), ?, ?, ?, ?, ?)"#,
    )
    .bind(format!("{version}{version}"))
    .bind(symbol)
    .bind(decimal(price))
    .bind(source)
    .bind(observed_at.naive_utc())
    .bind(generation)
    .bind(version)
    .execute(pool)
    .await?
    .last_insert_id())
}

type SnapshotRow = (
    String,
    Option<BigDecimal>,
    Option<u64>,
    Option<String>,
    Option<DateTime<Utc>>,
    Option<u64>,
    Option<String>,
);

async fn order_snapshot(pool: &MySqlPool, order_id: u64) -> Result<SnapshotRow, sqlx::Error> {
    sqlx::query_as(
        r#"SELECT status, settlement_price, settlement_price_tick_id,
                  settlement_price_source, settlement_price_observed_at,
                  settlement_price_generation, settlement_price_version
           FROM seconds_contract_orders WHERE id = ?"#,
    )
    .bind(order_id)
    .fetch_one(pool)
    .await
}

#[test]
fn settlement_result_uses_direction_and_exact_prices() {
    assert_eq!(
        seconds_contract_settlement_result("up", &decimal("100"), &decimal("101")).unwrap(),
        "win"
    );
    assert_eq!(
        seconds_contract_settlement_result("down", &decimal("100"), &decimal("100")).unwrap(),
        "loss"
    );
}

#[tokio::test]
async fn latest_only_migration_preserves_completed_evidence_and_seeds_one_checkpoint()
-> Result<(), Box<dyn Error>> {
    let Ok(url) = std::env::var("LATEST_PRICE_MIGRATION_DATABASE_URL") else {
        return Ok(());
    };
    let parsed = url::Url::parse(&url)?;
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    assert!(parsed.path().ends_with("_test"));
    let pool = MySqlPoolOptions::new().connect(&url).await?;
    let mut previous = sqlx::migrate!();
    previous.migrations = previous
        .migrations
        .iter()
        .filter(|m| m.version < 148)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    previous.run(&pool).await?;
    let fixture = seed_fixture(&pool).await?;
    let now = Utc::now();
    let opened = seed_order(&pool, &fixture, now, "up").await?;
    let settled = seed_order(&pool, &fixture, now, "up").await?;
    for id in [opened, settled] {
        sqlx::query("INSERT INTO seconds_contract_price_evidence(order_id,entry_json) VALUES (?,JSON_OBJECT('price','100.123456789012345678'))")
            .bind(id).execute(&pool).await?;
    }
    sqlx::query(
        "UPDATE seconds_contract_orders SET status='settled',settlement_price=99 WHERE id=?",
    )
    .bind(settled)
    .execute(&pool)
    .await?;
    insert_tick(
        &pool,
        &fixture.pair_symbol,
        "1",
        "default",
        now - TimeDelta::seconds(2),
        1,
    )
    .await?;
    insert_tick(
        &pool,
        &fixture.pair_symbol,
        "2",
        "default",
        now - TimeDelta::seconds(1),
        1,
    )
    .await?;
    insert_tick(&pool, &fixture.pair_symbol, "999", "bitget", now, 1).await?;
    // 早期无策略身份的遗留记录不能被误当成有效生成器检查点。
    insert_tick(&pool, "INVALIDUSDT", "1", "strategy", now, 1).await?;
    sqlx::migrate!().run(&pool).await?;
    sqlx::migrate!().run(&pool).await?;
    let rows: Vec<(u64, String, String)> = sqlx::query_as("SELECT order_id,policy,CAST(JSON_UNQUOTE(JSON_EXTRACT(entry_json,'$.price')) AS CHAR) FROM seconds_contract_price_evidence ORDER BY order_id")
        .fetch_all(&pool).await?;
    assert_eq!(
        rows,
        vec![
            (
                opened,
                "redis_latest_v1".into(),
                "100.123456789012345678".into()
            ),
            (
                settled,
                "redis_window_v1".into(),
                "100.123456789012345678".into()
            )
        ]
    );
    let checkpoints: Vec<BigDecimal> =
        sqlx::query_scalar("SELECT price FROM market_generation_price_checkpoints")
            .fetch_all(&pool)
            .await?;
    assert_eq!(checkpoints, vec![decimal("2")]);
    assert_eq!(order_snapshot(&pool, settled).await?.1, Some(decimal("99")));
    let balance: BigDecimal = sqlx::query_scalar("SELECT SUM(available) FROM wallet_accounts")
        .fetch_one(&pool)
        .await?;
    assert_eq!(balance, decimal("80"));
    Ok(())
}

#[tokio::test]
async fn latest_pricing_and_retention_preserve_order_evidence() -> Result<(), Box<dyn Error>> {
    let _guard = TEST_LOCK.lock().await;
    let (Some(pool), Some(redis)) = (mysql_pool_or_skip().await?, cached_test_redis().await?)
    else {
        return Ok(());
    };
    let fixture = seed_fixture(&pool).await?;
    let expiry = Utc::now() - TimeDelta::seconds(60);
    let order = seed_order(&pool, &fixture, expiry, "up").await?;
    sqlx::query("INSERT INTO seconds_contract_price_evidence(order_id,entry_json) VALUES (?, JSON_OBJECT('price','100','source','bitget'))")
        .bind(order).execute(&pool).await?;
    use exchange_api::modules::market::{
        MarketDataProvider, MarketTickerCacheEntry, MarketTickerSnapshot, RedisMarketCache,
    };
    let cache = RedisMarketCache::new(redis.clone());
    for (at, price) in [
        (
            expiry + TimeDelta::milliseconds(10),
            "101.123456789012345678",
        ),
        (expiry + TimeDelta::seconds(10), "1"),
    ] {
        let snapshot = MarketTickerSnapshot::new(
            MarketDataProvider::Bitget,
            &fixture.pair_symbol,
            decimal(price),
            decimal("1"),
            at,
        )?;
        cache
            .save_ticker(MarketTickerCacheEntry::from_snapshot(&snapshot)?)
            .await?;
    }
    run_once_with_cached_prices(&pool, &redis, None, expiry + TimeDelta::seconds(4), 500).await?;
    assert_eq!(order_snapshot(&pool, order).await?.0, "settled");
    run_once_with_cached_prices(&pool, &redis, None, expiry + TimeDelta::seconds(30), 500).await?;
    let result = order_snapshot(&pool, order).await?;
    assert_eq!(result.0, "settled");
    assert_eq!(result.1, Some(decimal("1")));
    assert_eq!(result.3.as_deref(), Some("bitget"));
    use redis::AsyncCommands;
    let _: usize = redis
        .clone()
        .del(format!(
            "market:price-history:v1:{}",
            fixture.pair_symbol.replace('-', "")
        ))
        .await?;
    run_once_with_cached_prices(&pool, &redis, None, expiry + TimeDelta::hours(1), 500).await?;
    assert_eq!(order_snapshot(&pool, order).await?, result);
    let evidence: String = sqlx::query_scalar("SELECT CAST(JSON_UNQUOTE(JSON_EXTRACT(entry_json,'$.price')) AS CHAR) FROM seconds_contract_price_evidence WHERE order_id=?").bind(order).fetch_one(&pool).await?;
    assert_eq!(evidence, "100");

    // 即使证据来自上一版窗口策略，也按用户确认的最新价立即结算；不查历史。
    let immediate = seed_order(&pool, &fixture, expiry, "up").await?;
    sqlx::query("INSERT INTO seconds_contract_price_evidence(order_id,policy,entry_json) VALUES (?, 'redis_window_v1', JSON_OBJECT('price','100'))")
        .bind(immediate).execute(&pool).await?;
    run_once_with_cached_prices(&pool, &redis, None, expiry, 500).await?;
    let immediate_snapshot = order_snapshot(&pool, immediate).await?;
    assert_eq!(immediate_snapshot.0, "settled");
    assert_eq!(immediate_snapshot.1, Some(decimal("1")));

    let old = Utc::now() - TimeDelta::days(3);
    let remove = insert_tick(&pool, &fixture.pair_symbol, "90", "bitget", old, 1).await?;
    let linked = insert_tick(
        &pool,
        &fixture.pair_symbol,
        "91",
        "bitget",
        old + TimeDelta::seconds(1),
        1,
    )
    .await?;
    let pending_tick = insert_tick(
        &pool,
        &fixture.pair_symbol,
        "92",
        "bitget",
        old + TimeDelta::seconds(10),
        1,
    )
    .await?;
    let head = insert_tick(
        &pool,
        &fixture.pair_symbol,
        "93",
        "bitget",
        old + TimeDelta::seconds(20),
        1,
    )
    .await?;
    sqlx::query("UPDATE market_price_ticks SET ingested_at=? WHERE id IN (?,?,?,?)")
        .bind(old.naive_utc())
        .bind(remove)
        .bind(linked)
        .bind(pending_tick)
        .bind(head)
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE seconds_contract_orders SET settlement_price_tick_id=? WHERE id=?")
        .bind(linked)
        .bind(order)
        .execute(&pool)
        .await?;
    let pending = seed_order(&pool, &fixture, old + TimeDelta::seconds(10), "up").await?;
    let mut cursor = 0;
    loop {
        let (next, _) =
            exchange_api::workers::market_retention::prune_price_batch(&pool, cursor).await?;
        cursor = next;
        if next == 0 {
            break;
        }
    }
    for (id, expected) in [
        (remove, false),
        (linked, true),
        (pending_tick, true),
        (head, true),
    ] {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM market_price_ticks WHERE id=?)")
                .bind(id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(exists, expected, "tick {id}");
    }
    sqlx::query("UPDATE seconds_contract_orders SET status='settled' WHERE id=?")
        .bind(pending)
        .execute(&pool)
        .await?;
    Ok(())
}

#[tokio::test]
async fn timely_delayed_and_replay_use_the_same_event_price_snapshot() -> Result<(), Box<dyn Error>>
{
    let _guard = TEST_LOCK.lock().await;
    let Some(pool) = mysql_pool_or_skip().await? else {
        return Ok(());
    };
    sqlx::query("UPDATE seconds_contract_orders SET status = 'settled' WHERE idempotency_key LIKE 'event-price-%' AND status = 'opened'")
        .execute(&pool)
        .await?;
    let fixture = seed_fixture(&pool).await?;
    let expires_at = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
    let timely_order = seed_order(&pool, &fixture, expires_at, "up").await?;
    let delayed_order = seed_order(&pool, &fixture, expires_at, "up").await?;
    insert_tick(
        &pool,
        &fixture.pair_symbol,
        "105",
        "htx",
        expires_at + TimeDelta::milliseconds(100),
        8,
    )
    .await?;
    let expected_tick = insert_tick(
        &pool,
        &fixture.pair_symbol,
        "104",
        "bitget",
        expires_at + TimeDelta::milliseconds(100),
        7,
    )
    .await?;

    let timely = run_once_with_pool(&pool, expires_at + TimeDelta::seconds(5), 1).await?;
    assert_eq!(timely.settled, 1);
    let delayed = run_once_with_pool(&pool, expires_at + TimeDelta::minutes(5), 1).await?;
    assert_eq!(delayed.settled, 1);

    let timely_snapshot = order_snapshot(&pool, timely_order).await?;
    let delayed_snapshot = order_snapshot(&pool, delayed_order).await?;
    assert_eq!(timely_snapshot, delayed_snapshot);
    assert_eq!(timely_snapshot.0, "settled");
    assert_eq!(
        timely_snapshot.1.as_ref().unwrap().normalized(),
        decimal("104")
    );
    assert_eq!(timely_snapshot.2, Some(expected_tick));
    assert_eq!(timely_snapshot.3.as_deref(), Some("bitget"));
    assert_eq!(timely_snapshot.5, Some(7));

    let replay = run_once_with_pool(&pool, expires_at + TimeDelta::minutes(6), 100).await?;
    assert_eq!(replay.settled, 0);
    assert_eq!(order_snapshot(&pool, timely_order).await?, timely_snapshot);
    let (ledger_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM wallet_ledger WHERE ref_type = 'seconds_contract_order' AND ref_id IN (?, ?) AND change_type = 'seconds_contract_settle_win'",
    )
    .bind(timely_order.to_string())
    .bind(delayed_order.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(ledger_count, 2);
    let journal: (i64, BigDecimal) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(amount), 0) FROM platform_financial_journal WHERE context = 'seconds_contract' AND ref_id IN (?, ?)",
    )
    .bind(timely_order.to_string())
    .bind(delayed_order.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(journal, (6, decimal("0")));
    Ok(())
}

#[tokio::test]
async fn scan_before_window_close_does_not_postpone_first_eligible_settlement()
-> Result<(), Box<dyn Error>> {
    let _guard = TEST_LOCK.lock().await;
    let Some(pool) = mysql_pool_or_skip().await? else {
        return Ok(());
    };
    let fixture = seed_fixture(&pool).await?;
    let expires_at = Utc.with_ymd_and_hms(2020, 1, 1, 1, 0, 0).unwrap();
    let order_id = seed_order(&pool, &fixture, expires_at, "up").await?;
    let tick_id = insert_tick(&pool, &fixture.pair_symbol, "101", "bitget", expires_at, 1).await?;

    let early = run_once_with_pool(&pool, expires_at + TimeDelta::seconds(1), 1).await?;
    assert_eq!(early.scanned, 0);
    assert_eq!(order_snapshot(&pool, order_id).await?.0, "opened");

    let on_time = run_once_with_pool(&pool, expires_at + TimeDelta::seconds(5), 1).await?;
    assert_eq!(on_time.settled, 1);
    assert_eq!(order_snapshot(&pool, order_id).await?.2, Some(tick_id));
    Ok(())
}

#[tokio::test]
async fn missing_history_stays_pending_then_late_replay_uses_event_time_tick()
-> Result<(), Box<dyn Error>> {
    let _guard = TEST_LOCK.lock().await;
    let Some(pool) = mysql_pool_or_skip().await? else {
        return Ok(());
    };
    let fixture = seed_fixture(&pool).await?;
    let expires_at = Utc.with_ymd_and_hms(2020, 1, 2, 0, 0, 0).unwrap();
    let order_id = seed_order(&pool, &fixture, expires_at, "up").await?;
    let first_attempt = expires_at + TimeDelta::seconds(5);
    let pending = run_once_with_pool(&pool, first_attempt, 1).await?;
    assert_eq!(pending.skipped, 1);
    assert_eq!(pending.failed, 0);
    let pending_snapshot = order_snapshot(&pool, order_id).await?;
    assert_eq!(pending_snapshot.0, "opened");
    assert!(pending_snapshot.1.is_none());

    let tick_id = insert_tick(
        &pool,
        &fixture.pair_symbol,
        "103",
        "coinbase",
        expires_at + TimeDelta::seconds(1),
        9,
    )
    .await?;
    let delayed = run_once_with_pool(&pool, first_attempt + TimeDelta::seconds(60), 1).await?;
    assert_eq!(delayed.settled, 1);
    let snapshot = order_snapshot(&pool, order_id).await?;
    assert_eq!(snapshot.2, Some(tick_id));
    assert_eq!(snapshot.1.unwrap().normalized(), decimal("103"));
    Ok(())
}

#[tokio::test]
async fn settlement_window_is_left_closed_and_right_open() -> Result<(), Box<dyn Error>> {
    let _guard = TEST_LOCK.lock().await;
    let Some(pool) = mysql_pool_or_skip().await? else {
        return Ok(());
    };
    let fixture = seed_fixture(&pool).await?;
    let expires_at = Utc.with_ymd_and_hms(2020, 1, 3, 0, 0, 0).unwrap();
    let order_id = seed_order(&pool, &fixture, expires_at, "up").await?;
    insert_tick(
        &pool,
        &fixture.pair_symbol,
        "999",
        "bitget",
        expires_at - TimeDelta::microseconds(1),
        1,
    )
    .await?;
    insert_tick(
        &pool,
        &fixture.pair_symbol,
        "888",
        "bitget",
        expires_at + TimeDelta::seconds(5),
        1,
    )
    .await?;
    let pending = run_once_with_pool(&pool, expires_at + TimeDelta::seconds(5), 1).await?;
    assert_eq!(pending.skipped, 1);
    assert_eq!(order_snapshot(&pool, order_id).await?.0, "opened");

    let exact_tick =
        insert_tick(&pool, &fixture.pair_symbol, "101", "bitget", expires_at, 2).await?;
    let settled = run_once_with_pool(&pool, expires_at + TimeDelta::seconds(65), 1).await?;
    assert_eq!(settled.settled, 1);
    assert_eq!(order_snapshot(&pool, order_id).await?.2, Some(exact_tick));
    Ok(())
}

#[tokio::test]
async fn missing_snapshot_moves_once_to_manual_review_and_restart_does_not_mutate_wallet()
-> Result<(), Box<dyn Error>> {
    let _guard = TEST_LOCK.lock().await;
    let Some(pool) = mysql_pool_or_skip().await? else {
        return Ok(());
    };
    sqlx::query(
        "UPDATE seconds_contract_orders SET status = 'settled' WHERE idempotency_key LIKE 'event-price-%' AND status = 'opened'",
    )
    .execute(&pool)
    .await?;
    let fixture = seed_fixture(&pool).await?;
    let expires_at = Utc.with_ymd_and_hms(2030, 1, 4, 0, 0, 0).unwrap();
    let order_id = seed_order(&pool, &fixture, expires_at, "up").await?;
    let processing_time = expires_at + TimeDelta::seconds(15);

    let first = run_once_with_pool_and_max_wait(&pool, processing_time, 1, 10).await?;
    assert_eq!(first.scanned, 1);
    assert_eq!(first.settled, 0);
    assert_eq!(first.manual_review, 1);
    assert_eq!(first.skipped, 0);
    assert_eq!(first.failed, 0);

    type FailureRow = (
        String,
        Option<String>,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
        Option<BigDecimal>,
    );
    let failure: FailureRow = sqlx::query_as(
        r#"SELECT status, settlement_failure_code, settlement_failed_at,
                  settlement_window_start, settlement_window_end, settlement_price
           FROM seconds_contract_orders
           WHERE id = ?"#,
    )
    .bind(order_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(failure.0, "manual_review");
    assert_eq!(failure.1.as_deref(), Some("missing_settlement_snapshot"));
    assert_eq!(failure.2, Some(processing_time));
    assert_eq!(failure.3, Some(expires_at));
    assert_eq!(failure.4, Some(expires_at + TimeDelta::seconds(5)));
    assert!(failure.5.is_none());

    let exception: (i64, String, DateTime<Utc>, DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
        r#"SELECT COUNT(*), MAX(failure_code), MAX(detected_at),
                      MAX(window_start), MAX(window_end)
               FROM seconds_contract_settlement_exceptions
               WHERE order_id = ?"#,
    )
    .bind(order_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(exception.0, 1);
    assert_eq!(exception.1, "missing_settlement_snapshot");
    assert_eq!(exception.2, processing_time);
    assert_eq!(exception.3, expires_at);
    assert_eq!(exception.4, expires_at + TimeDelta::seconds(5));

    let wallet_before_restart: BigDecimal = sqlx::query_scalar(
        r#"SELECT wallets.available
           FROM seconds_contract_orders orders
           INNER JOIN wallet_accounts wallets
                   ON wallets.user_id = orders.user_id
                  AND wallets.asset_id = orders.stake_asset
           WHERE orders.id = ?"#,
    )
    .bind(order_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(wallet_before_restart.normalized(), decimal("40"));
    let ledger_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM wallet_ledger WHERE ref_type = 'seconds_contract_order' AND ref_id = ?",
    )
    .bind(order_id.to_string())
    .fetch_one(&pool)
    .await?;
    assert_eq!(ledger_count, 0);

    let replay =
        run_once_with_pool_and_max_wait(&pool, processing_time + TimeDelta::minutes(1), 100, 10)
            .await?;
    assert_eq!(replay.scanned, 0);
    assert_eq!(replay.manual_review, 0);
    let exception_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM seconds_contract_settlement_exceptions WHERE order_id = ?",
    )
    .bind(order_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(exception_count, 1);
    let wallet_after_restart: BigDecimal = sqlx::query_scalar(
        r#"SELECT wallets.available
           FROM seconds_contract_orders orders
           INNER JOIN wallet_accounts wallets
                   ON wallets.user_id = orders.user_id
                  AND wallets.asset_id = orders.stake_asset
           WHERE orders.id = ?"#,
    )
    .bind(order_id)
    .fetch_one(&pool)
    .await?;
    assert_eq!(wallet_after_restart, wallet_before_restart);
    Ok(())
}
