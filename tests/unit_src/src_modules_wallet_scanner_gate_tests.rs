use super::*;
use std::{sync::Arc, time::Instant};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

const KEY: &str = "0123456789abcdef0123456789abcdef";

async fn fixture() -> Option<(Pool<MySql>, u64, u64)> {
    let Ok(url) = std::env::var("DEPOSIT_GATE_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED: isolated DEPOSIT_GATE_TEST_DATABASE_URL required");
        return None;
    };
    assert!(url.contains("127.0.0.1") && url.ends_with("timed_deposits_test_gate"));
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(16)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("SET time_zone='+00:00'")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::query("UPDATE wallet_deposit_scanners SET revision=1 WHERE network='base'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE wallet_deposit_scanner_streams SET lease_token=CONCAT('gate-',stream),lease_until=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 10 MINUTE) WHERE network='base'")
        .execute(&pool).await.unwrap();
    let provider=sqlx::query("INSERT INTO wallet_deposit_scanner_providers(network,name,endpoint_encrypted,request_interval_ms) VALUES ('base','gate-fixture','fixture',500)")
        .execute(&pool).await.unwrap().last_insert_id();
    let key=sqlx::query("INSERT INTO wallet_deposit_scanner_keys(provider_id,name,request_interval_ms) VALUES (?,'gate-fixture',500)")
        .bind(provider).execute(&pool).await.unwrap().last_insert_id();
    Some((pool, provider, key))
}

fn gate(pool: &Pool<MySql>, provider: u64, key: u64, stream: &str, interval: u32) -> Gate {
    Gate {
        stream: stream.into(),
        pool: pool.clone(),
        network: "base".into(),
        revision: 1,
        token: format!("gate-{stream}"),
        provider_id: provider,
        key_id: key,
        provider_interval: interval,
        key_interval: interval,
        rpc_batch_limit: MAX_RPC_BATCH_CALLS,
        retryable: AtomicBool::new(false),
        channel_failed: AtomicBool::new(false),
        credential_configured: false,
    }
}

#[tokio::test]
async fn scanner_gate_rechecks_delayed_admission_and_cooling_before_send() {
    let Some((pool, provider, key)) = fixture().await else {
        return;
    };
    let native = Arc::new(gate(&pool, provider, key, "native", 500));
    let token = gate(&pool, provider, key, "token", 500);
    sqlx::query("UPDATE wallet_deposit_scanner_providers SET next_request_at=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 300000 MICROSECOND) WHERE id=?")
        .bind(provider).execute(&pool).await.unwrap();
    let mut pending = Box::pin(native.acquire());
    tokio::select! {
        result = &mut pending => panic!("request should still be waiting: {result:?}"),
        _ = tokio::time::sleep(Duration::from_millis(100)) => {},
    }
    // A sibling response extends the shared deadline while this call is asleep.
    token.completed().await.unwrap();
    let completed = Instant::now();
    pending.await.unwrap();
    assert!(
        completed.elapsed() >= Duration::from_millis(450),
        "a sleeping request ignored the extended response deadline: {:?}",
        completed.elapsed()
    );

    // Simulate scheduler delay past multiple reserved slots. Admission must still be spaced.
    sqlx::query("UPDATE wallet_deposit_scanner_providers SET next_request_at=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 100000 MICROSECOND) WHERE id=?")
        .bind(provider).execute(&pool).await.unwrap();
    sqlx::query("UPDATE wallet_deposit_scanner_keys SET next_request_at=NULL WHERE id=?")
        .bind(key)
        .execute(&pool)
        .await
        .unwrap();
    let mut requests = Vec::new();
    for index in 0..4 {
        let gate = gate(
            &pool,
            provider,
            key,
            if index % 2 == 0 { "native" } else { "token" },
            100,
        );
        requests.push(tokio::spawn(async move {
            gate.acquire().await.unwrap();
            Instant::now()
        }));
    }
    tokio::time::sleep(Duration::from_millis(40)).await;
    std::thread::sleep(Duration::from_millis(650));
    let mut sent = Vec::new();
    for request in requests {
        sent.push(request.await.unwrap());
    }
    sent.sort();
    assert!(
        sent.windows(2)
            .all(|pair| pair[1].duration_since(pair[0]) >= Duration::from_millis(85)),
        "expired reservations collapsed into a burst: {sent:?}"
    );

    // A quota failure while waiting applies to both Base streams and must prevent admission.
    sqlx::query("UPDATE wallet_deposit_scanner_providers SET next_request_at=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 300000 MICROSECOND) WHERE id=?")
        .bind(provider).execute(&pool).await.unwrap();
    let mut pending = Box::pin(native.acquire());
    tokio::select! {
        result = &mut pending => panic!("request should still be waiting: {result:?}"),
        _ = tokio::time::sleep(Duration::from_millis(50)) => {},
    }
    token.failed("fixture HTTP 429", false, 120).await.unwrap();
    assert!(pending.await.is_err());
    assert!(native.retryable.load(Ordering::Relaxed));
    assert!(token.acquire().await.is_err());
    adaptive_base_scans_and_configuration(&pool, provider, key).await;
    fence_waiting_requests(&pool, provider, key).await;
    pool.close().await;
}

