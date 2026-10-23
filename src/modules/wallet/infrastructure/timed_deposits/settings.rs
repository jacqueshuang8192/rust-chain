//! 监听节点、资产身份和共享地址的显式管理；写入及审计原子提交。

use super::{audit, lock_network};
use crate::{
    error::{AppError, AppResult},
    infra::secrets::encrypt_secret_field,
    modules::wallet::{
        domain::timed_deposits::{canonical_address, identity_key},
        presentation::timed_deposits::*,
    },
};
use serde_json::json;
use sqlx::{MySql, Pool};

/// 配置页只回显凭据是否存在，不向浏览器或审计输出节点密钥、带密钥路径或密文。
pub(crate) async fn load(pool: &Pool<MySql>) -> AppResult<DepositSettings> {
    Ok(DepositSettings{
        scanners:sqlx::query_as("SELECT network,enabled,endpoint_encrypted IS NOT NULL AS endpoint_configured,api_key_encrypted IS NOT NULL AS api_key_configured,confirmations,revision,last_scan_at,last_error FROM wallet_deposit_scanners ORDER BY network").fetch_all(pool).await?,
        assets:sqlx::query_as("SELECT id,network,asset_symbol,asset_kind,token_address,decimals,enabled,revision FROM wallet_deposit_chain_assets ORDER BY network,asset_symbol").fetch_all(pool).await?,
        addresses:sqlx::query_as("SELECT pool_id,network,address,enabled FROM wallet_deposit_shared_addresses ORDER BY network,pool_id").fetch_all(pool).await?,
    })
}

/// 修改节点先锁网络并验证版本；留空保留原密文，配置变化使旧租约失效，重启不得跳过旧游标。
pub(crate) async fn save_scanner(
    pool: &Pool<MySql>,
    admin_id: u64,
    network: &str,
    key: &str,
    request: &SaveScanner,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    lock_network(&mut tx, network).await?;
    let (revision,endpoint,api_key,enabled,confirmations)=sqlx::query_as::<_,(u64,Option<String>,Option<String>,bool,u32)>("SELECT revision,endpoint_encrypted,api_key_encrypted,enabled,confirmations FROM wallet_deposit_scanners WHERE network=?")
        .bind(network).fetch_one(&mut *tx).await?;
    if revision != request.expected_revision {
        return Err(AppError::Conflict(
            "deposit scanner configuration changed; reload first".into(),
        ));
    }
    let endpoint = encrypt_secret_field(key, request.endpoint.as_deref(), endpoint)?;
    let api_key = encrypt_secret_field(key, request.api_key.as_deref(), api_key)?;
    if request.enabled && endpoint.is_none() {
        return Err(AppError::Validation(
            "a scanner endpoint is required before enabling deposits".into(),
        ));
    }
    sqlx::query("UPDATE wallet_deposit_scanners SET enabled=?,endpoint_encrypted=?,api_key_encrypted=?,confirmations=?,revision=revision+1,last_scan_at=NULL,last_error=NULL,lease_token=NULL,lease_until=NULL WHERE network=?")
        .bind(request.enabled).bind(endpoint).bind(api_key).bind(request.confirmations).bind(network).execute(&mut *tx).await?;
    audit(&mut tx,admin_id,"deposit_scanner.update",0,Some(json!({"network":network,"enabled":enabled,"confirmations":confirmations,"revision":revision})),json!({"network":network,"enabled":request.enabled,"confirmations":request.confirmations,"revision":revision+1}),&request.reason).await?;
    tx.commit().await?;
    Ok(())
}

