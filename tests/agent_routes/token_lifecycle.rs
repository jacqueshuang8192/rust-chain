use super::coin_applications::{ok, request, token};
use super::*;
use redis::AsyncCommands;
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn policy() -> Value {
    json!({"version":2,"timezone":"Asia/Hong_Kong","purchase_initial_price":"0.3","listing_reference_price":"9.99",
    "subscription":{"automatic":false,"at":1800000000000i64},"distribution":{"automatic":false,"at":1800001000000i64},"purchase":{"automatic":false,"at":1800002000000i64},"listing":{"automatic":false,"at":1800003000000i64},"lock":{"kind":"on_purchase"},"fee":{"kind":"profit","rate":"0.2"}})
}
fn utoken(user: u64) -> String {
    issue_token(
        &test_settings(),
        format!("user:{user}"),
        TokenScope::User,
        900,
    )
    .unwrap()
}
async fn stage(app: &axum::Router, token: &str, id: u64, from: &str, to: &str) -> Value {
    ok(request(
        app,
        token,
        "POST",
        &format!("/agent/api/v1/token-projects/{id}/stage"),
        json!({"expected_stage":from,"target_stage":to,"reason":"advance stage"}),
    )
    .await)
}
async fn ticker(
    redis: &redis::aio::ConnectionManager,
    pool: &MySqlPool,
    pair: u64,
    symbol: &str,
    price: &str,
    age: i64,
) {
    let now = chrono::Utc::now();
    let payload = json!({"symbol":symbol,"last_price":price,"provider":"strategy","source":"default","observed_at":(now-chrono::Duration::seconds(age)).timestamp_millis()});
    redis
        .clone()
        .set::<_, _, ()>(
            exchange_api::modules::market::market_ticker_redis_key(symbol),
            payload.to_string(),
        )
        .await
        .unwrap();
    sqlx::query("INSERT INTO market_pair_generation_runs(pair_id,generation,active_source,default_version,lease_owner,lease_expires_at,last_price,last_tick_at) VALUES (?,1,'default',2,'test',DATE_ADD(UTC_TIMESTAMP(6),INTERVAL 5 MINUTE),?,UTC_TIMESTAMP(6)) ON DUPLICATE KEY UPDATE active_source='default',lease_expires_at=VALUES(lease_expires_at),last_price=VALUES(last_price),last_tick_at=VALUES(last_tick_at)")
        .bind(pair).bind(BigDecimal::from_str(price).unwrap()).execute(pool).await.unwrap();
}
async fn wallet(pool: &MySqlPool, user: u64, asset: u64) -> (BigDecimal, BigDecimal, BigDecimal) {
    sqlx::query_as(
        "SELECT available,frozen,locked FROM wallet_accounts WHERE user_id=? AND asset_id=?",
    )
    .bind(user)
    .bind(asset)
    .fetch_one(pool)
    .await
    .unwrap()
}
fn decimal(value: &Value) -> BigDecimal {
    BigDecimal::from_str(value.as_str().unwrap()).unwrap()
}

