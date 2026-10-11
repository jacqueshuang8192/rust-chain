//! 读取服务端最新 ticker，不读取或追加历史价格。
use super::*;
use crate::error::{AppError, AppResult};
use redis::{AsyncCommands, aio::ConnectionManager};
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// 保留当前价格的来源和原始时间，消费方按各自业务规则决定是否允许陈旧报价。
pub(crate) struct LatestPrice {
    pub symbol: String,
    pub price: BigDecimal,
    pub source: String,
    pub observed_at: DateTime<Utc>,
    pub source_version: String,
}

/// 最新缓存缺失返回 None；损坏、身份不符或存储精度错误必须显式失败，不能回退 SQL 旧价。
pub(crate) async fn load_latest_price(
    redis: &ConnectionManager,
    symbol: &str,
) -> AppResult<Option<LatestPrice>> {
    #[derive(Deserialize)]
    struct Payload {
        symbol: String,
        last_price: BigDecimal,
        provider: Option<String>,
        source: Option<String>,
        #[serde(with = "crate::time::unix_millis")]
        observed_at: DateTime<Utc>,
    }
    let symbol =
        ValidatedMarketSymbol::from_raw(symbol).map_err(|e| AppError::Validation(e.to_string()))?;
    let payload: Option<String> = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        redis.clone().get(market_ticker_redis_key(symbol.as_str())),
    )
    .await
    .map_err(|_| AppError::Internal("读取最新行情超时".into()))??;
    let Some(payload) = payload else {
        return Ok(None);
    };
    let value: Payload = serde_json::from_str(&payload)
        .map_err(|e| AppError::Validation(format!("invalid latest ticker: {e}")))?;
    if ValidatedMarketSymbol::from_raw(&value.symbol).ok().as_ref() != Some(&symbol)
        || value.last_price <= 0
    {
        return Err(AppError::Validation(
            "invalid latest ticker identity or price".into(),
        ));
    }
    crate::numeric::ensure_amount_storage(&value.last_price, "latest ticker price")?;
    crate::time::ensure_timestamp_storage(&value.observed_at, "latest ticker time")?;
    Ok(Some(LatestPrice {
        symbol: symbol.as_str().into(),
        price: value.last_price,
        source: if value.source.as_deref() == Some("default") {
            "default".into()
        } else {
            value.provider.unwrap_or_else(|| "unknown".into())
        },
        observed_at: value.observed_at,
        source_version: format!("redis:{}", hex::encode(Sha256::digest(payload.as_bytes()))),
    }))
}

impl RedisMarketCache {
    /// 提供最新价只读入口给生成器参考；不暴露连接，不改变来源时间或写入状态。
    pub(crate) async fn latest_price(&self, symbol: &str) -> AppResult<Option<LatestPrice>> {
        load_latest_price(&self.manager, symbol).await
    }
}

#[cfg(test)]
#[path = "../../../../../tests/unit_src/src_market_latest_price_tests.rs"]
mod tests;
