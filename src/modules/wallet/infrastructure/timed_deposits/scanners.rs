//! 五网络只读区块扫描：持久游标、租约、网络身份校验和逐事件幂等处理。

mod bitcoin;
mod evm;
mod provider_runtime;
pub(crate) mod recovery;
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

const CONFIG_SELECT: &str = "SELECT network,confirmations,revision,cursor_json,recovery_request_json,backfill_cursor_json,backfill_target_json,COALESCE(backfill_next_at<=CURRENT_TIMESTAMP(6),TRUE) AS backfill_due,COALESCE(last_scan_at>DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 30 SECOND) AND last_error IS NULL,FALSE) AS live_recent,COALESCE(backfill_blocked,FALSE) AS backfill_blocked FROM wallet_deposit_scanners";

#[derive(sqlx::FromRow)]
struct ScannerConfig {
    network: String,
    confirmations: u32,
    revision: u64,
    cursor_json: Option<Json<Cursor>>,
    recovery_request_json: Option<Json<recovery::Request>>,
    backfill_cursor_json: Option<Json<Cursor>>,
    backfill_target_json: Option<Json<Cursor>>,
    backfill_due: bool,
    live_recent: bool,
    backfill_blocked: bool,
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
    gate: Option<provider_runtime::Gate>,
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
    let configs = sqlx::query_as::<_, ScannerConfig>(&format!(
        "{CONFIG_SELECT} WHERE endpoint_encrypted IS NOT NULL ORDER BY network"
    ))
    .fetch_all(pool)
    .await?;
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
    recovery::run_network(pool, key, config).await
}