async fn batch_limit(pool: &Pool<MySql>, key: u64) -> u32 {
    sqlx::query_scalar("SELECT rpc_batch_limit FROM wallet_deposit_scanner_keys WHERE id=?")
        .bind(key)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn expire_cooldown(pool: &Pool<MySql>, key: u64) {
    sqlx::query("UPDATE wallet_deposit_scanner_keys SET cooldown_until=DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 1 SECOND) WHERE id=?")
        .bind(key).execute(pool).await.unwrap();
}

async fn adaptive_base_scans_and_configuration(pool: &Pool<MySql>, provider: u64, key: u64) {
    let server = MockServer::start().await;
    let batch_sizes = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sizes = batch_sizes.clone();
    let time = Utc::now().timestamp() - 1;
    Mock::given(matchers::method("POST")).respond_with(move |request: &wiremock::Request| {
        let body: Value = serde_json::from_slice(&request.body).unwrap();
        if let Some(batch) = body.as_array() {
            sizes.lock().unwrap().push(batch.len());
            if batch.len() > 12 {
                return ResponseTemplate::new(429).insert_header("Retry-After", "120").set_body_string("private upstream quota detail");
            }
        }
        let answer = |request: &Value| {
            let result = match request["method"].as_str().unwrap() {
                "eth_chainId" => json!("0x2105"),
                "eth_getBlockByNumber" => {
                    assert_eq!(request["params"][1], false);
                    let height = if request["params"][0] == "latest" {105} else {
                        u64::from_str_radix(request["params"][0].as_str().unwrap().trim_start_matches("0x"), 16).unwrap()
                    };
                    json!({"number":format!("0x{height:x}"),"hash":format!("b{height}"),"parentHash":format!("b{}",height-1),"timestamp":format!("0x{time:x}"),"transactions":(0..49).map(|i| format!("t{height}-{i}")).collect::<Vec<_>>()})
                },
                "eth_getTransactionByHash" => {
                    let hash = request["params"][0].as_str().unwrap();
                    let (height,index) = hash.strip_prefix('t').unwrap().split_once('-').unwrap();
                    let (height,index) = (height.parse::<u64>().unwrap(),index.parse::<u64>().unwrap());
                    json!({"hash":hash,"blockHash":format!("b{height}"),"blockNumber":format!("0x{height:x}"),"transactionIndex":format!("0x{index:x}"),"from":"0x1111111111111111111111111111111111111111","to":"0x2222222222222222222222222222222222222222","value":"0x0"})
                },
                method => panic!("unexpected method {method}"),
            };
            json!({"jsonrpc":"2.0","id":request["id"],"result":result})
        };
        ResponseTemplate::new(200).set_body_json(if let Some(batch) = body.as_array() {
            Value::Array(batch.iter().rev().map(answer).collect())
        } else {answer(&body)})
    }).mount(&server).await;

    let encrypted = crate::infra::secrets::encrypt_secret_field(KEY, Some(&server.uri()), None)
        .unwrap()
        .unwrap();
    sqlx::query("UPDATE wallet_deposit_scanner_providers SET enabled=(id=?) WHERE network='base'")
        .bind(provider)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE wallet_deposit_scanner_providers SET endpoint_encrypted=?,request_interval_ms=100,next_request_at=NULL WHERE id=?")
        .bind(encrypted).bind(provider).execute(pool).await.unwrap();
    sqlx::query("UPDATE wallet_deposit_scanner_keys SET request_interval_ms=100,next_request_at=NULL,cooldown_until=NULL,last_error=NULL,failure_count=0 WHERE id=?")
        .bind(key).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO assets(symbol,name,precision_scale) VALUES ('GATEETH','Gate ETH',18) ON DUPLICATE KEY UPDATE name=VALUES(name)")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO wallet_deposit_chain_assets(network,asset_id,asset_symbol,asset_kind,token_address,identity_key,decimals,enabled) SELECT 'base',id,'GATEETH','native','',SHA2('native',256),18,TRUE FROM assets WHERE symbol='GATEETH' ON DUPLICATE KEY UPDATE enabled=TRUE")
        .execute(pool).await.unwrap();
    let old_cursor = json!({"height":100,"hash_height":100,"hash":"b100"});
    sqlx::query("UPDATE wallet_deposit_scanner_streams SET cursor_json=? WHERE network='base' AND stream='native'")
        .bind(Json(old_cursor.clone())).execute(pool).await.unwrap();
    let mut config = sqlx::query_as::<_, ScannerConfig>(&format!(
        "{} WHERE p.network='base' AND p.stream='native'",
        progress::select("native")
    ))
    .fetch_one(pool)
    .await
    .unwrap();
    config.confirmations = 1;
    let failure = scan_available(
        pool,
        Some(KEY),
        &config,
        "gate-native",
        recovery::Work::Live,
    )
    .await;
    assert!(failure.is_err());
    assert_eq!(batch_limit(pool, key).await, 12);
    assert_eq!(*batch_sizes.lock().unwrap(), vec![25]);
    let cursor: Json<Value> = sqlx::query_scalar("SELECT cursor_json FROM wallet_deposit_scanner_streams WHERE network='base' AND stream='native'")
        .fetch_one(pool).await.unwrap();
    assert_eq!(cursor.0, old_cursor, "429 must preserve the checkpoint");
    let remaining: i64 = sqlx::query_scalar("SELECT TIMESTAMPDIFF(SECOND,CURRENT_TIMESTAMP(6),cooldown_until) FROM wallet_deposit_scanner_keys WHERE id=?")
        .bind(key).fetch_one(pool).await.unwrap();
    assert!(remaining >= 115);
    assert!(
        gate(pool, provider, key, "token", 100)
            .acquire()
            .await
            .is_err()
    );

    // A late sibling success and duplicated 25-call failures preserve both the cooldown and cap.
    let native = gate(pool, provider, key, "native", 100);
    native.succeeded().await.unwrap();
    assert!(native.acquire().await.is_err());
    native
        .throttled("duplicate batch 429", 120, Some(25))
        .await
        .unwrap();
    assert_eq!(batch_limit(pool, key).await, 12);

    expire_cooldown(pool, key).await;
    let result = scan_available(
        pool,
        Some(KEY),
        &config,
        "gate-native",
        recovery::Work::Live,
    )
    .await;
    assert!(result.is_ok(), "retry with smaller batches must complete");
    let (next, count, caught_up) = result.unwrap_or_else(|error| panic!("{}", error.error));
    assert_eq!((next.height, count, caught_up), (102, 0, false));
    assert_eq!(batch_limit(pool, key).await, 12);
    let sizes = batch_sizes.lock().unwrap().clone();
    assert!(sizes[1..].iter().all(|size| *size <= 12));
    assert_eq!(
        sizes[1..].iter().sum::<usize>(),
        98,
        "all transactions in both blocks must be checked"
    );
    config.cursor_json = Some(Json(next));
    let second = scan_available(
        pool,
        Some(KEY),
        &config,
        "gate-native",
        recovery::Work::Live,
    )
    .await
    .unwrap_or_else(|error| panic!("{}", error.error));
    assert_eq!(
        second.0.height, 104,
        "a fresh Node must retain the learned cap"
    );
    assert_eq!(batch_limit(pool, key).await, 12);

    // A scalar RPC 429 does not infer a new batch size. Other credentials retain their budget.
    let backup=sqlx::query("INSERT INTO wallet_deposit_scanner_keys(provider_id,name,enabled) VALUES (?,'unused-backup',FALSE)")
        .bind(provider).execute(pool).await.unwrap().last_insert_id();
    native.throttled("scalar RPC 429", 60, None).await.unwrap();
    assert_eq!(batch_limit(pool, key).await, 12);
    for (size, expected) in [(12, 6), (6, 3), (3, 1), (1, 1)] {
        native
            .throttled("smaller batch 429", 60, Some(size))
            .await
            .unwrap();
        assert_eq!(batch_limit(pool, key).await, expected);
    }
    assert_eq!(batch_limit(pool, backup).await, 25);

    configuration_preserves_or_resets_cap(pool, provider, key, &server.uri()).await;
}

