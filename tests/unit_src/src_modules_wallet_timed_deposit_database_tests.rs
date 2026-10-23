use super::*;
use crate::modules::wallet::{
    application::timed_deposits as app, domain::timed_deposits::ChainReceipt,
    presentation::timed_deposits::*,
};
use bigdecimal::BigDecimal;
use chrono::Duration;
use sqlx::mysql::MySqlPoolOptions;

fn amount(s: &str) -> BigDecimal {
    s.parse().unwrap()
}
fn request(symbol: &str, value: &str, key: &str) -> CreateDepositOrder {
    CreateDepositOrder {
        asset_symbol: symbol.into(),
        network: "eth".into(),
        amount: amount(value),
        idempotency_key: key.into(),
    }
}
fn evidence(order: &DepositOrder, value: &str) -> ChainReceipt {
    ChainReceipt {
        network: "eth".into(),
        tx_hash: uuid::Uuid::now_v7().simple().to_string(),
        event_index: 0,
        asset_kind: "token".into(),
        token_address: "0x1111111111111111111111111111111111111111".into(),
        address: order.address.clone(),
        amount: amount(value),
        block_height: 100,
        block_hash: "block100".into(),
        block_time: order.created_at + Duration::seconds(1),
        confirmations: 12,
    }
}

#[tokio::test]
async fn timed_deposit_database_reservations_and_accounting() {
    let Ok(url) = std::env::var("TIMED_DEPOSIT_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED: TIMED_DEPOSIT_TEST_DATABASE_URL is required for real MySQL validation");
        return;
    };
    assert!(
        url.contains("127.0.0.1") && url.contains("timed_deposits_test"),
        "This fixture must run against the isolated local test database"
    );
    let pool = MySqlPoolOptions::new()
        .max_connections(24)
        .after_connect(|conn, _| {
            Box::pin(async move {
                sqlx::query("SET time_zone='+00:00'").execute(conn).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let suffix = uuid::Uuid::now_v7().simple().to_string();
    let symbol = format!("TD{}", &suffix[16..]).to_uppercase();
    let asset=sqlx::query("INSERT INTO assets (symbol,name,precision_scale,min_deposit_amount,deposit_fee) VALUES (?,'Timed test',18,1,1)").bind(&symbol).execute(&pool).await.unwrap().last_insert_id();
    let role = sqlx::query("INSERT INTO admin_roles (name,permissions) VALUES (?,JSON_ARRAY('*'))")
        .bind(&suffix)
        .execute(&pool)
        .await
        .unwrap()
        .last_insert_id();
    let admin =
        sqlx::query("INSERT INTO admin_users (username,password_hash,role_id) VALUES (?,'test',?)")
            .bind(&suffix)
            .bind(role)
            .execute(&pool)
            .await
            .unwrap()
            .last_insert_id();
    let mut users = vec![];
    for index in 0..12 {
        users.push(
            sqlx::query("INSERT INTO users (email,password_hash) VALUES (?,'test')")
                .bind(format!("{suffix}-{index}@example.test"))
                .execute(&pool)
                .await
                .unwrap()
                .last_insert_id(),
        );
    }
    sqlx::query("UPDATE deposit_network_configs SET asset_symbols_json=NULL WHERE network='eth'")
        .execute(&pool)
        .await
        .unwrap();
    app::save_asset(
        &pool,
        admin,
        SaveChainAsset {
            network: "eth".into(),
            asset_symbol: symbol.clone(),
            asset_kind: "token".into(),
            token_address: "0x1111111111111111111111111111111111111111".into(),
            decimals: 18,
            enabled: true,
            expected_revision: 0,
            reason: "test asset identity".into(),
        },
    )
    .await
    .unwrap();
    let mut addresses = vec![];
    for index in 0..10 {
        let address = format!("0x{:040x}", asset * 100 + index);
        let id=sqlx::query("INSERT INTO deposit_address_pool (network,address_group_code,address,asset_symbol) VALUES ('eth','evm',?,?)").bind(&address).bind(&symbol).execute(&pool).await.unwrap().last_insert_id();
        app::save_address(
            &pool,
            admin,
            id,
            SaveSharedAddress {
                enabled: true,
                reason: "test shared inventory".into(),
            },
        )
        .await
        .unwrap();
        addresses.push(id);
    }
    sqlx::query("UPDATE wallet_deposit_scanners SET enabled=TRUE,endpoint_encrypted='test-only',last_scan_at=CURRENT_TIMESTAMP(6),last_error=NULL WHERE network='eth'").execute(&pool).await.unwrap();
    assert!(
        app::options(&pool)
            .await
            .unwrap()
            .iter()
            .any(|o| o.asset_symbol == symbol)
    );
    // Concurrent same-amount requests consume exactly ten distinct combinations, never a modified amount.
    let attempts = futures_util::future::join_all((0..12).map(|index| {
        app::create_order(
            &pool,
            users[index],
            request(&symbol, "100", &format!("{suffix}-{index}")),
        )
    }))
    .await;
    let orders: Vec<_> = attempts
        .iter()
        .filter_map(|r| r.as_ref().ok())
        .cloned()
        .collect();
    assert_eq!(orders.len(), 10, "{attempts:?}");
    assert_eq!(
        orders
            .iter()
            .map(|o| &o.address)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        10
    );
    assert!(
        orders
            .iter()
            .all(|o| o.amount == amount("100")
                && o.expires_at - o.created_at == Duration::minutes(30))
    );
    let first = &orders[0];
    let filtered = app::list_orders(
        &pool,
        Some(first.user_id),
        &DepositPageQuery {
            order_no: Some(first.order_no.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(filtered.total, 1);
    assert_eq!(filtered.items[0].id, first.id);
    let other_user = *users.iter().find(|id| **id != first.user_id).unwrap();
    assert!(
        app::get_order(&pool, Some(other_user), first.id)
            .await
            .is_err()
    );
    assert_eq!(
        app::list_orders(
            &pool,
            Some(other_user),
            &DepositPageQuery {
                order_no: Some(first.order_no.clone()),
                ..Default::default()
            }
        )
        .await
        .unwrap()
        .total,
        0
    );

    let key = sqlx::query_scalar::<_, String>(
        "SELECT idempotency_key FROM wallet_deposit_orders WHERE id=?",
    )
    .bind(first.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        app::create_order(&pool, first.user_id, request(&symbol, "100.0", &key))
            .await
            .unwrap()
            .id,
        first.id
    );
    assert!(
        app::create_order(&pool, first.user_id, request(&symbol, "200", &key))
            .await
            .is_err()
    );
    let different = app::create_order(
        &pool,
        users[11],
        request(&symbol, "200", &format!("{suffix}-different")),
    )
    .await
    .unwrap();
    assert!(orders.iter().any(|o| o.address == different.address));
    assert!(
        app::get_order(&pool, Some(users[11]), first.id)
            .await
            .is_err()
            || first.user_id == users[11]
    );
    // Same request under concurrency yields the same order and one permanent idempotency binding.
    let (a, b) = tokio::join!(
        app::create_order(&pool, users[11], request(&symbol, "250", "same-request")),
        app::create_order(&pool, users[11], request(&symbol, "250", "same-request"))
    );
    assert_eq!(a.unwrap().id, b.unwrap().id);
    // Receipt replay, confirmation threshold, fee and balanced journal use the same transaction.
    let mut paid = evidence(&different, "200");
    let current_revision: u64 =
        sqlx::query_scalar("SELECT revision FROM wallet_deposit_scanners WHERE network='eth'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        settlement::observe_scanned(&pool, &paid, current_revision + 1)
            .await
            .is_err()
    );
    assert_ne!(
        app::get_order(&pool, None, different.id)
            .await
            .unwrap()
            .status,
        "credited"
    );
    paid.confirmations = 1;
    let receipt_id = settlement::observe_scanned(&pool, &paid, current_revision)
        .await
        .unwrap();
    assert_eq!(
        app::get_order(&pool, None, different.id)
            .await
            .unwrap()
            .status,
        "pending"
    );
    paid.confirmations = 12;
    settlement::observe(&pool, &paid).await.unwrap();
    settlement::observe(&pool, &paid).await.unwrap();
    assert_eq!(
        app::get_order(&pool, None, different.id)
            .await
            .unwrap()
            .status,
        "credited"
    );
    let receipt_query = DepositPageQuery {
        network: Some("eth".into()),
        asset_symbol: Some(symbol.clone()),
        order_no: Some(different.order_no.clone()),
        status: Some("credited".into()),
        ..Default::default()
    };
    let receipts = app::receipts(&pool, &receipt_query).await.unwrap();
    assert_eq!(receipts.total, 1);
    assert_eq!(receipts.items[0].id, receipt_id);
    assert_eq!(
        app::receipts(
            &pool,
            &DepositPageQuery {
                network: Some("base".into()),
                ..receipt_query
            }
        )
        .await
        .unwrap()
        .total,
        0
    );
    let available: BigDecimal =
        sqlx::query_scalar("SELECT available FROM wallet_accounts WHERE user_id=? AND asset_id=?")
            .bind(users[11])
            .bind(asset)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(available, amount("199"));
    let ledgers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallet_ledger WHERE asset_id=?")
        .bind(asset)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(ledgers, 1);
    let sum: BigDecimal =
        sqlx::query_scalar("SELECT SUM(amount) FROM platform_financial_journal WHERE asset_id=?")
            .bind(asset)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(sum, amount("0"));
    let mut changed = paid.clone();
    changed.amount = amount("201");
    assert!(settlement::observe(&pool, &changed).await.is_err());
    assert_eq!(
        settlement::list(&pool, &DepositPageQuery::default())
            .await
            .unwrap()
            .items
            .iter()
            .find(|r| r.id == receipt_id)
            .unwrap()
            .status,
        "credited"
    );
    // Expiry releases immediately, even without the expiration worker; history prevents ambiguous automatic credit.
    sqlx::query("UPDATE wallet_deposit_orders SET created_at=DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 60 MINUTE),expires_at=DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 30 MINUTE) WHERE id=?").bind(first.id).execute(&pool).await.unwrap();
    let replacement = app::create_order(
        &pool,
        users[11],
        request(&symbol, "100", &format!("{suffix}-reused")),
    )
    .await
    .unwrap();
    assert_eq!(replacement.address, first.address);
    let ambiguous = evidence(&replacement, "100");
    let ambiguous_id = settlement::observe(&pool, &ambiguous).await.unwrap();
    assert_eq!(
        app::get_order(&pool, None, replacement.id)
            .await
            .unwrap()
            .status,
        "review"
    );
    app::review(
        &pool,
        admin,
        ambiguous_id,
        ReviewDepositReceipt {
            action: "credit".into(),
            order_id: Some(replacement.id),
            reason: "verified payer and target order".into(),
        },
    )
    .await
    .unwrap();
    app::review(
        &pool,
        admin,
        ambiguous_id,
        ReviewDepositReceipt {
            action: "credit".into(),
            order_id: Some(replacement.id),
            reason: "replay".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        app::get_order(&pool, None, replacement.id)
            .await
            .unwrap()
            .paid_amount,
        Some(amount("100"))
    );
    // A delayed scanner uses block time to credit an expired historical window, not the current occupant.
    let old = app::create_order(
        &pool,
        users[0],
        request(&symbol, "300", &format!("{suffix}-late-old")),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE wallet_deposit_orders SET created_at=DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 60 MINUTE),expires_at=DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 30 MINUTE) WHERE id=?").bind(old.id).execute(&pool).await.unwrap();
    let old = app::get_order(&pool, None, old.id).await.unwrap();
    let new = app::create_order(
        &pool,
        users[1],
        request(&symbol, "300", &format!("{suffix}-late-new")),
    )
    .await
    .unwrap();
    settlement::observe(&pool, &evidence(&old, "300"))
        .await
        .unwrap();
    assert_eq!(
        app::get_order(&pool, None, old.id).await.unwrap().status,
        "credited"
    );
    assert_eq!(
        app::get_order(&pool, None, new.id).await.unwrap().status,
        "pending"
    );
    // Wrong amount remains separate evidence; review credits actual amount and preserves the original request.
    let mismatch = app::create_order(
        &pool,
        users[2],
        request(&symbol, "400", &format!("{suffix}-mismatch")),
    )
    .await
    .unwrap();
    let mismatch_id = settlement::observe(&pool, &evidence(&mismatch, "399"))
        .await
        .unwrap();
    assert_eq!(
        app::get_order(&pool, None, mismatch.id)
            .await
            .unwrap()
            .status,
        "pending"
    );
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallet_ledger WHERE asset_id=?")
        .bind(asset)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        app::review(
            &pool,
            u64::MAX,
            mismatch_id,
            ReviewDepositReceipt {
                action: "credit".into(),
                order_id: Some(mismatch.id),
                reason: "invalid auditor forces transaction rollback".into()
            }
        )
        .await
        .is_err()
    );
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallet_ledger WHERE asset_id=?")
        .bind(asset)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    app::review(
        &pool,
        admin,
        mismatch_id,
        ReviewDepositReceipt {
            action: "credit".into(),
            order_id: Some(mismatch.id),
            reason: "verified actual received amount".into(),
        },
    )
    .await
    .unwrap();
    let saved = app::get_order(&pool, None, mismatch.id).await.unwrap();
    assert_eq!(saved.amount, amount("400"));
    assert_eq!(saved.paid_amount, Some(amount("399")));
    let payload = serde_json::to_value(&saved).unwrap();
    assert!(payload["amount"].is_string());
    // A block's second-resolution timestamp cannot prove ordering within the creation second.
    // Keep it out of automatic credit, while permitting an explicitly audited manual assignment.
    let same_second = app::create_order(&pool, users[0], request(&symbol, "444", "same-second"))
        .await
        .unwrap();
    let mut rounded = evidence(&same_second, "444");
    rounded.block_time =
        chrono::DateTime::from_timestamp(same_second.created_at.timestamp(), 0).unwrap();
    let receipt_id = settlement::observe(&pool, &rounded).await.unwrap();
    assert_ne!(
        app::get_order(&pool, None, same_second.id)
            .await
            .unwrap()
            .status,
        "credited"
    );
    app::review(
        &pool,
        admin,
        receipt_id,
        ReviewDepositReceipt {
            action: "credit".into(),
            order_id: Some(same_second.id),
            reason: "verified ownership within timestamp resolution".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        app::get_order(&pool, None, same_second.id)
            .await
            .unwrap()
            .status,
        "credited"
    );
    // Readiness and inventory are explicit, and a permanent allocator cannot claim enrolled addresses.
    sqlx::query(
        "UPDATE wallet_deposit_scanners SET last_error='node unavailable' WHERE network='eth'",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        !app::options(&pool)
            .await
            .unwrap()
            .iter()
            .any(|o| o.asset_symbol == symbol)
    );
    assert!(
        app::create_order(&pool, users[0], request(&symbol, "500", "unavailable"))
            .await
            .is_err()
    );
    let assigned:i64=sqlx::query_scalar("SELECT COUNT(*) FROM deposit_address_pool WHERE asset_symbol=? AND assigned_user_id IS NOT NULL").bind(&symbol).fetch_one(&pool).await.unwrap();
    assert_eq!(assigned, 0);
    // A successful confirmed-block checkpoint survives restart; stale heads and reorgs retain the cursor.
    let server = wiremock::MockServer::start().await;
    let reorg = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = reorg.clone();
    wiremock::Mock::given(wiremock::matchers::method("POST")).respond_with(move |request:&wiremock::Request| {
        let body:serde_json::Value=serde_json::from_slice(&request.body).unwrap();
        let result=match body["method"].as_str().unwrap() {
            "eth_chainId"=>serde_json::json!("0x1"),
            "eth_call"=>serde_json::json!("0x12"),
            "eth_getLogs"=>serde_json::json!([]),
            "eth_getBlockByNumber"=>{
                let height=match body["params"][0].as_str().unwrap(){"latest"=>112,"0x64"=>100,"0x65"=>101,other=>panic!("unexpected block {other}")};
                let hash=if height==101 && flag.load(std::sync::atomic::Ordering::SeqCst){"changed".to_owned()}else{format!("b{height}")};
                serde_json::json!({"number":format!("0x{height:x}"),"hash":hash,"parentHash":format!("b{}",height-1),"timestamp":format!("0x{:x}",chrono::Utc::now().timestamp()),"transactions":[]})
            },
            other=>panic!("unexpected RPC {other}"),
        };
        wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({"jsonrpc":"2.0","id":1,"result":result}))
    }).mount(&server).await;
    let revision: u64 =
        sqlx::query_scalar("SELECT revision FROM wallet_deposit_scanners WHERE network='eth'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let key = "0123456789abcdef0123456789abcdef";
    app::save_scanner(
        &pool,
        admin,
        "eth",
        key,
        SaveScanner {
            enabled: true,
            endpoint: Some(server.uri()),
            api_key: None,
            confirmations: 12,
            expected_revision: revision,
            reason: "mock node checkpoint verification".into(),
        },
    )
    .await
    .unwrap();
    sqlx::query("UPDATE wallet_deposit_scanners SET cursor_json=JSON_OBJECT('height',100,'hash_height',100,'hash','b100') WHERE network='eth'").execute(&pool).await.unwrap();
    app::scan_once(&pool, Some(key)).await.unwrap();
    let checkpoint: sqlx::types::Json<serde_json::Value> =
        sqlx::query_scalar("SELECT cursor_json FROM wallet_deposit_scanners WHERE network='eth'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(checkpoint.0["height"], 101);
    assert!(
        app::options(&pool)
            .await
            .unwrap()
            .iter()
            .any(|option| option.asset_symbol == symbol)
    );
    reorg.store(true, std::sync::atomic::Ordering::SeqCst);
    app::scan_once(&pool, Some(key)).await.unwrap();
    let after: sqlx::types::Json<serde_json::Value> =
        sqlx::query_scalar("SELECT cursor_json FROM wallet_deposit_scanners WHERE network='eth'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after.0, checkpoint.0);
    let error: Option<String> =
        sqlx::query_scalar("SELECT last_error FROM wallet_deposit_scanners WHERE network='eth'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(error.unwrap().contains("reorganization"));
    assert!(
        !app::options(&pool)
            .await
            .unwrap()
            .iter()
            .any(|option| option.asset_symbol == symbol)
    );
    reorg.store(false, std::sync::atomic::Ordering::SeqCst);
    app::scan_once(&pool, Some(key)).await.unwrap();
    assert!(
        app::options(&pool)
            .await
            .unwrap()
            .iter()
            .any(|option| option.asset_symbol == symbol)
    );
    sqlx::query("UPDATE wallet_deposit_scanners SET cursor_json=NULL WHERE network='eth'")
        .execute(&pool)
        .await
        .unwrap();
    app::scan_once(&pool, Some(key)).await.unwrap();
    let error: Option<String> =
        sqlx::query_scalar("SELECT last_error FROM wallet_deposit_scanners WHERE network='eth'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(error.unwrap().contains("historical orders"));
    pool.close().await;
}
