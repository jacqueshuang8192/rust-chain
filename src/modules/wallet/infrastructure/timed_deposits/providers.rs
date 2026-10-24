//! 多渠道配置的事务持久化；稳定 ID 保护凭据保留语义，网络版本保护并发编辑。
use crate::{
    error::{AppError, AppResult},
    infra::secrets::{decrypt_optional_secret, encrypt_secret_field},
    modules::wallet::presentation::timed_deposits::*,
};
use sqlx::{MySql, Pool, Transaction};

/// 以固定批量查询装配所有网络渠道快照；不读取或返回任何凭据原文。
pub(super) async fn load(pool: &Pool<MySql>, scanners: &mut [ScannerView]) -> AppResult<()> {
    let mut providers = sqlx::query_as::<_, ScannerProviderView>("SELECT id,network,name,priority,enabled,request_interval_ms,cooldown_until,last_error,last_success_at FROM wallet_deposit_scanner_providers ORDER BY priority,id").fetch_all(pool).await?;
    let keys = sqlx::query_as::<_, ScannerKeyView>("SELECT id,provider_id,name,priority,enabled,api_key_encrypted IS NOT NULL AS api_key_configured,request_interval_ms,cooldown_until,last_error,last_success_at FROM wallet_deposit_scanner_keys ORDER BY priority,id").fetch_all(pool).await?;
    for key in keys {
        if let Some(provider) = providers.iter_mut().find(|p| p.id == key.provider_id) {
            provider.keys.push(key);
        }
    }
    for provider in providers {
        if let Some(scanner) = scanners.iter_mut().find(|s| s.network == provider.network) {
            scanner.providers.push(provider);
        }
    }
    Ok(())
}

/// 完整渠道草稿在网络锁内保存；跨网络 ID、跨渠道 Key ID 均拒绝，省略行才删除。
pub(super) async fn save(
    tx: &mut Transaction<'_, MySql>,
    network: &str,
    encryption_key: &str,
    drafts: &[SaveScannerProvider],
) -> AppResult<(Option<String>, Option<String>)> {
    let old: Vec<(u64, String)> = sqlx::query_as("SELECT id,endpoint_encrypted FROM wallet_deposit_scanner_providers WHERE network=? FOR UPDATE")
        .bind(network).fetch_all(&mut **tx).await?;
    let mut retained = Vec::new();
    for draft in drafts {
        let previous = draft.id.and_then(|id| old.iter().find(|row| row.0 == id));
        if draft.id.is_some() && previous.is_none() {
            return Err(AppError::Validation(
                "provider does not belong to this network".into(),
            ));
        }
        let endpoint = encrypt_secret_field(
            encryption_key,
            draft.endpoint.as_deref(),
            previous.map(|r| r.1.clone()),
        )?
        .ok_or_else(|| AppError::Validation("new provider requires an endpoint".into()))?;
        let endpoint_plain =
            decrypt_optional_secret(Some(&endpoint), encryption_key)?.unwrap_or_default();
        let endpoint_changed = previous.is_some_and(|r| r.1 != endpoint);
        let id = if let Some((id, _)) = previous {
            sqlx::query("UPDATE wallet_deposit_scanner_providers SET name=?,priority=?,enabled=?,endpoint_encrypted=?,request_interval_ms=?,cooldown_until=IF(?,NULL,cooldown_until),last_error=IF(?,NULL,last_error),last_success_at=IF(?,NULL,last_success_at),failure_count=IF(?,0,failure_count) WHERE id=?")
                .bind(&draft.name).bind(draft.priority).bind(draft.enabled).bind(&endpoint).bind(draft.request_interval_ms)
                .bind(endpoint_changed).bind(endpoint_changed).bind(endpoint_changed).bind(endpoint_changed).bind(id).execute(&mut **tx).await?;
            *id
        } else {
            sqlx::query("INSERT INTO wallet_deposit_scanner_providers (network,name,priority,enabled,endpoint_encrypted,request_interval_ms) VALUES (?,?,?,?,?,?)")
                .bind(network).bind(&draft.name).bind(draft.priority).bind(draft.enabled).bind(&endpoint).bind(draft.request_interval_ms).execute(&mut **tx).await?.last_insert_id()
        };
        retained.push(id);
        save_keys(
            tx,
            id,
            encryption_key,
            &draft.keys,
            endpoint_plain.contains("{API_KEY}"),
        )
        .await?;
    }
    for (id, _) in old {
        if !retained.contains(&id) {
            sqlx::query("DELETE FROM wallet_deposit_scanner_providers WHERE id=?")
                .bind(id)
                .execute(&mut **tx)
                .await?;
        }
    }
    // Legacy presence columns remain an encrypted projection used by existing admission/settlement contracts.
    let first: Option<(String,Option<String>)> = sqlx::query_as("SELECT p.endpoint_encrypted,k.api_key_encrypted FROM wallet_deposit_scanner_providers p JOIN wallet_deposit_scanner_keys k ON k.provider_id=p.id AND k.enabled=TRUE WHERE p.network=? AND p.enabled=TRUE ORDER BY p.priority,p.id,k.priority,k.id LIMIT 1")
        .bind(network).fetch_optional(&mut **tx).await?;
    Ok(first
        .map(|(endpoint, key)| (Some(endpoint), key))
        .unwrap_or_default())
}

