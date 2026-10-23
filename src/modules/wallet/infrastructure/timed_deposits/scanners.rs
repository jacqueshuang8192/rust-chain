//! 五网络只读区块扫描：持久游标、租约、网络身份校验和逐事件幂等处理。

mod bitcoin;
mod evm;
mod solana;
mod tron;

use super::{orders, settlement};
use crate::{
    error::{AppError, AppResult},
    infra::secrets::decrypt_optional_secret,
    modules::wallet::domain::timed_deposits::{
        ChainAsset, ChainReceipt, canonical_address, raw_amount,
    },
};
use chrono::{DateTime, Utc};
use futures_util::{StreamExt, TryStreamExt, stream};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{MySql, Pool, types::Json};
use std::{collections::HashSet, time::Duration};

const TRANSFER_TOPIC: &str = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

#[derive(sqlx::FromRow)]
struct ScannerConfig {
    network: String,
    endpoint_encrypted: Option<String>,
    api_key_encrypted: Option<String>,
    confirmations: u32,
    revision: u64,
    cursor_json: Option<Json<Cursor>>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Cursor {
    height: u64,
    hash_height: u64,
    hash: String,
}

struct Node {
    network: String,
    endpoint: String,
    api_key: Option<String>,
    client: reqwest::Client,
}
struct Context {
    network: String,
    tip: u64,
    assets: Vec<ChainAsset>,
    addresses: HashSet<String>,
}
struct Block {
    height: u64,
    hash: String,
    parent_hash: String,
    receipts: Vec<ChainReceipt>,
}

/// 各网络独立认领持久租约并并行读取；单网失败记录脱敏原因，其他网络继续，未处理完绝不推进游标。
pub(crate) async fn run_once(pool: &Pool<MySql>, key: Option<&str>) -> AppResult<u64> {
    let configs=sqlx::query_as::<_,ScannerConfig>("SELECT network,endpoint_encrypted,api_key_encrypted,confirmations,revision,cursor_json FROM wallet_deposit_scanners WHERE endpoint_encrypted IS NOT NULL ORDER BY network").fetch_all(pool).await?;
    let results = stream::iter(
        configs
            .into_iter()
            .map(|config| run_network(pool, key, config)),
    )
    .buffer_unordered(5)
    .collect::<Vec<_>>()
    .await;
    let mut count = 0;
    for result in results {
        count += result?;
    }
    Ok(count)
}

async fn run_network(
    pool: &Pool<MySql>,
    key: Option<&str>,
    config: ScannerConfig,
) -> AppResult<u64> {
    let token = uuid::Uuid::now_v7().to_string();
    let claim=sqlx::query("UPDATE wallet_deposit_scanners SET lease_token=?,lease_until=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 60 SECOND) WHERE network=? AND revision=? AND (lease_until IS NULL OR lease_until<CURRENT_TIMESTAMP(6))")
        .bind(&token).bind(&config.network).bind(config.revision).execute(pool).await?;
    if claim.rows_affected() != 1 {
        return Ok(0);
    }
    let result = tokio::time::timeout(Duration::from_secs(45), scan(pool, key, &config))
        .await
        .unwrap_or_else(|_| Err(failure("scanner round timed out")));
    match result {
        Ok((cursor, count, caught_up)) => {
            sqlx::query("UPDATE wallet_deposit_scanners SET cursor_json=?,last_scan_at=IF(?,CURRENT_TIMESTAMP(6),last_scan_at),last_error=NULL,lease_token=NULL,lease_until=NULL WHERE network=? AND revision=? AND lease_token=?")
                .bind(Json(cursor)).bind(caught_up).bind(&config.network).bind(config.revision).bind(&token).execute(pool).await?;
            Ok(count)
        }
        Err(error) => {
            let message = match &error {
                AppError::Internal(s) | AppError::Validation(s) | AppError::Conflict(s) => {
                    s.as_str()
                }
                _ => "deposit scanner persistence failed",
            };
            sqlx::query("UPDATE wallet_deposit_scanners SET last_error=?,lease_token=NULL,lease_until=NULL WHERE network=? AND revision=? AND lease_token=?")
                .bind(message.chars().take(255).collect::<String>()).bind(&config.network).bind(config.revision).bind(&token).execute(pool).await?;
            tracing::warn!(network=%config.network,error=%message,"限时充值监听失败，保留游标等待重试");
            Ok(0)
        }
    }
}

async fn scan(
    pool: &Pool<MySql>,
    key: Option<&str>,
    config: &ScannerConfig,
) -> AppResult<(Cursor, u64, bool)> {
    let key = key.ok_or_else(|| failure("credential encryption key is missing"))?;
    let endpoint = decrypt_optional_secret(config.endpoint_encrypted.as_deref(), key)?
        .ok_or_else(|| failure("scanner endpoint is missing"))?;
    let api_key = decrypt_optional_secret(config.api_key_encrypted.as_deref(), key)?;
    let node = Node {
        network: config.network.clone(),
        endpoint,
        api_key,
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| failure("scanner HTTP client initialization failed"))?,
    };
    let assets=sqlx::query_as::<_,(u64,String,String,String,i32)>("SELECT id,asset_symbol,asset_kind,token_address,decimals FROM wallet_deposit_chain_assets c WHERE network=? AND (enabled=TRUE OR EXISTS(SELECT 1 FROM wallet_deposit_orders o WHERE o.chain_asset_id=c.id)) ORDER BY id")
        .bind(&config.network).fetch_all(pool).await?.into_iter().map(|(id,asset_symbol,asset_kind,token_address,decimals)|ChainAsset{id,asset_symbol,asset_kind,token_address,decimals}).collect::<Vec<_>>();
    let historic:u32=sqlx::query_scalar("SELECT required_confirmations FROM wallet_deposit_orders WHERE network=? AND status<>'credited' ORDER BY required_confirmations DESC LIMIT 1").bind(&config.network).fetch_optional(pool).await?.unwrap_or(1);
    let confirmations = config.confirmations.max(historic);
    let tip = node.head(&assets).await?;
    let safe = tip.saturating_sub(u64::from(confirmations) - 1);
    let context = Context {
        network: config.network.clone(),
        tip,
        assets,
        addresses: orders::watched_addresses(pool, &config.network)
            .await?
            .into_iter()
            .collect(),
    };
    let cursor = if let Some(Json(cursor)) = &config.cursor_json {
        cursor.clone()
    } else {
        if orders::earliest_order_time(pool, &config.network)
            .await?
            .is_some()
        {
            return Err(failure(
                "scanner cursor missing with historical orders; recovery is required",
            ));
        }
        let heights = node.heights(safe.saturating_sub(32), safe).await?;
        let height = *heights
            .last()
            .ok_or_else(|| failure("no confirmed block available for scanner initialization"))?;
        let block = node.block(height, &context).await?;
        return Ok((
            Cursor {
                height: safe,
                hash_height: height,
                hash: block.hash,
            },
            0,
            true,
        ));
    };
    if safe < cursor.hash_height {
        return Err(failure("chain head moved behind the deposit cursor"));
    }
    let check = node
        .block(
            cursor.hash_height,
            &Context {
                network: context.network.clone(),
                tip,
                assets: vec![],
                addresses: HashSet::new(),
            },
        )
        .await?;
    if check.hash != cursor.hash {
        return Err(failure(
            "confirmed chain reorganization detected; reconcile deposits before recovery",
        ));
    }
    let end = safe.min(cursor.height.saturating_add(32));
    if end <= cursor.height {
        return Ok((cursor, 0, true));
    }
    let heights = node.heights(cursor.height + 1, end).await?;
    let mut blocks = stream::iter(
        heights
            .into_iter()
            .map(|height| node.block(height, &context)),
    )
    .buffer_unordered(4)
    .try_collect::<Vec<_>>()
    .await?;
    blocks.sort_by_key(|b| b.height);
    let mut next = cursor;
    let mut count = 0;
    for block in blocks {
        if block.parent_hash != next.hash {
            return Err(failure("deposit scan block continuity failed"));
        }
        for receipt in block.receipts {
            settlement::observe_scanned(pool, &receipt, config.revision).await?;
            count += 1;
        }
        next = Cursor {
            height: block.height,
            hash_height: block.height,
            hash: block.hash,
        };
    }
    next.height = end;
    Ok((next, count, end == safe))
}