async fn configuration_preserves_or_resets_cap(
    pool: &Pool<MySql>,
    provider: u64,
    key: u64,
    endpoint: &str,
) {
    use crate::modules::wallet::presentation::timed_deposits::SaveScannerProvider;
    async fn save(pool: &Pool<MySql>, draft: SaveScannerProvider) {
        let mut tx = pool.begin().await.unwrap();
        super::super::super::lock_network(&mut tx, "base")
            .await
            .unwrap();
        super::super::super::providers::save(&mut tx, "base", KEY, &[draft])
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    let mut draft = json!({"id":provider,"name":"fixture","enabled":true,"priority":0,"endpoint":null,"request_interval_ms":100,"keys":[{"id":key,"name":"fixture","enabled":true,"priority":0,"request_interval_ms":100,"api_key":null,"clear_api_key":false}]});
    let cool: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT cooldown_until FROM wallet_deposit_scanner_keys WHERE id=?")
            .bind(key)
            .fetch_one(pool)
            .await
            .unwrap();
    save(pool, serde_json::from_value(draft.clone()).unwrap()).await;
    assert_eq!(batch_limit(pool, key).await, 1);
    let after: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT cooldown_until FROM wallet_deposit_scanner_keys WHERE id=?")
            .bind(key)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(cool, after);
    draft["keys"][0]["api_key"] = json!("replacement-fixture-key");
    save(pool, serde_json::from_value(draft.clone()).unwrap()).await;
    assert_eq!(batch_limit(pool, key).await, 25);
    sqlx::query("UPDATE wallet_deposit_scanner_keys SET rpc_batch_limit=3 WHERE id=?")
        .bind(key)
        .execute(pool)
        .await
        .unwrap();
    draft["keys"][0]["api_key"] = Value::Null;
    draft["endpoint"] = json!(format!("{endpoint}/replacement"));
    save(pool, serde_json::from_value(draft).unwrap()).await;
    assert_eq!(batch_limit(pool, key).await, 25);

    // Legacy single-provider writes obey the same blank-versus-replacement distinction.
    sqlx::query("UPDATE wallet_deposit_scanner_keys SET rpc_batch_limit=2 WHERE id=?")
        .bind(key)
        .execute(pool)
        .await
        .unwrap();
    let encrypted = crate::infra::secrets::encrypt_secret_field(KEY, Some(endpoint), None)
        .unwrap()
        .unwrap();
    let mut legacy: crate::modules::wallet::presentation::timed_deposits::SaveScanner =
        serde_json::from_value(
            json!({"enabled":true,"confirmations":1,"expected_revision":1,"reason":"fixture"}),
        )
        .unwrap();
    for replacement in [false, true] {
        if replacement {
            legacy.endpoint = Some(endpoint.into());
        }
        let mut tx = pool.begin().await.unwrap();
        super::super::super::lock_network(&mut tx, "base")
            .await
            .unwrap();
        super::super::super::providers::save_legacy(
            &mut tx,
            "base",
            &legacy,
            Some(&encrypted),
            None,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        assert_eq!(
            batch_limit(pool, key).await,
            if replacement { 25 } else { 2 }
        );
    }
}

async fn fence_waiting_requests(pool: &Pool<MySql>, provider: u64, key: u64) {
    let native = gate(pool, provider, key, "native", 100);
    sqlx::query("UPDATE wallet_deposit_scanner_providers SET next_request_at=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 250000 MICROSECOND) WHERE id=?")
        .bind(provider).execute(pool).await.unwrap();
    let before: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT next_request_at FROM wallet_deposit_scanner_providers WHERE id=?",
    )
    .bind(provider)
    .fetch_one(pool)
    .await
    .unwrap();
    let mut cancelled = Box::pin(native.acquire());
    tokio::select! {
        result=&mut cancelled=>panic!("request should wait: {result:?}"),
        _=tokio::time::sleep(Duration::from_millis(25))=>{},
    }
    drop(cancelled);
    let after: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT next_request_at FROM wallet_deposit_scanner_providers WHERE id=?",
    )
    .bind(provider)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(
        before, after,
        "a cancelled waiter must not reserve future request slots"
    );
    for (table, id) in [
        ("wallet_deposit_scanner_providers", provider),
        ("wallet_deposit_scanner_keys", key),
    ] {
        sqlx::query(&format!("UPDATE {table} SET enabled=FALSE WHERE id=?"))
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
        assert!(native.acquire().await.is_err());
        assert!(native.retryable.load(Ordering::Relaxed));
        sqlx::query(&format!("UPDATE {table} SET enabled=TRUE WHERE id=?"))
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
    }
    native
        .failed_scoped("native-only failure", 60)
        .await
        .unwrap();
    assert!(native.acquire().await.is_err());
    gate(pool, provider, key, "token", 100)
        .acquire()
        .await
        .unwrap();
    sqlx::query("DELETE FROM wallet_deposit_scanner_stream_failures WHERE provider_id=?")
        .bind(provider)
        .execute(pool)
        .await
        .unwrap();
    for invalidate in [
        "UPDATE wallet_deposit_scanner_streams SET lease_until=DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 1 SECOND) WHERE network='base' AND stream='native'",
        "UPDATE wallet_deposit_scanners SET revision=2 WHERE network='base'",
    ] {
        sqlx::query("UPDATE wallet_deposit_scanner_providers SET next_request_at=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 150000 MICROSECOND) WHERE id=?")
            .bind(provider).execute(pool).await.unwrap();
        let mut pending = Box::pin(native.acquire());
        tokio::select! {
            result=&mut pending=>panic!("request should wait: {result:?}"),
            _=tokio::time::sleep(Duration::from_millis(25))=>{},
        }
        sqlx::query(invalidate).execute(pool).await.unwrap();
        assert!(pending.await.is_err());
        sqlx::query("UPDATE wallet_deposit_scanner_streams SET lease_until=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 10 MINUTE) WHERE network='base' AND stream='native'")
            .execute(pool).await.unwrap();
    }
}

