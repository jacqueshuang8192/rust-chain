use super::*;
use crate::modules::wallet::{
    application::timed_deposits as app, domain::rpc_presets,
    presentation::timed_deposits::SaveScanner,
};
const KEY: &str = "0123456789abcdef0123456789abcdef";

fn draft(
    id: Option<u64>,
    preset: Option<&str>,
    endpoint: Option<&str>,
    key_id: Option<u64>,
    secret: Option<&str>,
) -> Value {
    json!({"id":id,"preset_id":preset,"name":"Preset fixture","priority":0,"enabled":true,"endpoint":endpoint,"request_interval_ms":500,
        "keys":[{"id":key_id,"name":"Anonymous fixture","priority":0,"enabled":true,"api_key":secret,"clear_api_key":false,"request_interval_ms":500}]})
}
async fn revision(pool: &Pool<MySql>, network: &str) -> u64 {
    sqlx::query_scalar("SELECT revision FROM wallet_deposit_scanners WHERE network=?")
        .bind(network)
        .fetch_one(pool)
        .await
        .unwrap()
}
async fn save(pool: &Pool<MySql>, admin: u64, network: &str, providers: Value) -> AppResult<()> {
    let request: SaveScanner = serde_json::from_value(json!({"enabled":false,"endpoint":null,"api_key":null,"confirmations":12,
        "expected_revision":revision(pool,network).await,"reason":"verify public preset","providers":providers})).unwrap();
    app::save_scanner(pool, admin, network, KEY, request).await
}
async fn snapshot(pool: &Pool<MySql>, id: u64) -> Value {
    let result: Json<Value> = sqlx::query_scalar("SELECT JSON_OBJECT('endpoint',p.endpoint_encrypted,'preset',p.preset_id,'cooldown',p.cooldown_until,'error',p.last_error,'next',p.next_request_at,'key_id',k.id,'secret',k.api_key_encrypted,'key_cooldown',k.cooldown_until,'key_error',k.last_error,'key_next',k.next_request_at,'limit',k.rpc_batch_limit) FROM wallet_deposit_scanner_providers p JOIN wallet_deposit_scanner_keys k ON k.provider_id=p.id WHERE p.id=?")
        .bind(id).fetch_one(pool).await.unwrap();
    result.0
}

#[test]
fn public_catalog_covers_every_network_without_keys_and_keeps_optional_nodes_distinct() {
    let mut ids = HashSet::new();
    for preset in rpc_presets::PRESETS {
        assert!(ids.insert(preset.id));
        assert!(preset.endpoint.starts_with("https://"));
        assert!(!preset.endpoint.contains(['?', '{', '@']));
        assert!(preset.source.starts_with("https://"));
        assert!((100..=2000).contains(&preset.request_interval_ms));
        assert_eq!(preset.base_full_blocks, preset.network == "base");
        assert!(rpc_presets::find(preset.network, preset.id).is_some());
        assert!(rpc_presets::find("unsupported", preset.id).is_none());
    }
    for network in ["eth", "base", "tron", "btc", "solana"] {
        assert!(
            rpc_presets::PRESETS
                .iter()
                .any(|p| p.network == network && p.recommended)
        );
    }
    assert!(
        !rpc_presets::find("base", "base-official")
            .unwrap()
            .recommended
    );
    assert!(
        !rpc_presets::find("solana", "solana-official")
            .unwrap()
            .recommended
    );
}