async fn scan(
    pool: &Pool<MySql>,
    config: &ScannerConfig,
    node: &Node,
    work: recovery::Work,
) -> AppResult<(Cursor, u64, bool)> {
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
    if work == recovery::Work::Recover {
        return recovery::confirmed_head(node, config, &context, safe)
            .await
            .map(|cursor| (cursor, 0, true));
    }
    let selected_cursor = if work == recovery::Work::Backfill {
        &config.backfill_cursor_json
    } else {
        &config.cursor_json
    };
    let safe = if work == recovery::Work::Backfill {
        let target = &config
            .backfill_target_json
            .as_ref()
            .ok_or_else(|| failure("historical scan target missing"))?
            .0;
        recovery::verify_anchor(node, target, &context, safe).await?;
        target.height
    } else {
        safe
    };
    let cursor = if let Some(Json(cursor)) = selected_cursor {
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
    let interval = node.gate.as_ref().map_or(250, |gate| gate.interval());
    let batch = (12_000 / (u64::from(interval) * 4)).clamp(1, 32);
    // Base 原生扫描需分片查询每笔交易，限制区块数为持久节流和整轮超时留出余量。
    let batch =
        if context.network == "base" && context.assets.iter().any(|a| a.asset_kind == "native") {
            batch.min(4)
        } else {
            batch
        };
    let end = safe.min(cursor.height.saturating_add(batch));
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
    // 先验证完整批次和补扫接合点，再处理任何到账，避免后面的区块异常在本批部分入账后才暴露。
    let mut next = cursor;
    for block in &blocks {
        if block.parent_hash != next.hash {
            return Err(failure("deposit scan block continuity failed"));
        }
        next = Cursor {
            height: block.height,
            hash_height: block.height,
            hash: block.hash.clone(),
        };
    }
    next.height = end;
    if work == recovery::Work::Backfill && end == safe {
        let target = &config
            .backfill_target_json
            .as_ref()
            .ok_or_else(|| failure("historical scan target missing"))?
            .0;
        if next.hash != target.hash || next.hash_height != target.hash_height {
            return Err(failure("historical scan did not join recovery anchor"));
        }
    }
    let mut count = 0;
    for block in blocks {
        for receipt in block.receipts {
            settlement::observe_scanned(pool, &receipt, config.revision).await?;
            count += 1;
        }
    }
    Ok((next, count, end == safe))
}

impl Node {
    async fn post(&self, path: &'static str, body: Value) -> AppResult<Value> {
        self.post_operation(path, path, body).await
    }

    // 操作名只接受代码常量；不能从节点 URL、请求参数或上游响应提取诊断文本。
    async fn post_operation(
        &self,
        path: &'static str,
        operation: &'static str,
        body: Value,
    ) -> AppResult<Value> {
        if let Some(gate) = &self.gate {
            gate.acquire().await?;
        }
        let result = self.send_operation(path, operation, body).await;
        if let Some(gate) = &self.gate {
            gate.completed().await?;
        }
        result
    }

    async fn send_operation(
        &self,
        path: &'static str,
        operation: &'static str,
        body: Value,
    ) -> AppResult<Value> {
        let url = if path.is_empty() {
            self.endpoint.clone()
        } else {
            let mut endpoint = url::Url::parse(&self.endpoint)
                .map_err(|_| failure("invalid provider endpoint"))?;
            endpoint.set_path(&format!("{}{path}", endpoint.path().trim_end_matches('/')));
            endpoint.to_string()
        };
        let mut request = self
            .client
            .post(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&body);
        if let Some(key) = &self.api_key {
            request = if self.network == "tron" {
                request.header("TRON-PRO-API-KEY", key)
            } else {
                request.bearer_auth(key)
            };
        }
        let mut response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                let hint = if error.is_timeout() {
                    "请求超时，请检查节点响应与服务器网络"
                } else if error.is_connect() {
                    "连接失败，请检查 DNS、TLS 与服务器网络"
                } else {
                    "请求发送失败，请检查节点地址与凭据格式"
                };
                let message = format!("{} {operation}：{hint}", self.network);
                if let Some(gate) = &self.gate {
                    gate.failed(&message, true, 30).await?;
                }
                return Err(failure(&message));
            }
        };
        if !response.status().is_success() {
            let error = self.http_failure(operation, response.status());
            if let Some(gate) = &self.gate {
                let status = response.status().as_u16();
                let seconds = if status == 429 {
                    provider_runtime::retry_after(
                        response.headers().get(reqwest::header::RETRY_AFTER),
                    )
                } else if status == 401 || status == 403 {
                    300
                } else {
                    30
                };
                gate.failed(&error.to_string(), !matches!(status, 401 | 429), seconds)
                    .await?;
            }
            return Err(error);
        }
        if response
            .content_length()
            .is_some_and(|size| size > 64 * 1024 * 1024)
        {
            return Err(failure("chain node response exceeds size limit"));
        }
        let mut bytes = Vec::new();
        loop {
            let chunk = match response.chunk().await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break,
                Err(_) => {
                    if let Some(gate) = &self.gate {
                        gate.failed("节点响应读取失败，等待备用渠道", true, 30)
                            .await?;
                    }
                    return Err(failure("chain node response read failed"));
                }
            };
            if bytes.len().saturating_add(chunk.len()) > 64 * 1024 * 1024 {
                return Err(failure("chain node response exceeds size limit"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => {
                if let Some(gate) = &self.gate {
                    gate.failed("节点返回无效 JSON，请检查节点接口", true, 60)
                        .await?;
                }
                return Err(failure("chain node returned invalid JSON"));
            }
        };
        if value.get("Error").is_some() {
            if let Some(gate) = &self.gate {
                gate.failed("节点返回应用错误，请检查渠道权限和额度", true, 60)
                    .await?;
            }
            return Err(failure("chain node returned an application error"));
        }
        Ok(value)
    }
    // HTTP 状态与固定提示可持久化；不回显原始错误体、Location、URL 或请求密钥。
    fn http_failure(&self, operation: &str, status: reqwest::StatusCode) -> AppError {
        let trongrid = self.network == "tron"
            && reqwest::Url::parse(&self.endpoint)
                .is_ok_and(|url| url.host_str() == Some("api.trongrid.io"));
        let hint = match status.as_u16() {
            401 => "鉴权失败，请检查 API Key 是否有效及其请求头配置",
            403 if trongrid => "TronGrid 拒绝请求，请检查 API Key 的 IP/接口白名单、请求频率和额度",
            403 => "访问被拒绝，请检查节点权限、IP 白名单、调用额度或网关限制",
            404 | 405 => "接口不存在或请求方式不支持，请检查节点基础地址和接口权限",
            429 => "请求限流或额度耗尽，请检查服务商用量及是否共用 API Key，等待额度恢复",
            500..=599 => "节点或上游网关异常，请检查服务商状态及节点可用性",
            300..=399 => "节点返回重定向，请填写最终 API 地址；监听器不会转发凭据到重定向地址",
            _ => "节点拒绝请求，请检查节点配置和服务商控制台",
        };
        let credential = if self.api_key.is_some()
            || self
                .gate
                .as_ref()
                .is_some_and(|gate| gate.credential_configured)
        {
            "已配置"
        } else {
            "未配置"
        };
        failure(&format!(
            "{} {operation} HTTP {}：{hint}（API Key {credential}）",
            self.network,
            status.as_u16()
        ))
    }

    async fn rpc(&self, method: &'static str, params: Value) -> AppResult<Value> {
        let response = self
            .post_operation(
                "",
                method,
                json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}),
            )
            .await?;
        self.rpc_result(method, &response).await
    }

    /// 批量请求按最多二十五个只读调用分片，每个 HTTP 请求仍经过持久节流；响应按 ID 重排，拒绝漏项和重号。
    /// 任一子调用失败则不返回部分结果，保留渠道冷却与故障切换语义，诊断不包含参数或上游原文。
    async fn rpc_batch(&self, method: &'static str, params: Vec<Value>) -> AppResult<Vec<Value>> {
        let mut results = Vec::with_capacity(params.len());
        for chunk in params.chunks(25) {
            let requests: Vec<_> = chunk.iter().enumerate().map(|(index, params)| {
                json!({"jsonrpc":"2.0","id":index + 1,"method":method,"params":params})
            }).collect();
            let response = self.post_operation("", method, json!(requests)).await?;
            let ordered = (|| -> AppResult<Vec<&Value>> {
                let replies = array(&response)?;
                if replies.len() != chunk.len() {
                    return Err(failure("chain RPC batch response count mismatch"));
                }
                let mut ordered = vec![None; chunk.len()];
                for reply in replies {
                    let index = integer(&reply["id"])?
                        .checked_sub(1)
                        .and_then(|id| usize::try_from(id).ok())
                        .filter(|id| *id < chunk.len())
                        .ok_or_else(|| failure("chain RPC batch response ID is invalid"))?;
                    if reply["jsonrpc"] != "2.0" || ordered[index].replace(reply).is_some() {
                        return Err(failure("chain RPC batch response identity mismatch"));
                    }
                }
                ordered
                    .into_iter()
                    .map(|reply| {
                        reply.ok_or_else(|| failure("chain RPC batch response is incomplete"))
                    })
                    .collect()
            })();
            let ordered = match ordered {
                Ok(ordered) => ordered,
                Err(error) => {
                    if let Some(gate) = &self.gate {
                        gate.failed("RPC 批量响应不完整或标识无效，请检查渠道接口", true, 60)
                            .await?;
                    }
                    return Err(error);
                }
            };
            for reply in ordered {
                results.push(self.rpc_result(method, reply).await?);
            }
        }
        Ok(results)
    }

    /// 普通与批量 RPC 共用错误分类，任一方法错误按原规则冷却渠道；缺失结果不能当成空数据继续扫描。
    async fn rpc_result(&self, method: &'static str, response: &Value) -> AppResult<Value> {
        if !response["error"].is_null() {
            if let Some(gate) = &self.gate {
                gate.failed(
                    &format!("RPC 方法 {method} 失败，请检查渠道权限和额度"),
                    true,
                    60,
                )
                .await?;
            }
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