#[tokio::test]
async fn rpc_batch_limit_upgrade_preserves_existing_cooldowns_and_progress() {
    let Ok(url) = std::env::var("DEPOSIT_BATCH_UPGRADE_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED: isolated DEPOSIT_BATCH_UPGRADE_TEST_DATABASE_URL required");
        return;
    };
    assert!(url.contains("127.0.0.1") && url.ends_with("timed_deposits_test_gate_upgrade"));
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .connect(&url)
        .await
        .unwrap();
    let mut old = sqlx::migrate!("./migrations");
    old.migrations = std::borrow::Cow::Owned(
        old.iter()
            .filter(|migration| migration.version < 167)
            .cloned()
            .collect(),
    );
    old.run(&pool).await.unwrap();
    let provider = sqlx::query("INSERT INTO wallet_deposit_scanner_providers(network,name,endpoint_encrypted,cooldown_until,last_error,failure_count) VALUES ('base','upgrade','fixture',DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 1 DAY),'provider error',3)")
        .execute(&pool).await.unwrap().last_insert_id();
    let key = sqlx::query("INSERT INTO wallet_deposit_scanner_keys(provider_id,name,api_key_encrypted,next_request_at,cooldown_until,last_error,failure_count) VALUES (?,'upgrade','fixture-secret',DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 1 HOUR),DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 1 DAY),'HTTP 429',4)")
        .bind(provider).execute(&pool).await.unwrap().last_insert_id();
    sqlx::query("UPDATE wallet_deposit_scanner_streams SET cursor_json=JSON_OBJECT('height',100,'hash_height',100,'hash','b100'),backfill_cursor_json=JSON_OBJECT('height',90,'hash_height',90,'hash','b90'),backfill_target_json=JSON_OBJECT('height',100,'hash_height',100,'hash','b100') WHERE network='base'")
        .execute(&pool).await.unwrap();
    let key_snapshot = "SELECT JSON_OBJECT('secret',api_key_encrypted,'enabled',enabled,'interval',request_interval_ms,'next',next_request_at,'cool',cooldown_until,'error',last_error,'count',failure_count,'success',last_success_at) FROM wallet_deposit_scanner_keys WHERE id=?";
    let before: Json<Value> = sqlx::query_scalar(key_snapshot)
        .bind(key)
        .fetch_one(&pool)
        .await
        .unwrap();
    let progress_snapshot = "SELECT JSON_OBJECT('stream',stream,'cursor',cursor_json,'backfill',backfill_cursor_json,'target',backfill_target_json,'lease',lease_token) FROM wallet_deposit_scanner_streams WHERE network='base' ORDER BY stream";
    let progress: Vec<Json<Value>> = sqlx::query_scalar(progress_snapshot)
        .fetch_all(&pool)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let after: Json<Value> = sqlx::query_scalar(key_snapshot)
        .bind(key)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(batch_limit(&pool, key).await, 25);
    let after_progress: Vec<Json<Value>> = sqlx::query_scalar(progress_snapshot)
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(progress, after_progress);
    let error: String =
        sqlx::query_scalar("SELECT last_error FROM wallet_deposit_scanner_providers WHERE id=?")
            .bind(provider)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(error, "provider error");
    pool.close().await;
}