#[tokio::test]
async fn token_lifecycle_scopes_allocations_quotes_buyback_and_continuous_spot_handoff() {
    let _serial = SERIAL.lock().await;
    let Some(pool) = mysql_pool().await else {
        return;
    };
    let redis_url =
        std::env::var("TOKEN_LIFECYCLE_REDIS_URL").expect("explicit disposable Redis is required");
    let redis = redis::aio::ConnectionManager::new(redis::Client::open(redis_url).unwrap())
        .await
        .unwrap();
    let a = create_agent(&pool, "token-a").await;
    let c = create_agent(&pool, "token-c").await;
    let b = create_user(&pool, "token-b").await;
    let d = create_user(&pool, "token-d").await;
    refer_user_to_agent(&pool, b, a.agent_id, 1).await;
    refer_user_to_agent(&pool, d, c.agent_id, 1).await;
    let (_, admin, _) = create_admin_with_password(&pool, "token-admin", "password").await;
    let admin_token = issue_token(
        &test_settings(),
        format!("admin:{admin}"),
        TokenScope::Admin,
        900,
    )
    .unwrap();
    let at = token(a);
    let ct = token(c);
    let bt = utoken(b);
    let dt = utoken(d);
    let state = AppState::new(test_settings())
        .with_mysql(pool.clone())
        .with_redis(redis.clone());
    let app = build_router(state.clone());
    let quote = sqlx::query(
        "INSERT INTO assets(symbol,name,precision_scale,status) VALUES (?,'quote',8,'active')",
    )
    .bind(format!("Q{}", &Uuid::now_v7().simple().to_string()[20..]).to_uppercase())
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();
    for user in [b, d] {
        sqlx::query("INSERT INTO wallet_accounts(user_id,asset_id,available) VALUES (?,?,5000)")
            .bind(user)
            .bind(quote)
            .execute(&pool)
            .await
            .unwrap();
    }
    let symbol = format!("T{}", &Uuid::now_v7().simple().to_string()[20..]).to_uppercase();
    let input = json!({"symbol":symbol,"name":"Token lifecycle","precision_scale":8,"total_supply":"20000","issue_price":"0.2","quote_asset_id":quote,"listed_at":1800003000000i64,"reason":"new issue","idempotency_key":"token-application","issuance_policy":policy()});
    let mut oversized = input.clone();
    oversized["symbol"] = json!("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    assert_eq!(
        request(
            &app,
            &ct,
            "POST",
            "/agent/api/v1/coin-applications",
            oversized
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let application = ok(request(
        &app,
        &ct,
        "POST",
        "/agent/api/v1/coin-applications",
        input.clone(),
    )
    .await);
    assert_eq!(application["issuance_policy"], policy());
    let mut changed = input.clone();
    changed["issuance_policy"]["lock"] = json!({"kind":"none"});
    assert_eq!(
        request(
            &app,
            &ct,
            "POST",
            "/agent/api/v1/coin-applications",
            changed
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let approved = ok(request(
        &app,
        &admin_token,
        "POST",
        &format!(
            "/admin/api/v1/agent-coin-applications/{}/review",
            application["id"]
        ),
        json!({"decision":"approve","reason":"approved"}),
    )
    .await);
    let project = approved["project_id"].as_u64().unwrap();
    let (asset, pair): (u64, u64) =
        sqlx::query_as("SELECT asset_id,post_listing_pair_id FROM new_coin_projects WHERE id=?")
            .bind(project)
            .fetch_one(&pool)
            .await
            .unwrap();
    let pair_symbol: String = sqlx::query_scalar("SELECT symbol FROM trading_pairs WHERE id=?")
        .bind(pair)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        wallet(&pool, b, asset).await,
        (
            BigDecimal::from(0),
            BigDecimal::from(0),
            BigDecimal::from(0)
        )
    );
    assert_eq!(request(&app,&at,"POST",&format!("/agent/api/v1/token-projects/{project}/stage"),json!({"expected_stage":"preheat","target_stage":"subscription","reason":"wrong issuer"})).await.0,StatusCode::NOT_FOUND);
    let public = ok(request(
        &app,
        &bt,
        "GET",
        &format!("/api/v1/new-coins/{symbol}"),
        json!(null),
    )
    .await);
    assert_eq!(public["post_listing_pair_symbol"], pair_symbol);
    let own = ok(request(
        &app,
        &ct,
        "GET",
        "/agent/api/v1/token-projects",
        json!(null),
    )
    .await);
    assert_eq!(own["projects"][0]["id"], project);
    let older = ok(request(
        &app,
        &ct,
        "GET",
        &format!("/agent/api/v1/token-projects?before_id={project}"),
        json!(null),
    )
    .await);
    assert!(older["projects"].as_array().unwrap().is_empty());
    let foreign = ok(request(
        &app,
        &at,
        "GET",
        "/agent/api/v1/token-projects",
        json!(null),
    )
    .await);
    assert!(foreign["projects"].as_array().unwrap().is_empty());
    stage(&app, &ct, project, "preheat", "subscription").await;
    for (t, key) in [(&bt, "b-sub"), (&dt, "d-sub")] {
        ok(request(&app,t,"POST",&format!("/api/v1/new-coins/{symbol}/subscriptions"),json!({"quote_asset_id":quote,"quote_amount":"1000","quantity":"5000","idempotency_key":format!("{key}-{project}")})).await);
    }
    let b_order: u64 = sqlx::query_scalar(
        "SELECT id FROM new_coin_subscriptions WHERE project_id=? AND user_id=?",
    )
    .bind(project)
    .bind(b)
    .fetch_one(&pool)
    .await
    .unwrap();
    let d_order: u64 = sqlx::query_scalar(
        "SELECT id FROM new_coin_subscriptions WHERE project_id=? AND user_id=?",
    )
    .bind(project)
    .bind(d)
    .fetch_one(&pool)
    .await
    .unwrap();
    stage(&app, &ct, project, "subscription", "distribution").await;
    let batch = json!({"subscription_ids":[b_order],"percent":"50","reason":"selected batch","idempotency_key":"b-50"});
    // Token issuer C cannot allocate A's buyer; even a mixed batch must roll back everything.
    assert_eq!(
        request(
            &app,
            &ct,
            "POST",
            "/agent/api/v1/new-coin-allocations",
            batch.clone()
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let mixed = json!({"subscription_ids":[b_order,d_order],"percent":"80","reason":"mixed","idempotency_key":"mixed"});
    assert_eq!(
        request(
            &app,
            &at,
            "POST",
            "/agent/api/v1/new-coin-allocations",
            mixed
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        wallet(&pool, b, quote).await,
        (
            BigDecimal::from(4000),
            BigDecimal::from(1000),
            BigDecimal::from(0)
        )
    );
    // Final audit failure proves that refund, token delivery, treasury and supply all roll back.
    sqlx::raw_sql(&format!("CREATE TRIGGER token_allocation_fail BEFORE INSERT ON new_coin_operations FOR EACH ROW BEGIN IF NEW.project_id={project} AND NEW.action='new_coin.allocate' THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='test audit failure'; END IF; END")).execute(&pool).await.unwrap();
    assert!(
        request(
            &app,
            &at,
            "POST",
            "/agent/api/v1/new-coin-allocations",
            batch.clone()
        )
        .await
        .0
        .is_server_error()
    );
    sqlx::raw_sql("DROP TRIGGER token_allocation_fail")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        wallet(&pool, b, quote).await,
        (
            BigDecimal::from(4000),
            BigDecimal::from(1000),
            BigDecimal::from(0)
        )
    );
    assert_eq!(
        wallet(&pool, b, asset).await,
        (
            BigDecimal::from(0),
            BigDecimal::from(0),
            BigDecimal::from(0)
        )
    );
    let (one, two) = tokio::join!(
        request(
            &app,
            &at,
            "POST",
            "/agent/api/v1/new-coin-allocations",
            batch.clone()
        ),
        request(
            &app,
            &at,
            "POST",
            "/agent/api/v1/new-coin-allocations",
            batch.clone()
        )
    );
    let allocated = ok(one);
    assert_eq!(allocated, ok(two));
    assert_eq!(
        decimal(&allocated["results"][0]["quantity"]),
        BigDecimal::from(2500)
    );
    assert_eq!(
        decimal(&allocated["results"][0]["refunded_quote_amount"]),
        BigDecimal::from(500)
    );
    assert_eq!(
        wallet(&pool, b, quote).await,
        (
            BigDecimal::from(4500),
            BigDecimal::from(0),
            BigDecimal::from(0)
        )
    );
    assert_eq!(
        wallet(&pool, b, asset).await,
        (
            BigDecimal::from(0),
            BigDecimal::from(0),
            BigDecimal::from(2500)
        )
    );
    let mut changed_batch = batch.clone();
    changed_batch["percent"] = json!("80");
    assert_eq!(
        request(
            &app,
            &at,
            "POST",
            "/agent/api/v1/new-coin-allocations",
            changed_batch
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut second_batch = batch.clone();
    second_batch["idempotency_key"] = json!("second-allocation");
    assert_eq!(
        request(
            &app,
            &at,
            "POST",
            "/agent/api/v1/new-coin-allocations",
            second_batch
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let holding = allocated["results"][0]["holding_id"].as_u64().unwrap();
    let old_unlocks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM asset_unlock_records WHERE user_id=? AND asset_id=?",
    )
    .bind(b)
    .bind(asset)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(old_unlocks, 0);
    let blocked=request(&app,&ct,"POST",&format!("/agent/api/v1/token-projects/{project}/stage"),json!({"expected_stage":"distribution","target_stage":"purchase","reason":"pending orders"})).await;
    assert_eq!(blocked.0, StatusCode::CONFLICT);
    let dbatch = json!({"subscription_ids":[d_order],"percent":"80","reason":"another batch","idempotency_key":"d-80"});
    let dalloc = ok(request(
        &app,
        &ct,
        "POST",
        "/agent/api/v1/new-coin-allocations",
        dbatch,
    )
    .await);
    assert_eq!(
        decimal(&dalloc["results"][0]["quantity"]),
        BigDecimal::from(4000)
    );
    stage(&app, &ct, project, "distribution", "purchase").await;
    ticker(&redis, &pool, pair, &pair_symbol, "0.4", 0).await;
    // Quote source must be current, not future, paused or from a departed engine generation.
    ticker(&redis, &pool, pair, &pair_symbol, "0.4", -30).await;
    assert_eq!(
        request(
            &app,
            &bt,
            "POST",
            &format!("/api/v1/new-coins/{symbol}/quotes"),
            json!({"side":"buy","quantity":"1","idempotency_key":"future"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    ticker(&redis, &pool, pair, &pair_symbol, "0.4", 0).await;
    sqlx::query("UPDATE market_default_generators SET all_market_paused=1 WHERE pair_id=?")
        .bind(pair)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            &app,
            &bt,
            "POST",
            &format!("/api/v1/new-coins/{symbol}/quotes"),
            json!({"side":"buy","quantity":"1","idempotency_key":"paused"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE market_default_generators SET all_market_paused=0 WHERE pair_id=?")
        .bind(pair)
        .execute(&pool)
        .await
        .unwrap();
    let expiry = ok(request(
        &app,
        &bt,
        "POST",
        &format!("/api/v1/new-coins/{symbol}/quotes"),
        json!({"side":"buy","quantity":"1","idempotency_key":"expiry"}),
    )
    .await);
    sqlx::query("UPDATE new_coin_trade_quotes SET expires_at=DATE_SUB(UTC_TIMESTAMP(6),INTERVAL 1 SECOND) WHERE id=?").bind(expiry["id"].as_str().unwrap()).execute(&pool).await.unwrap();
    assert_eq!(
        request(
            &app,
            &bt,
            "POST",
            &format!(
                "/api/v1/new-coins/quotes/{}/confirm",
                expiry["id"].as_str().unwrap()
            ),
            json!({})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let old_engine = ok(request(
        &app,
        &bt,
        "POST",
        &format!("/api/v1/new-coins/{symbol}/quotes"),
        json!({"side":"buy","quantity":"1","idempotency_key":"old-engine"}),
    )
    .await);
    sqlx::query("UPDATE market_pair_generation_runs SET generation=generation+1 WHERE pair_id=?")
        .bind(pair)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        request(
            &app,
            &bt,
            "POST",
            &format!(
                "/api/v1/new-coins/quotes/{}/confirm",
                old_engine["id"].as_str().unwrap()
            ),
            json!({})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // General spot cannot bypass the purchase phase, even though the chart's pair is active.
    let spot=request(&app,&bt,"POST","/api/v1/spot/orders",json!({"pair_id":pair_symbol,"side":"sell","order_type":"limit","price":"0.4","quantity":"1","idempotency_key":"prelisting-spot"})).await;
    assert!(!spot.0.is_success());
    let buy = ok(request(
        &app,
        &bt,
        "POST",
        &format!("/api/v1/new-coins/{symbol}/quotes"),
        json!({"side":"buy","quantity":"10","idempotency_key":"buy-live"}),
    )
    .await);
    assert_eq!(decimal(&buy["price"]), BigDecimal::from_str("0.4").unwrap());
    assert_eq!(decimal(&buy["gross"]), BigDecimal::from(4));
    let buy_path = format!(
        "/api/v1/new-coins/quotes/{}/confirm",
        buy["id"].as_str().unwrap()
    );
    let bought = ok(request(&app, &bt, "POST", &buy_path, json!({})).await);
    assert_eq!(
        ok(request(&app, &bt, "POST", &buy_path, json!({})).await),
        bought
    );
    let sell = ok(request(
        &app,
        &bt,
        "POST",
        &format!("/api/v1/new-coins/{symbol}/quotes"),
        json!({"side":"sell","holding_id":holding,"idempotency_key":"sell-live"}),
    )
    .await);
    assert_eq!(decimal(&sell["gross"]), BigDecimal::from(1000));
    assert_eq!(decimal(&sell["fee"]), BigDecimal::from(100));
    assert_eq!(decimal(&sell["net"]), BigDecimal::from(900));
    let sell_path = format!(
        "/api/v1/new-coins/quotes/{}/confirm",
        sell["id"].as_str().unwrap()
    );
    assert_eq!(
        request(&app, &dt, "POST", &sell_path, json!({})).await.0,
        StatusCode::NOT_FOUND
    );
    // Insufficient platform funds and a final audit failure cannot consume the holding.
    let treasury: u64 = sqlx::query_scalar("SELECT user_id FROM wallet_ledger WHERE change_type='new_coin_issuance_proceeds' AND ref_type='new_coin_subscription' AND ref_id=? LIMIT 1").bind(b_order.to_string()).fetch_one(&pool).await.unwrap();
    let original_treasury = wallet(&pool, treasury, quote).await.0;
    sqlx::query("UPDATE wallet_accounts SET available=0 WHERE user_id=? AND asset_id=?")
        .bind(treasury)
        .bind(quote)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        !request(&app, &bt, "POST", &sell_path, json!({}))
            .await
            .0
            .is_success()
    );
    assert_eq!(wallet(&pool, b, asset).await.2, BigDecimal::from(2510));
    sqlx::query("UPDATE wallet_accounts SET available=? WHERE user_id=? AND asset_id=?")
        .bind(&original_treasury)
        .bind(treasury)
        .bind(quote)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(&format!("CREATE TRIGGER token_trade_fail BEFORE INSERT ON new_coin_operations FOR EACH ROW BEGIN IF NEW.project_id={project} AND NEW.action='new_coin.trade' THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='test trade audit failure'; END IF; END")).execute(&pool).await.unwrap();
    assert!(
        request(&app, &bt, "POST", &sell_path, json!({}))
            .await
            .0
            .is_server_error()
    );
    sqlx::raw_sql("DROP TRIGGER token_trade_fail")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(wallet(&pool, b, asset).await.2, BigDecimal::from(2510));
    assert_eq!(wallet(&pool, treasury, quote).await.0, original_treasury);
    let (s1, s2) = tokio::join!(
        request(&app, &bt, "POST", &sell_path, json!({})),
        request(&app, &bt, "POST", &sell_path, json!({}))
    );
    assert_eq!(ok(s1), ok(s2));
    assert_eq!(
        wallet(&pool, b, quote).await,
        (
            BigDecimal::from(5396),
            BigDecimal::from(0),
            BigDecimal::from(0)
        )
    );
    assert_eq!(
        wallet(&pool, b, asset).await,
        (
            BigDecimal::from(0),
            BigDecimal::from(0),
            BigDecimal::from(10)
        )
    );
    let unbalanced:i64=sqlx::query_scalar("SELECT COUNT(*) FROM (SELECT transaction_key,asset_id FROM platform_financial_journal WHERE context='new_coin_issuance' GROUP BY transaction_key,asset_id HAVING SUM(amount)<>0) x").fetch_one(&pool).await.unwrap();
    assert_eq!(unbalanced, 0);
    ticker(&redis, &pool, pair, &pair_symbol, "0.41", 120).await;
    assert_eq!(
        request(
            &app,
            &bt,
            "POST",
            &format!("/api/v1/new-coins/{symbol}/quotes"),
            json!({"side":"buy","quantity":"1","idempotency_key":"stale"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    ticker(&redis, &pool, pair, &pair_symbol, "0.42", 0).await;
    let version: u32 =
        sqlx::query_scalar("SELECT version FROM market_default_generators WHERE pair_id=?")
            .bind(pair)
            .fetch_one(&pool)
            .await
            .unwrap();
    stage(&app, &ct, project, "purchase", "listed").await;
    let after:(u64,u32,BigDecimal)=sqlx::query_as("SELECT p.post_listing_pair_id,g.version,r.last_price FROM new_coin_projects p JOIN market_default_generators g ON g.pair_id=p.post_listing_pair_id JOIN market_pair_generation_runs r ON r.pair_id=g.pair_id WHERE p.id=?").bind(project).fetch_one(&pool).await.unwrap();
    assert_eq!(
        after,
        (pair, version, BigDecimal::from_str("0.42").unwrap())
    );
    assert_eq!(
        request(
            &app,
            &bt,
            "POST",
            &format!("/api/v1/new-coins/{symbol}/quotes"),
            json!({"side":"buy","quantity":"1","idempotency_key":"after-listing"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let h = ok(request(&app, &bt, "GET", "/api/v1/new-coins/holdings", json!(null)).await);
    assert_eq!(h["holdings"].as_array().unwrap().len(), 2);
    let newer_id = h["holdings"][0]["holding"]["id"].as_u64().unwrap();
    let older = ok(request(
        &app,
        &bt,
        "GET",
        &format!("/api/v1/new-coins/holdings?project_id={project}&before_id={newer_id}"),
        json!(null),
    )
    .await);
    assert_eq!(older["holdings"].as_array().unwrap().len(), 1);
    assert_eq!(older["holdings"][0]["holding"]["id"], holding);
    let wrong_project = ok(request(
        &app,
        &bt,
        "GET",
        "/api/v1/new-coins/holdings?project_id=18446744073709551615",
        json!(null),
    )
    .await);
    assert!(wrong_project["holdings"].as_array().unwrap().is_empty());
    assert_eq!(
        request(
            &app,
            &bt,
            "POST",
            &format!("/api/v1/new-coins/{symbol}/quotes"),
            json!({"side":"buy","quantity":0.1,"idempotency_key":"numeric"})
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    // Each real subscription/settlement pins the original acquisition instant and policy.
    for (i, lock) in [
        json!({"kind":"none"}),
        json!({"kind":"on_purchase"}),
        json!({"kind":"on_listing"}),
        json!({"kind":"weeks","count":3}),
        json!({"kind":"months","count":3}),
        json!({"kind":"fixed","at":1900000000000i64}),
    ]
    .into_iter()
    .enumerate()
    {
        let mut policy = policy();
        policy["lock"] = lock;
        let token_symbol =
            format!("L{i}{}", &Uuid::now_v7().simple().to_string()[24..]).to_uppercase();
        let mut application_input = input.clone();
        application_input["symbol"] = json!(token_symbol);
        application_input["issuance_policy"] = policy.clone();
        application_input["idempotency_key"] = json!(format!("lock-{i}"));
        let a = ok(request(
            &app,
            &ct,
            "POST",
            "/agent/api/v1/coin-applications",
            application_input,
        )
        .await);
        let approved = ok(request(
            &app,
            &admin_token,
            "POST",
            &format!("/admin/api/v1/agent-coin-applications/{}/review", a["id"]),
            json!({"decision":"approve","reason":"lock verification"}),
        )
        .await);
        let pid = approved["project_id"].as_u64().unwrap();
        stage(&app, &ct, pid, "preheat", "subscription").await;
        ok(request(&app, &bt, "POST", &format!("/api/v1/new-coins/{token_symbol}/subscriptions"), json!({"quantity":"1","quote_amount":"0.2","quote_asset_id":quote,"idempotency_key":format!("lock-sub-{pid}")})).await);
        let acquired = chrono::Utc::now() - chrono::Duration::days(150);
        sqlx::query(
            "UPDATE new_coin_subscriptions SET created_at=? WHERE project_id=? AND user_id=?",
        )
        .bind(acquired)
        .bind(pid)
        .bind(b)
        .execute(&pool)
        .await
        .unwrap();
        let sid: u64 = sqlx::query_scalar(
            "SELECT id FROM new_coin_subscriptions WHERE project_id=? AND user_id=?",
        )
        .bind(pid)
        .bind(b)
        .fetch_one(&pool)
        .await
        .unwrap();
        stage(&app, &ct, pid, "subscription", "distribution").await;
        ok(request(&app, &at, "POST", "/agent/api/v1/new-coin-allocations", json!({"subscription_ids":[sid],"percent":"100","reason":"lock verification","idempotency_key":format!("lock-batch-{pid}")})).await);
        let read_path = format!("/api/v1/new-coins/holdings?project_id={pid}");
        let before = ok(request(&app, &bt, "GET", &read_path, json!(null)).await);
        let row = &before["holdings"][0];
        assert_eq!(row["holding"]["policy_json"], policy);
        assert_eq!(row["holding"]["acquired_at"], acquired.timestamp_millis());
        assert_eq!(row["eligible"], matches!(i, 0 | 3 | 4));
        let parsed: exchange_api::modules::new_coin::domain::issuance::IssuancePolicy =
            serde_json::from_value(policy).unwrap();
        assert_eq!(
            row["holding"]["unlock_at"],
            json!(
                parsed
                    .unlock_at(acquired)
                    .unwrap()
                    .map(|v| v.timestamp_millis())
            )
        );
        stage(&app, &ct, pid, "distribution", "purchase").await;
        let purchase = ok(request(&app, &bt, "GET", &read_path, json!(null)).await);
        assert_eq!(purchase["holdings"][0]["eligible"], !matches!(i, 2 | 5));
        stage(&app, &ct, pid, "purchase", "listed").await;
        let listed = ok(request(&app, &bt, "GET", &read_path, json!(null)).await);
        assert_eq!(listed["holdings"][0]["eligible"], i != 5);
        assert_eq!(listed["holdings"][0]["holding"]["status"], "held");
    }
    assert_eq!(
        exchange_api::workers::new_coin_lifecycle::run_once(&state)
            .await
            .unwrap(),
        0
    );
    // Applying the complete migration set again validates immutable checksums and legacy compatibility.
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
}

#[tokio::test]
async fn token_lifecycle_worker_honors_manual_gates_restarts_and_failure_backoff() {
    let _serial = SERIAL.lock().await;
    let Some(pool) = mysql_pool().await else {
        return;
    };
    let agent = create_agent(&pool, "scheduled-token").await;
    let (_, admin, _) = create_admin_with_password(&pool, "sched", "password").await;
    let admin_token = issue_token(
        &test_settings(),
        format!("admin:{admin}"),
        TokenScope::Admin,
        900,
    )
    .unwrap();
    let at = token(agent);
    let state = AppState::new(test_settings()).with_mysql(pool.clone());
    let app = build_router(state.clone());
    let quote=sqlx::query("INSERT INTO assets(symbol,name,precision_scale,status) VALUES (?,'schedule quote',8,'active')")
        .bind(format!("Q{}",&Uuid::now_v7().simple().to_string()[20..]).to_uppercase()).execute(&pool).await.unwrap().last_insert_id();
    let symbol = format!("S{}", &Uuid::now_v7().simple().to_string()[20..]).to_uppercase();
    let mut p = policy();
    let now = chrono::Utc::now().timestamp_millis();
    for (i, key) in ["subscription", "distribution", "purchase", "listing"]
        .iter()
        .enumerate()
    {
        p[key]["at"] = json!(now - 60000 + (i as i64) * 1000);
        p[key]["automatic"] = json!(*key != "distribution");
    }
    let a=ok(request(&app,&at,"POST","/agent/api/v1/coin-applications",json!({"symbol":symbol,"name":"Schedule","precision_scale":8,"total_supply":"100","issue_price":"0.2","quote_asset_id":quote,"listed_at":p["listing"]["at"],"reason":"schedule","idempotency_key":"schedule","issuance_policy":p})).await);
    let approved = ok(request(
        &app,
        &admin_token,
        "POST",
        &format!("/admin/api/v1/agent-coin-applications/{}/review", a["id"]),
        json!({"decision":"approve","reason":"approve"}),
    )
    .await);
    let id = approved["project_id"].as_u64().unwrap();
    let (r1, r2) = tokio::join!(
        exchange_api::workers::new_coin_lifecycle::run_once(&state),
        exchange_api::workers::new_coin_lifecycle::run_once(&state)
    );
    assert_eq!(r1.unwrap() + r2.unwrap(), 1);
    assert_eq!(
        exchange_api::workers::new_coin_lifecycle::run_once(&state)
            .await
            .unwrap(),
        0
    );
    let status: String =
        sqlx::query_scalar("SELECT lifecycle_status FROM new_coin_projects WHERE id=?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "subscription");
    stage(&app, &at, id, "subscription", "distribution").await;
    sqlx::raw_sql(&format!("CREATE TRIGGER token_schedule_fail BEFORE INSERT ON new_coin_operations FOR EACH ROW BEGIN IF NEW.project_id={id} AND NEW.action='new_coin.stage' THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='test schedule rollback'; END IF; END")).execute(&pool).await.unwrap();
    assert_eq!(
        exchange_api::workers::new_coin_lifecycle::run_once(&state)
            .await
            .unwrap(),
        0
    );
    sqlx::raw_sql("DROP TRIGGER token_schedule_fail")
        .execute(&pool)
        .await
        .unwrap();
    let blocked:(String,Option<chrono::DateTime<chrono::Utc>>,Option<String>,bool)=sqlx::query_as("SELECT lifecycle_status,actual_purchase_at,transition_error,next_transition_attempt_at>UTC_TIMESTAMP(6) FROM new_coin_projects WHERE id=?").bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(
        blocked,
        (
            "distribution".into(),
            None,
            Some("transition_blocked".into()),
            true
        )
    );
    assert_eq!(
        exchange_api::workers::new_coin_lifecycle::run_once(&state)
            .await
            .unwrap(),
        0
    );
    sqlx::query(
        "UPDATE new_coin_projects SET next_transition_attempt_at=UTC_TIMESTAMP(6) WHERE id=?",
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        exchange_api::workers::new_coin_lifecycle::run_once(&state)
            .await
            .unwrap(),
        1
    );
    // A fresh worker state resumes the next persisted node without resetting market history.
    let restarted = AppState::new(test_settings()).with_mysql(pool.clone());
    assert_eq!(
        exchange_api::workers::new_coin_lifecycle::run_once(&restarted)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        exchange_api::workers::new_coin_lifecycle::run_once(&restarted)
            .await
            .unwrap(),
        0
    );
    let end:(String,bool,bool,bool)=sqlx::query_as("SELECT lifecycle_status,actual_purchase_at IS NOT NULL,actual_listed_at IS NOT NULL,next_transition_at IS NULL FROM new_coin_projects WHERE id=?").bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(end, ("listed".into(), true, true, true));
    let logs: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM new_coin_operations WHERE project_id=? AND actor_type='scheduler'",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(logs, 3);
}