impl Node {
    async fn post(&self, path: &str, body: Value) -> AppResult<Value> {
        let url = if path.is_empty() {
            self.endpoint.clone()
        } else {
            format!("{}{path}", self.endpoint.trim_end_matches('/'))
        };
        let mut request = self.client.post(url).json(&body);
        if let Some(key) = &self.api_key {
            request = if self.network == "tron" {
                request.header("TRON-PRO-API-KEY", key)
            } else {
                request.bearer_auth(key)
            };
        }
        let mut response = request
            .send()
            .await
            .map_err(|_| failure("chain node request failed"))?;
        if !response.status().is_success() {
            return Err(failure("chain node returned a non-success HTTP status"));
        }
        if response
            .content_length()
            .is_some_and(|size| size > 64 * 1024 * 1024)
        {
            return Err(failure("chain node response exceeds size limit"));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| failure("chain node response read failed"))?
        {
            if bytes.len().saturating_add(chunk.len()) > 64 * 1024 * 1024 {
                return Err(failure("chain node response exceeds size limit"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| failure("chain node returned invalid JSON"))?;
        if value.get("Error").is_some() {
            return Err(failure("chain node returned an application error"));
        }
        Ok(value)
    }
    async fn rpc(&self, method: &str, params: Value) -> AppResult<Value> {
        let response = self
            .post(
                "",
                json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}),
            )
            .await?;
        if !response["error"].is_null() {
            return Err(failure(&format!("chain RPC method {method} failed")));
        }
        response
            .get("result")
            .cloned()
            .filter(|v| !v.is_null())
            .ok_or_else(|| failure("chain RPC result is missing"))
    }
    async fn head(&self, assets: &[ChainAsset]) -> AppResult<u64> {
        match self.network.as_str() {
            "eth" | "base" => evm::head(self, assets).await,
            "tron" => tron::head(self, assets).await,
            "btc" => bitcoin::head(self).await,
            "solana" => solana::head(self, assets).await,
            _ => Err(failure("unsupported deposit scanner network")),
        }
    }
    async fn heights(&self, from: u64, to: u64) -> AppResult<Vec<u64>> {
        if self.network == "solana" {
            let value = self
                .rpc("getBlocks", json!([from,to,{"commitment":"finalized"}]))
                .await?;
            let heights = array(&value)?
                .iter()
                .map(integer)
                .collect::<AppResult<Vec<_>>>()?;
            if heights.iter().any(|height| *height < from || *height > to)
                || heights.windows(2).any(|pair| pair[0] >= pair[1])
            {
                return Err(failure("Solana block list is out of range or unordered"));
            }
            Ok(heights)
        } else {
            Ok((from..=to).collect())
        }
    }
    async fn block(&self, height: u64, context: &Context) -> AppResult<Block> {
        match self.network.as_str() {
            "eth" | "base" => evm::block(self, height, context).await,
            "tron" => tron::block(self, height, context).await,
            "btc" => bitcoin::block(self, height, context).await,
            "solana" => solana::block(self, height, context).await,
            _ => Err(failure("unsupported deposit scanner network")),
        }
    }
}

fn failure(message: &str) -> AppError {
    AppError::Internal(message.to_owned())
}
fn text(value: &Value) -> AppResult<&str> {
    value
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 255)
        .ok_or_else(|| failure("chain response text is missing or invalid"))
}
fn integer(value: &Value) -> AppResult<u64> {
    value
        .as_u64()
        .ok_or_else(|| failure("chain response integer is invalid"))
}
fn hex_integer(value: &Value) -> AppResult<u64> {
    u64::from_str_radix(
        text(value)?
            .strip_prefix("0x")
            .ok_or_else(|| failure("chain hex prefix is missing"))?,
        16,
    )
    .map_err(|_| failure("chain hex integer is invalid"))
}
fn array(value: &Value) -> AppResult<&Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| failure("chain response array is missing"))
}
fn timestamp(seconds: u64) -> AppResult<DateTime<Utc>> {
    let value = i64::try_from(seconds)
        .ok()
        .and_then(|s| DateTime::from_timestamp(s, 0))
        .ok_or_else(|| failure("chain timestamp is invalid"))?;
    crate::time::ensure_timestamp_storage(&value, "chain timestamp")?;
    Ok(value)
}
fn confirmations(tip: u64, height: u64) -> AppResult<u32> {
    u32::try_from(
        tip.checked_sub(height)
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| failure("chain block exceeds observed head"))?,
    )
    .map_err(|_| failure("chain confirmation count overflow"))
}

fn recent_head(time: DateTime<Utc>) -> AppResult<()> {
    let age = (Utc::now() - time).num_seconds();
    if !(-60..=180).contains(&age) {
        return Err(failure(
            "chain node head timestamp is stale or in the future",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn receipt(
    context: &Context,
    block: &Block,
    time: DateTime<Utc>,
    hash: &str,
    index: u32,
    asset: &ChainAsset,
    address: String,
    amount: bigdecimal::BigDecimal,
) -> AppResult<ChainReceipt> {
    Ok(ChainReceipt {
        network: context.network.clone(),
        tx_hash: hash.to_owned(),
        event_index: index,
        asset_kind: asset.asset_kind.clone(),
        token_address: asset.token_address.clone(),
        address,
        amount,
        block_height: block.height,
        block_hash: block.hash.clone(),
        block_time: time,
        confirmations: confirmations(context.tip, block.height)?,
    })
}

#[cfg(test)]
#[path = "../../../../../tests/unit_src/src_modules_wallet_timed_deposit_scanner_tests.rs"]
mod tests;
