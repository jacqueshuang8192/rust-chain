use super::*;
use crate::modules::wallet::{
    application::timed_deposits as app, presentation::timed_deposits::SaveScanner,
};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};
const ENCRYPTION_KEY: &str = "0123456789abcdef0123456789abcdef";

#[tokio::test]
async fn tron_provider_preserves_query_credentials_when_appending_operation() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/gateway/wallet/getnowblock"))
        .and(matchers::query_param("api_key", "key+/ value"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok":true})))
        .expect(1)
        .mount(&server)
        .await;
    let mut node = node(&server, "tron");
    node.endpoint = format!("{}/gateway?api_key=key%2B%2F%20value", server.uri());
    assert_eq!(
        node.post_operation("/wallet/getnowblock", "/wallet/getnowblock", json!({}))
            .await
            .unwrap(),
        json!({"ok":true})
    );
}

#[test]
fn retry_after_is_bounded_and_supports_http_date() {
    assert_eq!(provider_runtime::retry_after(None), 60);
    assert_eq!(
        provider_runtime::retry_after(Some(&"120".parse().unwrap())),
        120
    );
    assert_eq!(
        provider_runtime::retry_after(Some(&"invalid".parse().unwrap())),
        60
    );
    let date = (Utc::now() + chrono::Duration::seconds(300)).to_rfc2822();
    assert!((298..=300).contains(&provider_runtime::retry_after(Some(&date.parse().unwrap()))));
}

fn provider(id: Option<u64>, endpoint: Option<String>, priority: u32, keys: Value) -> Value {
    json!({"id":id,"name":format!("Channel {priority}"),"enabled":true,"priority":priority,"endpoint":endpoint,"request_interval_ms":100,"keys":keys})
}
fn credential(id: Option<u64>, secret: Option<&str>, priority: u32) -> Value {
    json!({"id":id,"name":format!("Key {priority}"),"enabled":true,"priority":priority,"api_key":secret,"clear_api_key":false,"request_interval_ms":100})
}
async fn save(pool: &Pool<MySql>, admin: u64, providers: Value) -> AppResult<()> {
    let revision: u64 =
        sqlx::query_scalar("SELECT revision FROM wallet_deposit_scanners WHERE network='tron'")
            .fetch_one(pool)
            .await?;
    let request:SaveScanner=serde_json::from_value(json!({"enabled":true,"endpoint":null,"api_key":null,"confirmations":2,"expected_revision":revision,"reason":"provider failover verification","providers":providers})).unwrap();
    app::save_scanner(pool, admin, "tron", ENCRYPTION_KEY, request).await
}

#[tokio::test]
async fn provider_database_migration_failover_cooldown_and_revision() {
    let Ok(url) = std::env::var("DEPOSIT_PROVIDER_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED: DEPOSIT_PROVIDER_TEST_DATABASE_URL required");
        return;
    };
    assert!(url.contains("127.0.0.1") && url.contains("timed_deposits_test_ha"));
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(12)
        .connect(&url)
        .await
        .unwrap();
    let mut before = sqlx::migrate!("./migrations");
    before.migrations = std::borrow::Cow::Owned(
        before
            .migrations
            .iter()
            .filter(|m| m.version < 162)
            .cloned()
            .collect(),
    );
    before.run(&pool).await.unwrap();
    let primary = MockServer::start().await;
    let endpoint =
        crate::infra::secrets::encrypt_secret_field(ENCRYPTION_KEY, Some(&primary.uri()), None)
            .unwrap()
            .unwrap();
    let secret =
        crate::infra::secrets::encrypt_secret_field(ENCRYPTION_KEY, Some("private-primary"), None)
            .unwrap()
            .unwrap();
    sqlx::query("UPDATE wallet_deposit_scanners SET endpoint_encrypted=?,api_key_encrypted=? WHERE network='tron'").bind(&endpoint).bind(&secret).execute(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let migrated:(u64,String,u64,String)=sqlx::query_as("SELECT p.id,p.endpoint_encrypted,k.id,k.api_key_encrypted FROM wallet_deposit_scanner_providers p JOIN wallet_deposit_scanner_keys k ON k.provider_id=p.id WHERE p.network='tron'").fetch_one(&pool).await.unwrap();
    assert_eq!(migrated.1, endpoint);
    assert_eq!(migrated.3, secret);
    let role = sqlx::query(
        "INSERT INTO admin_roles (name,permissions) VALUES ('provider-test-role',JSON_ARRAY('*'))",
    )
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();
    let admin=sqlx::query("INSERT INTO admin_users (username,password_hash,role_id) VALUES ('provider-test-admin','fixture',?)").bind(role).execute(&pool).await.unwrap().last_insert_id();
    let backup = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "120")
                .set_body_string("private-primary response-secret"),
        )
        .mount(&primary)
        .await;
    Mock::given(matchers::header("TRON-PRO-API-KEY", "private-invalid"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&backup)
        .await;
    let times = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = times.clone();
    Mock::given(matchers::header("TRON-PRO-API-KEY","private-backup")).respond_with(move |r:&wiremock::Request| {
        captured.lock().unwrap().push(std::time::Instant::now());
        let body:Value=serde_json::from_slice(&r.body).unwrap();
        let result=if r.url.path()=="/wallet/getnowblock" {json!({"block_header":{"raw_data":{"number":100,"timestamp":Utc::now().timestamp_millis()}}})}
        else if body["num"]==0 {json!({"blockID":"genesis2b6653dc"})}
        else {let n=body["num"].as_u64().unwrap();json!({"blockID":format!("b{n}"),"block_header":{"raw_data":{"number":n,"parentHash":format!("b{}",n-1),"timestamp":Utc::now().timestamp_millis()}},"transactions":[]})};
        ResponseTemplate::new(200).set_body_json(result)
    }).mount(&backup).await;
    save(
        &pool,
        admin,
        json!([
            provider(
                Some(migrated.0),
                None,
                0,
                json!([credential(Some(migrated.2), None, 0)])
            ),
            provider(
                None,
                Some(backup.uri()),
                1,
                json!([
                    credential(None, Some("private-invalid"), 0),
                    credential(None, Some("private-backup"), 1)
                ])
            )
        ]),
    )
    .await
    .unwrap();
    run_once(&pool, Some(ENCRYPTION_KEY)).await.unwrap();
    let (cursor, error): (Json<Cursor>, Option<String>) = sqlx::query_as(
        "SELECT cursor_json,last_error FROM wallet_deposit_scanners WHERE network='tron'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(cursor.height, 99);
    assert!(error.is_none(), "{error:?}");
    assert_eq!(primary.received_requests().await.unwrap().len(), 1);
    let remaining:i64=sqlx::query_scalar("SELECT TIMESTAMPDIFF(SECOND,CURRENT_TIMESTAMP(6),cooldown_until) FROM wallet_deposit_scanner_keys WHERE id=?").bind(migrated.2).fetch_one(&pool).await.unwrap();
    assert!(remaining >= 115);
    // Fresh scan constructs new nodes from DB; cooldown survives between rounds.
    run_once(&pool, Some(ENCRYPTION_KEY)).await.unwrap();
    assert_eq!(primary.received_requests().await.unwrap().len(), 1);
    let snapshots = app::settings(&pool).await.unwrap();
    let scanner = snapshots
        .scanners
        .iter()
        .find(|s| s.network == "tron")
        .unwrap();
    assert!(scanner.scan_fresh);
    assert_eq!(scanner.providers.len(), 2);
    let backup_id = scanner.providers[1].id;
    assert!(scanner.providers[1].keys[0].cooldown_until.is_some());
    let serialized = serde_json::to_string(&snapshots).unwrap();
    for private in [
        "private-",
        "response-secret",
        &endpoint,
        &secret,
        &primary.uri(),
        &backup.uri(),
    ] {
        assert!(!serialized.contains(private), "secret leaked");
    }
    let unchanged = json!([
        provider(
            Some(migrated.0),
            None,
            0,
            json!([credential(Some(migrated.2), None, 0)])
        ),
        provider(
            Some(backup_id),
            None,
            1,
            json!([
                credential(Some(scanner.providers[1].keys[0].id), None, 0),
                credential(Some(scanner.providers[1].keys[1].id), None, 1)
            ])
        )
    ]);
    let cooldown_before: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT cooldown_until FROM wallet_deposit_scanner_keys WHERE id=?")
            .bind(migrated.2)
            .fetch_one(&pool)
            .await
            .unwrap();
    save(&pool, admin, unchanged.clone()).await.unwrap();
    let cooldown_after: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT cooldown_until FROM wallet_deposit_scanner_keys WHERE id=?")
            .bind(migrated.2)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(cooldown_before, cooldown_after);
    let mut invalid = unchanged.clone();
    invalid[1]["keys"][0]["id"] = json!(migrated.2);
    assert!(save(&pool, admin, invalid).await.is_err());
    let persisted: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallet_deposit_scanner_keys")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(persisted, 3);
    // Saving while an HTTP request is in flight fences subsequent requests and cursor writes.
    backup.reset().await;
    Mock::given(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(400)).set_body_json(json!({
            "block_header":{"raw_data":{"number":101,"timestamp":Utc::now().timestamp_millis()}}
        })))
        .mount(&backup)
        .await;
    let scanning_pool = pool.clone();
    let scanning =
        tokio::spawn(async move { run_once(&scanning_pool, Some(ENCRYPTION_KEY)).await });
    tokio::time::timeout(Duration::from_secs(5), async {
        while backup.received_requests().await.unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    save(&pool, admin, unchanged).await.unwrap();
    scanning.await.unwrap().unwrap();
    assert_eq!(backup.received_requests().await.unwrap().len(), 1);
    let fenced: Json<Cursor> =
        sqlx::query_scalar("SELECT cursor_json FROM wallet_deposit_scanners WHERE network='tron'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(fenced.height, cursor.height);
    assert_eq!(fenced.hash, cursor.hash);
    // All channels fail: preserve cursor, close readiness and do not keep hitting cooling endpoints.
    backup.reset().await;
    Mock::given(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&backup)
        .await;
    run_once(&pool, Some(ENCRYPTION_KEY)).await.unwrap();
    let after: Json<Cursor> =
        sqlx::query_scalar("SELECT cursor_json FROM wallet_deposit_scanners WHERE network='tron'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(after.height, cursor.height);
    assert_eq!(after.hash, cursor.hash);
    let calls = backup.received_requests().await.unwrap().len();
    run_once(&pool, Some(ENCRYPTION_KEY)).await.unwrap();
    assert_eq!(backup.received_requests().await.unwrap().len(), calls);
    let error: Option<String> =
        sqlx::query_scalar("SELECT last_error FROM wallet_deposit_scanners WHERE network='tron'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(error.unwrap().contains("不可用"));
    let times = times.lock().unwrap();
    assert!(
        times
            .windows(2)
            .all(|t| t[1].duration_since(t[0]) >= Duration::from_millis(90))
    );
    let audit:Vec<Option<String>>=sqlx::query_scalar("SELECT CAST(after_json AS CHAR) FROM admin_audit_logs WHERE action='deposit_scanner.update'").fetch_all(&pool).await.unwrap();
    assert!(!format!("{audit:?}").contains("private-"));
}