/// 保存资产映射时验证平台资产存在；已经建立的链身份与 decimals 不允许原地改写，停用仍保留历史监听。
pub(crate) async fn save_asset(
    pool: &Pool<MySql>,
    admin_id: u64,
    request: &SaveChainAsset,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    lock_network(&mut tx, &request.network).await?;
    let asset_id = sqlx::query_scalar::<_, u64>("SELECT id FROM assets WHERE symbol=? FOR SHARE")
        .bind(&request.asset_symbol)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;
    let before=sqlx::query_as::<_,ChainAssetView>("SELECT id,network,asset_symbol,asset_kind,token_address,decimals,enabled,revision FROM wallet_deposit_chain_assets WHERE network=? AND asset_id=? FOR UPDATE").bind(&request.network).bind(asset_id).fetch_optional(&mut *tx).await?;
    let id = if let Some(old) = &before {
        if old.revision != request.expected_revision {
            return Err(AppError::Conflict(
                "chain asset configuration changed; reload first".into(),
            ));
        }
        if old.asset_kind != request.asset_kind
            || old.token_address != request.token_address
            || old.decimals != request.decimals
        {
            return Err(AppError::Conflict(
                "chain identity and decimals are immutable; disable this mapping instead".into(),
            ));
        }
        sqlx::query(
            "UPDATE wallet_deposit_chain_assets SET enabled=?,revision=revision+1 WHERE id=?",
        )
        .bind(request.enabled)
        .bind(old.id)
        .execute(&mut *tx)
        .await?;
        old.id
    } else {
        if request.expected_revision != 0 {
            return Err(AppError::Conflict("chain asset does not exist".into()));
        }
        let identity = identity_key(&[&request.asset_kind, &request.token_address]);
        let collision:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM wallet_deposit_chain_assets WHERE network=? AND identity_key=?)").bind(&request.network).bind(&identity).fetch_one(&mut *tx).await?;
        if collision {
            return Err(AppError::Conflict(
                "chain identity is already mapped to an asset".into(),
            ));
        }
        sqlx::query("INSERT INTO wallet_deposit_chain_assets (network,asset_id,asset_symbol,asset_kind,token_address,identity_key,decimals,enabled) VALUES (?,?,?,?,?,?,?,?)")
            .bind(&request.network).bind(asset_id).bind(&request.asset_symbol).bind(&request.asset_kind).bind(&request.token_address).bind(identity).bind(request.decimals).bind(request.enabled).execute(&mut *tx).await?.last_insert_id()
    };
    sqlx::query("UPDATE wallet_deposit_scanners SET last_scan_at=NULL,revision=revision+1,lease_token=NULL,lease_until=NULL WHERE network=?").bind(&request.network).execute(&mut *tx).await?;
    audit(&mut tx,admin_id,"deposit_chain_asset.save",id,before.as_ref().map(|v|json!(v)),json!({"network":request.network,"asset_symbol":request.asset_symbol,"asset_kind":request.asset_kind,"token_address":request.token_address,"decimals":request.decimals,"enabled":request.enabled}),&request.reason).await?;
    tx.commit().await?;
    Ok(())
}

/// 把未永久分配过的库存显式纳入共享池；网络/地址快照永久保留，停用只禁止新订单，不删除历史归属。
pub(crate) async fn save_address(
    pool: &Pool<MySql>,
    admin_id: u64,
    pool_id: u64,
    request: &SaveSharedAddress,
) -> AppResult<()> {
    let network =
        sqlx::query_scalar::<_, String>("SELECT network FROM deposit_address_pool WHERE id=?")
            .bind(pool_id)
            .fetch_optional(pool)
            .await?
            .ok_or(AppError::NotFound)?;
    let mut tx = pool.begin().await?;
    lock_network(&mut tx, &network).await?;
    let (locked_network, address, status, memo, assigned_user_id) =
        sqlx::query_as::<_, (String, String, String, Option<String>, Option<u64>)>(
            "SELECT network,address,status,memo,assigned_user_id FROM deposit_address_pool WHERE id=? FOR UPDATE",
        )
        .bind(pool_id)
        .fetch_one(&mut *tx)
        .await?;
    if locked_network != network {
        return Err(AppError::Conflict(
            "address network changed; reload first".into(),
        ));
    }
    let before=sqlx::query_as::<_,SharedAddressView>("SELECT pool_id,network,address,enabled FROM wallet_deposit_shared_addresses WHERE pool_id=? FOR UPDATE").bind(pool_id).fetch_optional(&mut *tx).await?;
    if let Some(old) = &before {
        if request.enabled && (status != "available" || assigned_user_id.is_some()) {
            return Err(AppError::Conflict(
                "address inventory must be available".into(),
            ));
        }
        sqlx::query("UPDATE wallet_deposit_shared_addresses SET enabled=? WHERE pool_id=?")
            .bind(request.enabled)
            .bind(old.pool_id)
            .execute(&mut *tx)
            .await?;
    } else {
        if status != "available"
            || assigned_user_id.is_some()
            || memo.as_deref().is_some_and(|m| !m.is_empty())
        {
            return Err(AppError::Conflict(
                "only unused available addresses without a memo can enter this pool".into(),
            ));
        }
        let reclaimed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admin_audit_logs WHERE target_type='deposit_address_pool' AND target_id=? AND action='deposit_address_pool.reclaim')").bind(pool_id.to_string()).fetch_one(&mut *tx).await?;
        if reclaimed {
            return Err(AppError::Conflict(
                "a previously permanent user address cannot enter the shared pool".into(),
            ));
        }
        let address = canonical_address(&network, &address).map_err(AppError::Validation)?;
        let address_key = identity_key(&[&address]);
        sqlx::query("INSERT INTO wallet_deposit_shared_addresses (pool_id,network,address,address_key,enabled) VALUES (?,?,?,?,?)").bind(pool_id).bind(&network).bind(address).bind(address_key).bind(request.enabled).execute(&mut *tx).await?;
        // 新地址在扫描器重新加载库存后才能接单，避免轮询游标跨过它的首笔入账。
        sqlx::query("UPDATE wallet_deposit_scanners SET last_scan_at=NULL,revision=revision+1,lease_token=NULL,lease_until=NULL WHERE network=?").bind(&network).execute(&mut *tx).await?;
    }
    audit(
        &mut tx,
        admin_id,
        "deposit_shared_address.save",
        pool_id,
        before.map(|v| json!(v)),
        json!({"pool_id":pool_id,"network":network,"enabled":request.enabled}),
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}
