use super::*;
use crate::modules::wallet::{application::timed_deposits as app, presentation::timed_deposits::*};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
const KEY: &str = "0123456789abcdef0123456789abcdef";
async fn snapshot(pool: &Pool<MySql>) -> ScannerView {
    app::settings(pool)
        .await
        .unwrap()
        .scanners
        .into_iter()
        .find(|s| s.network == "tron")
        .unwrap()
}
async fn recover(pool: &Pool<MySql>, admin: u64, mode: &str) {
    let s = snapshot(pool).await;
    app::recover_scanner(
        pool,
        admin,
        "tron",
        RecoverScanner {
            mode: mode.into(),
            expected_revision: s.revision,
            acknowledge_skip: mode == "skip",
            reason: "local recovery verification".into(),
        },
    )
    .await
    .unwrap();
}
async fn due(pool: &Pool<MySql>) {
    sqlx::query("UPDATE wallet_deposit_scanners SET backfill_next_at=NULL WHERE network='tron'")
        .execute(pool)
        .await
        .unwrap();
}
#[tokio::test]
async fn timed_deposit_recovery_dual_skip_failure_revision_and_accounting() {
    let Ok(url) = std::env::var("DEPOSIT_RECOVERY_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED: isolated DEPOSIT_RECOVERY_TEST_DATABASE_URL required");
        return;
    };
    assert!(url.contains("127.0.0.1") && url.contains("timed_deposits_test_recovery"));
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(12)
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query("SET time_zone = '+00:00'")
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
    let admin_role = sqlx::query(
        "INSERT INTO admin_roles(name,permissions) VALUES ('recovery-fixture',JSON_ARRAY('*'))",
    )
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();
    let admin=sqlx::query("INSERT INTO admin_users(username,password_hash,role_id) VALUES ('recovery-fixture','fixture',?)").bind(admin_role).execute(&pool).await.unwrap().last_insert_id();
    let user = sqlx::query(
        "INSERT INTO users(email,password_hash) VALUES ('recovery@example.test','fixture')",
    )
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();
    sqlx::query("INSERT INTO assets(symbol,name,precision_scale,min_deposit_amount,deposit_fee,deposit_enabled) VALUES ('RCTEST','Recovery test',6,1,0,TRUE)").execute(&pool).await.unwrap();
    let contract =
        crate::modules::wallet::domain::timed_deposits::tron_address(&[0x11; 20]).unwrap();
    let address =
        crate::modules::wallet::domain::timed_deposits::tron_address(&[0x22; 20]).unwrap();
    app::save_asset(
        &pool,
        admin,
        SaveChainAsset {
            network: "tron".into(),
            asset_symbol: "RCTEST".into(),
            asset_kind: "token".into(),
            token_address: contract,
            decimals: 6,
            enabled: true,
            expected_revision: 0,
            reason: "fixture".into(),
        },
    )
    .await
    .unwrap();
    app::import_addresses(
        &pool,
        admin,
        ImportSharedAddresses {
            network: "tron".into(),
            addresses: vec![address],
            reason: "fixture".into(),
        },
    )
    .await
    .unwrap();
    let server = MockServer::start().await;
    let tip = Arc::new(AtomicU64::new(100));
    let bad = Arc::new(AtomicBool::new(false));
    let delay = Arc::new(AtomicBool::new(false));
    let unavailable = Arc::new(AtomicBool::new(false));
    let (t, b, d, u) = (tip.clone(), bad.clone(), delay.clone(), unavailable.clone());
    let millis = (Utc::now() - chrono::Duration::seconds(1)).timestamp_millis();
    Mock::given(matchers::method("POST")).respond_with(move |r:&wiremock::Request| {
        if u.load(Ordering::SeqCst) { return ResponseTemplate::new(503); }
        let v:Value=serde_json::from_slice(&r.body).unwrap(); let n=v["num"].as_u64().unwrap_or(0);
        let result=match r.url.path() {
            "/wallet/triggerconstantcontract" => json!({"result":{"result":true},"constant_result":["06"]}),
            "/wallet/getnowblock" => json!({"block_header":{"raw_data":{"number":t.load(Ordering::SeqCst),"timestamp":Utc::now().timestamp_millis()}}}),
            "/wallet/gettransactioninfobyblocknum" if n==90 => json!([{"id":"recovery-payment","blockNumber":90,"blockTimeStamp":millis,"receipt":{"result":"SUCCESS"},"log":[{"address":"11".repeat(20),"topics":[TRANSFER_TOPIC,"0".repeat(64),format!("{}{}","0".repeat(24),"22".repeat(20))],"data":format!("{:064x}",100_000_000u64)}]}]),
            "/wallet/gettransactioninfobyblocknum" => json!([]),
            _ if n==0 => json!({"blockID":"genesis2b6653dc"}),
            _ => json!({"blockID":if b.load(Ordering::SeqCst) && n==109 {"changed".to_owned()}else{format!("b{n}")},"block_header":{"raw_data":{"number":n,"parentHash":format!("b{}",n-1),"timestamp":millis}},"transactions":if n==90 {json!([{"txID":"recovery-payment"}])}else{json!([])}})
        };
        let response=ResponseTemplate::new(200).set_body_json(result);
        if d.load(Ordering::SeqCst) { response.set_delay(Duration::from_millis(400)) } else { response }
    }).mount(&server).await;
    let revision = snapshot(&pool).await.revision;
    app::save_scanner(&pool,admin,"tron",KEY,serde_json::from_value(json!({"enabled":true,"confirmations":2,"expected_revision":revision,"reason":"fixture","providers":[{"name":"Fixture","priority":0,"enabled":true,"endpoint":server.uri(),"request_interval_ms":100,"keys":[{"name":"Public","priority":0,"enabled":true,"api_key":null,"clear_api_key":false,"request_interval_ms":100}]}]})).unwrap()).await.unwrap();
    sqlx::query("UPDATE wallet_deposit_scanners SET cursor_json=?,last_scan_at=CURRENT_TIMESTAMP(6) WHERE network='tron'").bind(Json(json!({"height":80,"hash_height":80,"hash":"b80"}))).execute(&pool).await.unwrap();
    let order = app::create_order(
        &pool,
        user,
        CreateDepositOrder {
            network: "tron".into(),
            asset_symbol: "RCTEST".into(),
            amount: "100".parse().unwrap(),
            idempotency_key: "recovery-payment".into(),
        },
    )
    .await
    .unwrap();
    sqlx::query("UPDATE wallet_deposit_orders SET created_at=DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 60 SECOND) WHERE id=?").bind(order.id).execute(&pool).await.unwrap();
    let before = snapshot(&pool).await;
    assert!(
        app::recover_scanner(
            &pool,
            admin,
            "tron",
            RecoverScanner {
                mode: "skip".into(),
                acknowledge_skip: false,
                expected_revision: before.revision,
                reason: "fixture".into()
            }
        )
        .await
        .is_err()
    );
    recover(&pool, admin, "backfill").await;
    assert!(
        app::recover_scanner(
            &pool,
            admin,
            "tron",
            RecoverScanner {
                mode: "skip".into(),
                acknowledge_skip: true,
                expected_revision: before.revision,
                reason: "stale".into()
            }
        )
        .await
        .is_err()
    );
    assert!(!snapshot(&pool).await.scan_fresh);
    run_once(&pool, Some(KEY)).await.unwrap();
    let jumped = snapshot(&pool).await;
    assert_eq!(
        (
            jumped.cursor_height,
            jumped.backfill_cursor_height,
            jumped.backfill_target_height
        ),
        (Some(99), Some(80), Some(99))
    );
    assert!(jumped.recovery_mode.is_none());
    run_once(&pool, Some(KEY)).await.unwrap();
    assert!(
        snapshot(&pool).await.scan_fresh,
        "new deposits resume before history catches up"
    );
    assert_eq!(
        app::get_order(&pool, Some(user), order.id)
            .await
            .unwrap()
            .status,
        "pending"
    );
    // A second dual request extends the fixed history range without losing its oldest checkpoint.
    tip.store(110, Ordering::SeqCst);
    recover(&pool, admin, "backfill").await;
    run_once(&pool, Some(KEY)).await.unwrap();
    assert_eq!(snapshot(&pool).await.backfill_cursor_height, Some(80));
    assert_eq!(snapshot(&pool).await.backfill_target_height, Some(109));
    run_once(&pool, Some(KEY)).await.unwrap();
    // Historical anchor mismatch closes admission even when current scanning succeeds later.
    bad.store(true, Ordering::SeqCst);
    due(&pool).await;
    run_once(&pool, Some(KEY)).await.unwrap();
    assert!(snapshot(&pool).await.backfill_blocked);
    assert!(app::options(&pool).await.unwrap().is_empty());
    bad.store(false, Ordering::SeqCst);
    due(&pool).await;
    run_once(&pool, Some(KEY)).await.unwrap();
    assert!(!snapshot(&pool).await.backfill_blocked);
    assert_eq!(snapshot(&pool).await.backfill_target_height, None);
    assert_eq!(
        app::get_order(&pool, Some(user), order.id)
            .await
            .unwrap()
            .status,
        "credited"
    );
    let ledgers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallet_ledger")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(ledgers, 1);
    run_once(&pool, Some(KEY)).await.unwrap();
    // Replay the historical range as after a crash; existing chain evidence stays idempotent.
    sqlx::query("UPDATE wallet_deposit_scanners SET backfill_cursor_json=?,backfill_target_json=cursor_json,backfill_next_at=NULL WHERE network='tron'").bind(Json(json!({"height":80,"hash_height":80,"hash":"b80"}))).execute(&pool).await.unwrap();
    run_once(&pool, Some(KEY)).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM wallet_ledger")
            .fetch_one(&pool)
            .await
            .unwrap(),
        ledgers
    );
    assert!(
        snapshot(&pool).await.backfill_target_height.is_none(),
        "replayed history completed"
    );
    // A transport failure in history preserves the current listener health and history position.
    sqlx::query("UPDATE wallet_deposit_scanners SET backfill_cursor_json=?,backfill_target_json=cursor_json,backfill_next_at=NULL WHERE network='tron'").bind(Json(json!({"height":80,"hash_height":80,"hash":"b80"}))).execute(&pool).await.unwrap();
    unavailable.store(true, Ordering::SeqCst);
    run_once(&pool, Some(KEY)).await.unwrap();
    let failed_history = snapshot(&pool).await;
    assert_eq!(failed_history.backfill_cursor_height, Some(80));
    assert!(failed_history.backfill_last_error.is_some());
    assert!(!failed_history.backfill_blocked);
    assert!(failed_history.last_error.is_none());
    unavailable.store(false, Ordering::SeqCst);
    sqlx::query("UPDATE wallet_deposit_scanner_providers SET cooldown_until=NULL")
        .execute(&pool)
        .await
        .unwrap();
    // Skip discards the accumulated backlog only after a healthy head read.
    tip.store(200, Ordering::SeqCst);
    recover(&pool, admin, "backfill").await;
    run_once(&pool, Some(KEY)).await.unwrap();
    recover(&pool, admin, "skip").await;
    unavailable.store(true, Ordering::SeqCst);
    run_once(&pool, Some(KEY)).await.unwrap();
    assert_eq!(snapshot(&pool).await.cursor_height, Some(199));
    assert!(snapshot(&pool).await.backfill_target_height.is_some());
    assert_eq!(snapshot(&pool).await.recovery_mode.as_deref(), Some("skip"));
    unavailable.store(false, Ordering::SeqCst);
    tip.store(210, Ordering::SeqCst);
    sqlx::query("UPDATE wallet_deposit_scanner_providers SET cooldown_until=NULL")
        .execute(&pool)
        .await
        .unwrap();
    run_once(&pool, Some(KEY)).await.unwrap();
    let skipped = snapshot(&pool).await;
    assert_eq!(skipped.cursor_height, Some(209));
    assert!(skipped.backfill_cursor_height.is_none());
    // Saving a new revision during node I/O fences the obsolete recovery result.
    tip.store(300, Ordering::SeqCst);
    recover(&pool, admin, "backfill").await;
    delay.store(true, Ordering::SeqCst);
    let calls = server.received_requests().await.unwrap().len();
    let pool2 = pool.clone();
    let scan = tokio::spawn(async move { run_once(&pool2, Some(KEY)).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while server.received_requests().await.unwrap().len() == calls {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    sqlx::query("UPDATE wallet_deposit_scanners SET revision=revision+1,lease_token=NULL,lease_until=NULL WHERE network='tron'").execute(&pool).await.unwrap();
    scan.await.unwrap().unwrap();
    assert_eq!(snapshot(&pool).await.cursor_height, Some(209));
    let audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_audit_logs WHERE action='deposit_scanner.recovery_applied'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audits, 4);
}