async fn save_keys(
    tx: &mut Transaction<'_, MySql>,
    provider: u64,
    encryption_key: &str,
    drafts: &[SaveScannerKey],
    requires_key: bool,
) -> AppResult<()> {
    let old: Vec<(u64,Option<String>)> = sqlx::query_as("SELECT id,api_key_encrypted FROM wallet_deposit_scanner_keys WHERE provider_id=? FOR UPDATE").bind(provider).fetch_all(&mut **tx).await?;
    let mut retained = Vec::new();
    for draft in drafts {
        let previous = draft.id.and_then(|id| old.iter().find(|r| r.0 == id));
        if draft.id.is_some() && previous.is_none() {
            return Err(AppError::Validation(
                "key does not belong to this provider".into(),
            ));
        }
        let secret = if draft.clear_api_key {
            None
        } else {
            encrypt_secret_field(
                encryption_key,
                draft.api_key.as_deref(),
                previous.and_then(|r| r.1.clone()),
            )?
        };
        if requires_key && draft.enabled && secret.is_none() {
            return Err(AppError::Validation(
                "URL credential template requires an enabled API key".into(),
            ));
        }
        let changed = previous.is_some_and(|r| r.1 != secret);
        let id = if let Some((id, _)) = previous {
            sqlx::query("UPDATE wallet_deposit_scanner_keys SET name=?,priority=?,enabled=?,api_key_encrypted=?,request_interval_ms=?,cooldown_until=IF(?,NULL,cooldown_until),last_error=IF(?,NULL,last_error),last_success_at=IF(?,NULL,last_success_at),failure_count=IF(?,0,failure_count) WHERE id=?")
                .bind(&draft.name).bind(draft.priority).bind(draft.enabled).bind(&secret).bind(draft.request_interval_ms).bind(changed).bind(changed).bind(changed).bind(changed).bind(id).execute(&mut **tx).await?;
            *id
        } else {
            sqlx::query("INSERT INTO wallet_deposit_scanner_keys (provider_id,name,priority,enabled,api_key_encrypted,request_interval_ms) VALUES (?,?,?,?,?,?)")
                .bind(provider).bind(&draft.name).bind(draft.priority).bind(draft.enabled).bind(&secret).bind(draft.request_interval_ms).execute(&mut **tx).await?.last_insert_id()
        };
        retained.push(id);
    }
    for (id, _) in old {
        if !retained.contains(&id) {
            sqlx::query("DELETE FROM wallet_deposit_scanner_keys WHERE id=?")
                .bind(id)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}

/// 兼容旧版单节点保存；已有多渠道时仅允许修改网络开关/确认数，避免覆盖密钥池。
pub(super) async fn save_legacy(
    tx: &mut Transaction<'_, MySql>,
    network: &str,
    request: &SaveScanner,
    endpoint: Option<&str>,
    api_key: Option<&str>,
) -> AppResult<()> {
    let providers: Vec<u64> = sqlx::query_scalar(
        "SELECT id FROM wallet_deposit_scanner_providers WHERE network=? ORDER BY priority,id",
    )
    .bind(network)
    .fetch_all(&mut **tx)
    .await?;
    let secrets_changed = request
        .endpoint
        .as_ref()
        .is_some_and(|s| !s.trim().is_empty())
        || request
            .api_key
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty());
    if providers.len() > 1 && secrets_changed {
        return Err(AppError::Conflict(
            "use provider configuration to update multi-channel credentials".into(),
        ));
    }
    let Some(endpoint) = endpoint else {
        return Ok(());
    };
    let id = if let Some(id) = providers.first() {
        *id
    } else {
        sqlx::query("INSERT INTO wallet_deposit_scanner_providers (network,name,endpoint_encrypted) VALUES (?,'默认渠道',?)").bind(network).bind(endpoint).execute(&mut **tx).await?.last_insert_id()
    };
    let keys: Vec<u64> =
        sqlx::query_scalar("SELECT id FROM wallet_deposit_scanner_keys WHERE provider_id=?")
            .bind(id)
            .fetch_all(&mut **tx)
            .await?;
    if keys.len() > 1 && secrets_changed {
        return Err(AppError::Conflict(
            "use provider configuration to update multiple keys".into(),
        ));
    }
    if keys.is_empty() {
        sqlx::query("INSERT INTO wallet_deposit_scanner_keys (provider_id,name,api_key_encrypted) VALUES (?,'默认凭据',?)").bind(id).bind(api_key).execute(&mut **tx).await?;
    }
    if secrets_changed {
        sqlx::query("UPDATE wallet_deposit_scanner_providers SET endpoint_encrypted=?,cooldown_until=NULL,last_error=NULL,last_success_at=NULL,failure_count=0 WHERE id=?").bind(endpoint).bind(id).execute(&mut **tx).await?;
        sqlx::query("UPDATE wallet_deposit_scanner_keys SET api_key_encrypted=?,cooldown_until=NULL,last_error=NULL,last_success_at=NULL,failure_count=0 WHERE provider_id=?").bind(api_key).bind(id).execute(&mut **tx).await?;
    }
    Ok(())
}
