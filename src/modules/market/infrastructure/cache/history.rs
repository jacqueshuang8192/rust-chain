//! 有界时间窗口的 Redis 价格历史；与最新快照原子写入，供到期定价而非永久归档。

use super::*;
use crate::error::{AppError, AppResult};
use redis::aio::ConnectionManager;
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// Redis 历史与 SQL 清理共用一天保留期；订单永久证据不受此期限影响。
pub const PRICE_RETENTION_SECONDS: i64 = 86_400;

/// 独立保留精确源时间和价格，公开快照的毫秒序列化不会降低到期窗口精度。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PriceObservation {
    pub symbol: String,
    pub price: BigDecimal,
    pub source: String,
    pub observed_at: DateTime<Utc>,
    pub source_version: String,
}

impl PriceObservation {
    /// 从已校验快照提取定价证据；摘要绑定整个原始快照，不推断缺失的来源。
    pub(super) fn from_entry(entry: &MarketTickerCacheEntry, payload: &str) -> Self {
        Self {
            symbol: entry.symbol.clone(),
            price: entry.last_price.clone(),
            source: if entry.provenance.source == "default" {
                "default".into()
            } else {
                entry
                    .provenance
                    .provider
                    .clone()
                    .unwrap_or_else(|| "redis".into())
            },
            observed_at: entry.observed_at,
            source_version: format!("redis:{}", hex::encode(Sha256::digest(payload.as_bytes()))),
        }
    }
}

/// 历史键与最新键使用相同的符号规范化规则，不接受任意外部 key。
pub(crate) fn price_history_key(symbol: &str) -> String {
    format!("market:price-history:v1:{}", sanitize_symbol(symbol))
}

/// 读取半开到期窗口的第一条已接受报价；缺失返回 None，绝不用当前价格替代。
/// Redis 故障/损坏与空窗口区分，调用方只能对真实空窗口执行等待或人工审核。
pub(crate) async fn load_price_window(
    redis: &ConnectionManager,
    symbol: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> AppResult<Option<PriceObservation>> {
    let symbol =
        ValidatedMarketSymbol::from_raw(symbol).map_err(|e| AppError::Validation(e.to_string()))?;
    let rows: Vec<String> = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        redis::cmd("ZRANGEBYSCORE")
            .arg(price_history_key(symbol.as_str()))
            .arg(start.timestamp_micros())
            .arg(format!("({}", end.timestamp_micros()))
            .arg("LIMIT")
            .arg(0)
            .arg(1)
            .query_async(&mut redis.clone()),
    )
    .await
    .map_err(|_| AppError::Internal("秒合约读取 Redis 历史超时".into()))??;
    rows.first()
        .map(|payload| {
            let observation: PriceObservation = serde_json::from_str(payload)
                .map_err(|e| AppError::Internal(format!("invalid Redis price history: {e}")))?;
            if observation.symbol != symbol.as_str()
                || observation.price <= 0
                || observation.observed_at < start
                || observation.observed_at >= end
            {
                return Err(AppError::Internal(
                    "Redis price history identity/window mismatch".into(),
                ));
            }
            crate::numeric::ensure_amount_storage(&observation.price, "history price")?;
            crate::time::ensure_timestamp_storage(&observation.observed_at, "history observation")?;
            Ok(observation)
        })
        .transpose()
}

#[cfg(test)]
#[path = "../../../../../tests/unit_src/src_market_price_history_tests.rs"]
mod tests;