#[tokio::test]
async fn public_presets_migrate_preserve_cooldowns_isolate_credentials_and_fence_saves() {
    let Ok(url) = std::env::var("DEPOSIT_PRESET_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED: isolated DEPOSIT_PRESET_TEST_DATABASE_URL required");
        return;
    };
    assert!(url.contains("127.0.0.1") && url.ends_with("timed_deposits_test_presets"));
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(8)
        .connect(&url)
        .await
        .unwrap();
    let mut old = sqlx::migrate!("./migrations");
    old.migrations = std::borrow::Cow::Owned(
        old.migrations
            .iter()
            .filter(|m| m.version < 168)
            .cloned()
            .collect(),
    );
    old.run(&pool).await.unwrap();
    let endpoint =
        crate::infra::secrets::encrypt_secret_field(KEY, Some("https://private.example/rpc"), None)
            .unwrap()
            .unwrap();
    let secret =
        crate::infra::secrets::encrypt_secret_field(KEY, Some("fixture-private-key"), None)
            .unwrap()
            .unwrap();
    let id = sqlx::query("INSERT INTO wallet_deposit_scanner_providers(network,name,endpoint_encrypted,cooldown_until,last_error,next_request_at) VALUES ('base','custom',?,DATE_ADD(NOW(6),INTERVAL 1 HOUR),'provider cooling',DATE_ADD(NOW(6),INTERVAL 1 SECOND))")
        .bind(&endpoint).execute(&pool).await.unwrap().last_insert_id();
    let key_id = sqlx::query("INSERT INTO wallet_deposit_scanner_keys(provider_id,name,api_key_encrypted,cooldown_until,last_error,rpc_batch_limit) VALUES (?,'original key',?,DATE_ADD(NOW(6),INTERVAL 2 HOUR),'429',3)")
        .bind(id).bind(&secret).execute(&pool).await.unwrap().last_insert_id();
    sqlx::query("UPDATE wallet_deposit_scanners SET endpoint_encrypted=?,api_key_encrypted=?,cursor_json=JSON_OBJECT('height',42,'hash','block42') WHERE network='base'").bind(&endpoint).bind(&secret).execute(&pool).await.unwrap();
    sqlx::query("UPDATE wallet_deposit_scanner_streams SET cursor_json=JSON_OBJECT('height',40,'hash','block40') WHERE network='base'").execute(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let migrated = snapshot(&pool, id).await;
    assert_eq!(migrated["endpoint"], endpoint);
    assert_eq!(migrated["secret"], secret);
    assert_eq!(migrated["limit"], 3);
    assert_eq!(migrated["key_id"], key_id);
    assert!(migrated["preset"].is_null());
    assert!(!migrated["cooldown"].is_null());
    assert!(!migrated["key_cooldown"].is_null());
    let role = sqlx::query(
        "INSERT INTO admin_roles(name,permissions) VALUES ('preset-fixture',JSON_ARRAY('*'))",
    )
    .execute(&pool)
    .await
    .unwrap()
    .last_insert_id();
    let admin = sqlx::query("INSERT INTO admin_users(username,password_hash,role_id) VALUES ('preset-fixture','fixture',?)").bind(role).execute(&pool).await.unwrap().last_insert_id();
    let custom = draft(Some(id), None, None, Some(key_id), None);
    save(&pool, admin, "base", json!([custom])).await.unwrap();
    assert_eq!(snapshot(&pool, id).await, migrated);
    let base_revision = revision(&pool, "base").await;
    let public = draft(Some(id), Some("base-publicnode"), None, Some(key_id), None);
    for invalid in [
        draft(Some(id), Some("eth-drpc"), None, Some(key_id), None),
        draft(Some(id), Some("unknown"), None, Some(key_id), None),
        draft(
            Some(id),
            Some("base-publicnode"),
            Some("https://forged.example"),
            Some(key_id),
            None,
        ),
        draft(
            Some(id),
            Some("base-publicnode"),
            None,
            Some(key_id),
            Some("must-not-leak"),
        ),
        draft(None, Some("base-publicnode"), None, Some(key_id), None),
    ] {
        assert!(save(&pool, admin, "base", json!([invalid])).await.is_err());
        assert_eq!(snapshot(&pool, id).await, migrated);
        assert_eq!(revision(&pool, "base").await, base_revision);
    }
    let mut many_keys = public.clone();
    many_keys["keys"]
        .as_array_mut()
        .unwrap()
        .push(draft(None, None, None, None, None)["keys"][0].clone());
    assert!(
        save(&pool, admin, "base", json!([many_keys]))
            .await
            .is_err()
    );
    // Audit failure rolls back the endpoint change and secret clearing together.
    assert!(
        save(&pool, u64::MAX, "base", json!([public]))
            .await
            .is_err()
    );
    assert_eq!(snapshot(&pool, id).await, migrated);
    save(&pool, admin, "base", json!([public])).await.unwrap();
    let converted = snapshot(&pool, id).await;
    assert_eq!(converted["preset"], "base-publicnode");
    assert!(converted["secret"].is_null());
    assert_eq!(converted["limit"], 25);
    assert_eq!(converted["key_id"], key_id);
    assert_eq!(
        decrypt_optional_secret(converted["endpoint"].as_str(), KEY)
            .unwrap()
            .unwrap(),
        "https://base-rpc.publicnode.com"
    );
    let projected: Option<String> = sqlx::query_scalar(
        "SELECT api_key_encrypted FROM wallet_deposit_scanners WHERE network='base'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(projected.is_none());
    sqlx::query("UPDATE wallet_deposit_scanner_providers SET cooldown_until=DATE_ADD(NOW(6),INTERVAL 1 HOUR),last_error='unavailable' WHERE id=?").bind(id).execute(&pool).await.unwrap();
    sqlx::query("UPDATE wallet_deposit_scanner_keys SET cooldown_until=DATE_ADD(NOW(6),INTERVAL 1 HOUR),last_error='429',rpc_batch_limit=3 WHERE id=?").bind(key_id).execute(&pool).await.unwrap();
    let cooling = snapshot(&pool, id).await;
    // Both explicit same-preset saves and older clients' omitted fields preserve identity and cooldown.
    for request in [public.clone(), custom.clone()] {
        save(&pool, admin, "base", json!([request])).await.unwrap();
        assert_eq!(snapshot(&pool, id).await, cooling);
    }
    let stale: SaveScanner = serde_json::from_value(json!({"enabled":true,"endpoint":null,"api_key":null,"confirmations":12,"expected_revision":base_revision,"reason":"stale draft","providers":[public]})).unwrap();
    assert!(matches!(
        app::save_scanner(&pool, admin, "base", KEY, stale).await,
        Err(AppError::Conflict(_))
    ));
    assert_eq!(snapshot(&pool, id).await, cooling);
    save(
        &pool,
        admin,
        "base",
        json!([draft(Some(id), Some("base-drpc"), None, Some(key_id), None)]),
    )
    .await
    .unwrap();
    assert_eq!(snapshot(&pool, id).await["preset"], "base-drpc");
    // Explicit custom selection preserves the old URL if left blank, but removes its capability profile.
    save(
        &pool,
        admin,
        "base",
        json!([draft(Some(id), Some(""), None, Some(key_id), None)]),
    )
    .await
    .unwrap();
    assert!(snapshot(&pool, id).await["preset"].is_null());
    save(&pool, admin, "base", json!([public])).await.unwrap();
    save(
        &pool,
        admin,
        "base",
        json!([draft(
            Some(id),
            None,
            Some("https://custom.example"),
            Some(key_id),
            Some("new-fixture-key")
        )]),
    )
    .await
    .unwrap();
    assert!(snapshot(&pool, id).await["preset"].is_null());
    save(&pool, admin, "base", json!([public])).await.unwrap();
    let legacy: SaveScanner = serde_json::from_value(json!({"enabled":false,"endpoint":null,"api_key":"legacy-fixture-key","confirmations":12,"expected_revision":revision(&pool,"base").await,"reason":"legacy explicit credential"})).unwrap();
    app::save_scanner(&pool, admin, "base", KEY, legacy)
        .await
        .unwrap();
    assert!(snapshot(&pool, id).await["preset"].is_null());
    for network in ["eth", "base", "tron", "btc", "solana"] {
        let rows: Vec<_> = rpc_presets::PRESETS
            .iter()
            .filter(|p| p.network == network && p.recommended)
            .map(|p| draft(None, Some(p.id), None, None, None))
            .collect();
        save(&pool, admin, network, json!(rows)).await.unwrap();
    }
    let settings = app::settings(&pool).await.unwrap();
    assert_eq!(settings.rpc_presets.len(), rpc_presets::PRESETS.len());
    for scanner in &settings.scanners {
        assert!(!scanner.enabled);
        assert!(!scanner.scan_fresh);
        for provider in &scanner.providers {
            assert!(provider.preset_id.is_some());
            assert_eq!(provider.keys.len(), 1);
            assert!(!provider.keys[0].api_key_configured);
        }
    }
    let cursor: Json<Value> =
        sqlx::query_scalar("SELECT cursor_json FROM wallet_deposit_scanners WHERE network='base'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(cursor["height"], 42);
    let progress: Vec<Json<Value>> = sqlx::query_scalar(
        "SELECT cursor_json FROM wallet_deposit_scanner_streams WHERE network='base'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(progress.iter().all(|p| p["height"] == 40));
    let audit: Vec<Json<Value>> = sqlx::query_scalar(
        "SELECT after_json FROM admin_audit_logs WHERE action='deposit_scanner.update'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert!(
        audit
            .iter()
            .any(|a| a["provider_presets"][0]["preset_id"] == "base-publicnode")
    );
    for serialized in [
        serde_json::to_string(&settings).unwrap(),
        serde_json::to_string(&audit).unwrap(),
    ] {
        for private in [
            &endpoint,
            &secret,
            "fixture-private-key",
            "new-fixture-key",
            "legacy-fixture-key",
            "private.example",
            "custom.example",
        ] {
            assert!(!serialized.contains(private));
        }
    }
}
