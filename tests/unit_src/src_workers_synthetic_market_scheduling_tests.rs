use super::*;

#[tokio::test]
async fn attempts_rotate_manual_and_default_candidates_without_faking_price_checkpoints() {
    let Ok(url) = std::env::var("MARKET_SCHEDULING_TEST_DATABASE_URL") else {
        return;
    };
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    // This test owns an isolated schema; a shared business database must not be supplied.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM trading_pairs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "use a fresh isolated scheduling schema");
    let mut pairs = Vec::new();
    for index in 0..4 {
        let base = sqlx::query(
            "INSERT INTO assets (symbol, name, precision_scale, status) VALUES (?, ?, 8, 'active')",
        )
        .bind(format!("FAIR{index}"))
        .bind(format!("Fair {index}"))
        .execute(&pool)
        .await
        .unwrap()
        .last_insert_id();
        let pair = sqlx::query("INSERT INTO trading_pairs (base_asset, quote_asset, symbol, price_precision, qty_precision, min_order_value, status, market_type) VALUES (?, ?, ?, 8, 8, 1, 'active', 'strategy')")
            .bind(base).bind(base).bind(format!("FAIR{index}USDT")).execute(&pool).await.unwrap().last_insert_id();
        pairs.push(pair);
        sqlx::query("INSERT INTO market_default_generators (pair_id,enabled,all_market_paused,initial_price,config_json,version,seed) VALUES (?,TRUE,FALSE,1,JSON_OBJECT(),1,'fair')")
            .bind(pair).execute(&pool).await.unwrap();
    }
    let first = default_runtime::load_default_pairs(&pool, Utc::now(), 2)
        .await
        .unwrap();
    assert_eq!(
        first.iter().map(|row| row.pair_id).collect::<Vec<_>>(),
        pairs[..2]
    );
    for row in first {
        default_runtime::record_pair_generation_attempt(&pool, row.pair_id)
            .await
            .unwrap();
    }
    let second = default_runtime::load_default_pairs(&pool, Utc::now(), 2)
        .await
        .unwrap();
    assert_eq!(
        second.iter().map(|row| row.pair_id).collect::<Vec<_>>(),
        pairs[2..]
    );
    for row in second {
        default_runtime::record_pair_generation_attempt(&pool, row.pair_id)
            .await
            .unwrap();
    }
    let third = default_runtime::load_default_pairs(&pool, Utc::now(), 2)
        .await
        .unwrap();
    assert_eq!(
        third.iter().map(|row| row.pair_id).collect::<Vec<_>>(),
        pairs[..2]
    );
    // No failed attempt creates fake ticks, runtime prices or K-line checkpoints.
    let runtime: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM market_pair_generation_runs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(runtime, 0);
    let now = Utc::now();
    for pair in &pairs {
        let strategy = sqlx::query("INSERT INTO market_strategies (pair_id,strategy_type,status,start_price,target_price,start_time,end_time,volatility,volume_min,volume_max) VALUES (?,'linear','active',1,1,?,?,0,0,0)")
            .bind(pair).bind((now-TimeDelta::hours(1)).naive_utc()).bind((now+TimeDelta::hours(1)).naive_utc())
            .execute(&pool).await.unwrap().last_insert_id();
        sqlx::query("INSERT INTO strategy_versions (strategy_id,version,effective_time,config_json,seed) VALUES (?,1,?,JSON_OBJECT(),'fair')")
            .bind(strategy).bind(now.naive_utc()).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO strategy_runs (strategy_id,active_version,run_status) VALUES (?,1,'running')")
            .bind(strategy).execute(&pool).await.unwrap();
    }
    let first = load_active_strategies(&pool, now, 2, "fair-test")
        .await
        .unwrap();
    assert_eq!(
        first.iter().map(|row| row.pair_id).collect::<Vec<_>>(),
        pairs[..2]
    );
    for row in first {
        default_runtime::record_pair_generation_attempt(&pool, row.pair_id)
            .await
            .unwrap();
    }
    let second = load_active_strategies(&pool, now, 2, "fair-test")
        .await
        .unwrap();
    assert_eq!(
        second.iter().map(|row| row.pair_id).collect::<Vec<_>>(),
        pairs[2..]
    );
    assert!(
        default_runtime::load_default_pairs(&pool, now, 100)
            .await
            .unwrap()
            .is_empty()
    );
    let checkpoints: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM strategy_runs WHERE last_tick_at IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(checkpoints, 0);
}
