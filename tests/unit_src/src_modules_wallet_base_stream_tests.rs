use super::*;
use crate::modules::wallet::{application::timed_deposits as app, presentation::timed_deposits::*};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
const KEY: &str = "0123456789abcdef0123456789abcdef";
const TOKEN: &str = "0x1111111111111111111111111111111111111111";
const ADDRESS: &str = "0x2222222222222222222222222222222222222222";
const FROM: &str = "0x3333333333333333333333333333333333333333";

async fn pool(variable: &str, database: &str) -> Option<Pool<MySql>> {
    let Ok(url) = std::env::var(variable) else {
        eprintln!("SKIPPED: isolated {variable} required");
        return None;
    };
    assert!(url.contains("127.0.0.1") && url.ends_with(database));
    Some(
        sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(16)
            .after_connect(|c, _| {
                Box::pin(async move {
                    sqlx::query("SET time_zone='+00:00'").execute(c).await?;
                    Ok(())
                })
            })
            .connect(&url)
            .await
            .unwrap(),
    )
}
async fn snapshot(pool: &Pool<MySql>) -> ScannerView {
    app::settings(pool)
        .await
        .unwrap()
        .scanners
        .into_iter()
        .find(|s| s.network == "base")
        .unwrap()
}
fn lane<'a>(scanner: &'a ScannerView, stream: &str) -> &'a ScannerStreamView {
    scanner.streams.iter().find(|s| s.stream == stream).unwrap()
}
fn order(symbol: &str, key: &str) -> CreateDepositOrder {
    CreateDepositOrder {
        network: "base".into(),
        asset_symbol: symbol.into(),
        amount: "1".parse().unwrap(),
        idempotency_key: key.into(),
    }
}
async fn clear_scoped_cooldown(pool: &Pool<MySql>) {
    sqlx::query("UPDATE wallet_deposit_scanner_stream_failures SET cooldown_until=NULL")
        .execute(pool)
        .await
        .unwrap();
}
async fn recover(pool: &Pool<MySql>, admin: u64, stream: Option<&str>, mode: &str) {
    app::recover_scanner(
        pool,
        admin,
        "base",
        RecoverScanner {
            stream: stream.map(str::to_owned),
            mode: mode.into(),
            expected_revision: snapshot(pool).await.revision,
            acknowledge_skip: mode == "skip",
            reason: "isolated stream verification".into(),
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn base_streams_isolate_rpc_admission_credit_recovery_and_quota() {
    let Some(pool) = pool(
        "DEPOSIT_STREAM_TEST_DATABASE_URL",
        "timed_deposits_test_streams",
    )
    .await
    else {
        return;
    };
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let role = sqlx::query(
        "INSERT INTO admin_roles(name,permissions) VALUES ('stream-fixture',JSON_ARRAY('*'))",
    )
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();
    let admin=sqlx::query("INSERT INTO admin_users(username,password_hash,role_id) VALUES ('stream-fixture','fixture',?)").bind(role).execute(&pool).await.unwrap().last_insert_id();
    let user = sqlx::query(
        "INSERT INTO users(email,password_hash) VALUES ('streams@example.test','fixture')",
    )
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();
    for (symbol, kind, token, decimals) in [("ETH", "native", "", 18), ("BUSDC", "token", TOKEN, 6)]
    {
        sqlx::query("INSERT INTO assets(symbol,name,precision_scale,min_deposit_amount,deposit_fee,deposit_enabled) VALUES (?,?,?,1,0,TRUE) ON DUPLICATE KEY UPDATE precision_scale=VALUES(precision_scale),min_deposit_amount=1,deposit_fee=0,deposit_enabled=TRUE").bind(symbol).bind(symbol).bind(decimals).execute(&pool).await.unwrap();
        app::save_asset(
            &pool,
            admin,
            SaveChainAsset {
                network: "base".into(),
                asset_symbol: symbol.into(),
                asset_kind: kind.into(),
                token_address: token.into(),
                decimals,
                enabled: true,
                expected_revision: 0,
                reason: "fixture".into(),
            },
        )
        .await
        .unwrap();
    }
    app::import_addresses(
        &pool,
        admin,
        ImportSharedAddresses {
            network: "base".into(),
            addresses: vec![ADDRESS.into()],
            reason: "fixture".into(),
        },
    )
    .await
    .unwrap();
    let server = MockServer::start().await;
    let tip = Arc::new(AtomicU64::new(101));
    let native_error = Arc::new(AtomicBool::new(true));
    let token_error = Arc::new(AtomicBool::new(false));
    let quota = Arc::new(AtomicBool::new(false));
    let delay = Arc::new(AtomicBool::new(false));
    let (t, n, e, q, d) = (
        tip.clone(),
        native_error.clone(),
        token_error.clone(),
        quota.clone(),
        delay.clone(),
    );
    let time = Utc::now().timestamp() - 1;
    Mock::given(matchers::method("POST")).respond_with(move |r:&wiremock::Request| {
        if q.load(Ordering::SeqCst) {return ResponseTemplate::new(429).insert_header("Retry-After","60")}
        let body:Value=serde_json::from_slice(&r.body).unwrap();
        let answer=|v:&Value| {
            let method=v["method"].as_str().unwrap();
            if (method=="eth_getTransactionByHash" && n.load(Ordering::SeqCst)) || (method=="eth_getLogs" && e.load(Ordering::SeqCst)) {
                return json!({"jsonrpc":"2.0","id":v["id"],"error":{"code":-32601,"message":"unavailable fixture method"}})
            }
            let result=match method {
                "eth_chainId"=>json!("0x2105"),
                "eth_call"=>json!("0x6"),
                "eth_getBlockByNumber"=> {
                    let height=if v["params"][0]=="latest" {t.load(Ordering::SeqCst)} else {u64::from_str_radix(v["params"][0].as_str().unwrap().trim_start_matches("0x"),16).unwrap()};
                    json!({"number":format!("0x{height:x}"),"hash":format!("b{height}"),"parentHash":format!("b{}",height-1),"timestamp":format!("0x{time:x}"),"transactions":if height==101 {json!(["native-payment"])}else{json!([])}})
                },
                "eth_getTransactionByHash"=>json!({"hash":"native-payment","blockHash":"b101","blockNumber":"0x65","transactionIndex":"0x0","from":FROM,"to":ADDRESS,"value":"0xde0b6b3a7640000"}),
                "eth_getTransactionReceipt"=>json!({"transactionHash":"native-payment","blockHash":"b101","blockNumber":"0x65","transactionIndex":"0x0","from":FROM,"to":ADDRESS,"status":"0x1"}),
                "eth_getLogs" if v["params"][0]["blockHash"]=="b101"=>json!([{"address":TOKEN,"blockHash":"b101","blockNumber":"0x65","transactionHash":"token-payment","logIndex":"0x0","removed":false,"topics":[format!("0x{TRANSFER_TOPIC}"),format!("0x{}{}","0".repeat(24),&FROM[2..]),format!("0x{}{}","0".repeat(24),&ADDRESS[2..])],"data":format!("0x{:064x}",1_000_000u64)}]),
                "eth_getLogs"=>json!([]),
                _=>panic!("unexpected method {method}")
            };
            json!({"jsonrpc":"2.0","id":v["id"],"result":result})
        };
        let response=ResponseTemplate::new(200).set_body_json(if let Some(batch)=body.as_array(){Value::Array(batch.iter().map(answer).collect())}else{answer(&body)});
        if d.load(Ordering::SeqCst) {response.set_delay(Duration::from_millis(350))}else{response}
    }).mount(&server).await;
    app::save_scanner(&pool,admin,"base",KEY,serde_json::from_value(json!({"enabled":true,"confirmations":1,"expected_revision":snapshot(&pool).await.revision,"reason":"fixture","providers":[{"name":"Fixture","priority":0,"enabled":true,"endpoint":server.uri(),"request_interval_ms":100,"keys":[{"name":"Public","priority":0,"enabled":true,"api_key":null,"clear_api_key":false,"request_interval_ms":100}]}]})).unwrap()).await.unwrap();
    sqlx::query("UPDATE wallet_deposit_scanner_streams SET cursor_json=?,last_scan_at=CURRENT_TIMESTAMP(6),last_error=NULL WHERE network='base'").bind(Json(json!({"height":100,"hash_height":100,"hash":"b100"}))).execute(&pool).await.unwrap();
    let usdc = app::create_order(&pool, user, order("BUSDC", "usdc"))
        .await
        .unwrap();
    let eth = app::create_order(&pool, user, order("ETH", "eth"))
        .await
        .unwrap();
    sqlx::query("UPDATE wallet_deposit_orders SET created_at=DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 60 SECOND)").execute(&pool).await.unwrap();
    run_once(&pool, Some(KEY)).await.unwrap();
    let state = snapshot(&pool).await;
    assert_eq!(lane(&state, "native").cursor_height, Some(100));
    assert!(
        lane(&state, "native")
            .last_error
            .as_ref()
            .unwrap()
            .contains("eth_getTransactionByHash")
    );
    assert_eq!(lane(&state, "token").cursor_height, Some(101));
    assert!(lane(&state, "token").scan_fresh && lane(&state, "token").last_error.is_none());
    assert_eq!(
        app::get_order(&pool, Some(user), usdc.id)
            .await
            .unwrap()
            .status,
        "credited"
    );
    assert_eq!(
        app::get_order(&pool, Some(user), eth.id)
            .await
            .unwrap()
            .status,
        "pending"
    );
    let options = app::options(&pool).await.unwrap();
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].asset_symbol, "BUSDC");
    let mut next = order("BUSDC", "usdc-next");
    next.amount = "2".parse().unwrap();
    assert!(app::create_order(&pool, user, next).await.is_ok());
    assert!(
        app::create_order(&pool, user, order("ETH", "eth-rejected"))
            .await
            .is_err()
    );
    assert!(
        state.providers[0].cooldown_until.is_none(),
        "native method failure must not globally cool RPC"
    );

    // Reverse the failure: native now completes, while token remains at its old checkpoint.
    native_error.store(false, Ordering::SeqCst);
    token_error.store(true, Ordering::SeqCst);
    clear_scoped_cooldown(&pool).await;
    tip.store(102, Ordering::SeqCst);
    run_once(&pool, Some(KEY)).await.unwrap();
    let state = snapshot(&pool).await;
    assert_eq!(lane(&state, "native").cursor_height, Some(102));
    assert!(lane(&state, "native").last_error.is_none());
    assert_eq!(lane(&state, "token").cursor_height, Some(101));
    assert!(lane(&state, "token").last_error.is_some());
    assert_eq!(
        app::get_order(&pool, Some(user), eth.id)
            .await
            .unwrap()
            .status,
        "credited"
    );
    assert_eq!(app::options(&pool).await.unwrap()[0].asset_symbol, "ETH");
    token_error.store(false, Ordering::SeqCst);
    clear_scoped_cooldown(&pool).await;
    run_once(&pool, Some(KEY)).await.unwrap();

    // Token-only recovery does not rewrite native history or close native admission.
    tip.store(110, Ordering::SeqCst);
    recover(&pool, admin, Some("token"), "backfill").await;
    let queued = snapshot(&pool).await;
    assert!(lane(&queued, "native").scan_fresh);
    assert!(lane(&queued, "native").recovery_mode.is_none());
    run_once(&pool, Some(KEY)).await.unwrap();
    let jumped = snapshot(&pool).await;
    assert_eq!(lane(&jumped, "token").backfill_cursor_height, Some(102));
    assert_eq!(lane(&jumped, "token").backfill_target_height, Some(110));
    assert!(lane(&jumped, "native").backfill_cursor_height.is_none());
    run_once(&pool, Some(KEY)).await.unwrap();
    run_once(&pool, Some(KEY)).await.unwrap();
    assert!(
        lane(&snapshot(&pool).await, "token")
            .backfill_cursor_height
            .is_none()
    );
    // Replay after a crash retains existing financial identities.
    sqlx::query("UPDATE wallet_deposit_scanner_streams SET backfill_cursor_json=?,backfill_target_json=cursor_json,backfill_next_at=NULL WHERE stream='token'").bind(Json(json!({"height":100,"hash_height":100,"hash":"b100"}))).execute(&pool).await.unwrap();
    run_once(&pool, Some(KEY)).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM wallet_ledger")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );

    // A rejected stale request cannot change either cursor or queued state.
    let revision = snapshot(&pool).await.revision;
    assert!(
        app::recover_scanner(
            &pool,
            admin,
            "base",
            RecoverScanner {
                stream: Some("token".into()),
                mode: "skip".into(),
                expected_revision: revision - 1,
                acknowledge_skip: true,
                reason: "stale request".into()
            }
        )
        .await
        .is_err()
    );
    // Legacy payload explicitly targets both streams; skip clears each backlog independently.
    tip.store(120, Ordering::SeqCst);
    recover(&pool, admin, None, "skip").await;
    run_once(&pool, Some(KEY)).await.unwrap();
    let skipped = snapshot(&pool).await;
    assert!(skipped.streams.iter().all(|s| s.cursor_height == Some(120)
        && s.recovery_mode.is_none()
        && s.backfill_cursor_height.is_none()
        && !s.scan_fresh));
    run_once(&pool, Some(KEY)).await.unwrap();

    // Shared HTTP quota is still respected; a stream success must not clear sibling 429 state.
    quota.store(true, Ordering::SeqCst);
    tip.store(121, Ordering::SeqCst);
    run_once(&pool, Some(KEY)).await.unwrap();
    let limited = snapshot(&pool).await;
    assert!(limited.providers[0].keys[0].cooldown_until.is_some());
    assert!(limited.streams.iter().all(|s| s.cursor_height == Some(120)));
    quota.store(false, Ordering::SeqCst);
    sqlx::query("UPDATE wallet_deposit_scanner_keys SET cooldown_until=NULL")
        .execute(&pool)
        .await
        .unwrap();
    // A concurrent configuration save fences both delayed workers and invalidates readiness.
    delay.store(true, Ordering::SeqCst);
    let worker_pool = pool.clone();
    let worker = tokio::spawn(async move { run_once(&worker_pool, Some(KEY)).await.unwrap() });
    tokio::time::sleep(Duration::from_millis(120)).await;
    app::save_scanner(
        &pool,
        admin,
        "base",
        KEY,
        SaveScanner {
            providers: None,
            enabled: true,
            endpoint: None,
            api_key: None,
            confirmations: 1,
            expected_revision: snapshot(&pool).await.revision,
            reason: "concurrent save".into(),
        },
    )
    .await
    .unwrap();
    worker.await.unwrap();
    let fenced = snapshot(&pool).await;
    assert!(
        fenced
            .streams
            .iter()
            .all(|s| s.cursor_height == Some(120) && !s.scan_fresh)
    );
    delay.store(false, Ordering::SeqCst);
    run_once(&pool, Some(KEY)).await.unwrap();
    assert!(
        snapshot(&pool)
            .await
            .streams
            .iter()
            .all(|s| s.cursor_height == Some(121) && s.scan_fresh)
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM wallet_ledger")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    // A live native lease does not prevent repeated token rounds from progressing.
    sqlx::query("UPDATE wallet_deposit_scanner_streams SET lease_token='slow-native',lease_until=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 60 SECOND) WHERE stream='native'").execute(&pool).await.unwrap();
    tip.store(122, Ordering::SeqCst);
    run_group(&pool, Some(KEY), Some("token")).await.unwrap();
    tip.store(123, Ordering::SeqCst);
    run_group(&pool, Some(KEY), Some("token")).await.unwrap();
    let independently_scheduled = snapshot(&pool).await;
    assert_eq!(
        lane(&independently_scheduled, "native").cursor_height,
        Some(121)
    );
    assert_eq!(
        lane(&independently_scheduled, "token").cursor_height,
        Some(123)
    );
    let receipt = ChainReceipt {
        network: "base".into(),
        tx_hash: "stale-work".into(),
        event_index: 0,
        asset_kind: "token".into(),
        token_address: TOKEN.into(),
        address: ADDRESS.into(),
        amount: "1".parse().unwrap(),
        block_height: 123,
        block_hash: "b123".into(),
        block_time: Utc::now(),
        confirmations: 1,
    };
    assert!(
        settlement::observe_stream(
            &pool,
            &receipt,
            independently_scheduled.revision,
            "token",
            "expired-worker"
        )
        .await
        .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM wallet_deposit_receipts WHERE tx_hash='stale-work'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert!(
        run_group(&pool, Some(KEY), Some("unsupported"))
            .await
            .is_err()
    );
    pool.close().await;
}

#[tokio::test]
async fn base_stream_migration_preserves_all_checkpoints_and_fences_workers() {
    let Some(pool) = pool(
        "DEPOSIT_STREAM_MIGRATION_DATABASE_URL",
        "timed_deposits_test_stream_migration",
    )
    .await
    else {
        return;
    };
    let mut old = sqlx::migrate!("./migrations");
    old.migrations =
        std::borrow::Cow::Owned(old.iter().filter(|m| m.version < 165).cloned().collect());
    old.run(&pool).await.unwrap();
    sqlx::query("UPDATE wallet_deposit_scanners SET revision=77,cursor_json=?,backfill_cursor_json=?,backfill_target_json=?,backfill_blocked=TRUE,backfill_last_error='history blocked',recovery_request_json=?,last_scan_at=CURRENT_TIMESTAMP(6),last_error='old error',lease_token='old-worker',lease_until=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 60 SECOND) WHERE network='base'")
        .bind(Json(json!({"height":100,"hash_height":100,"hash":"b100"}))).bind(Json(json!({"height":50,"hash_height":50,"hash":"b50"}))).bind(Json(json!({"height":100,"hash_height":100,"hash":"b100"}))).bind(Json(json!({"mode":"backfill","admin_id":1,"reason":"pending"}))).execute(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let state = snapshot(&pool).await;
    assert_eq!(state.revision, 78);
    assert_eq!(state.streams.len(), 2);
    for s in &state.streams {
        assert_eq!(s.cursor_height, Some(100));
        assert_eq!(s.backfill_cursor_height, Some(50));
        assert_eq!(s.backfill_target_height, Some(100));
        assert!(s.backfill_blocked);
        assert_eq!(s.backfill_last_error.as_deref(), Some("history blocked"));
        assert_eq!(s.recovery_mode.as_deref(), Some("backfill"));
        assert!(!s.scan_fresh);
        assert_eq!(s.last_error.as_deref(), Some("old error"));
    }
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT COUNT(*) FROM wallet_deposit_scanner_streams WHERE lease_token IS NOT NULL OR lease_until IS NOT NULL").fetch_one(&pool).await.unwrap(),0);
    assert_eq!(
        sqlx::query_scalar::<_, u64>(
            "SELECT revision FROM wallet_deposit_scanners WHERE network='eth'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    pool.close().await;
}
