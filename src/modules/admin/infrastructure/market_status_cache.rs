//! 行情状态请求只读取 Redis；数据库配置和 K 线诊断由后台更新短 TTL 元数据。
use super::*;
use crate::modules::{
    admin::{presentation::MarketFeedConfigResponse, service::MarketFeedSymbolObservation},
    market::{ValidatedMarketSymbol, market_ticker_redis_key},
};
use redis::{AsyncCommands, aio::ConnectionManager};
use serde::Serialize;

const STATUS_KEY: &str = "market:admin-status:v1";

#[cfg(test)]
#[path = "../../../../tests/unit_src/src_admin_market_status_cache_tests.rs"]
mod tests;

/// 后台聚合元数据，不包含任何凭据；时间与 TTL 一起界定诊断可用期。
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CachedMarketStatus {
    pub saved_config: Option<MarketFeedConfigResponse>,
    pub kline_gap_count: i64,
    pub kline_recovery_failed_count: i64,
    pub checked_at: DateTime<Utc>,
}

/// 发布已完成的后台诊断；短 TTL 和显式时间校验防止后台停止后继续宣称健康。
pub(crate) async fn store_market_status(
    redis: &ConnectionManager,
    status: &CachedMarketStatus,
) -> AppResult<()> {
    let payload = serde_json::to_string(status).map_err(|e| AppError::Internal(e.to_string()))?;
    let _: () = redis.clone().set_ex(STATUS_KEY, payload, 30).await?;
    Ok(())
}

/// 缓存未就绪/过期返回明确 503，不回源扫描 SQL，也不把缺失诊断伪装为零错误。
pub(crate) async fn load_market_status(redis: &ConnectionManager) -> AppResult<CachedMarketStatus> {
    let payload: Option<String> = redis.clone().get(STATUS_KEY).await?;
    let unavailable = || AppError::Api {
        status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
        code: "MARKET_STATUS_NOT_READY",
        message: "行情状态快照尚未就绪或已过期，请稍后重试".into(),
    };
    let status: CachedMarketStatus = serde_json::from_str(&payload.ok_or_else(unavailable)?)
        .map_err(|e| AppError::Internal(format!("invalid market status: {e}")))?;
    let age = Utc::now()
        .signed_duration_since(status.checked_at)
        .num_seconds();
    if !(0..30).contains(&age) {
        return Err(unavailable());
    }
    Ok(status)
}

/// 原子 MGET 最新行情和接收时间；必须匹配配置的 provider，旧源不能掩盖新源断流。
pub(crate) async fn load_live_market_observations(
    redis: &ConnectionManager,
    symbols: &[String],
    providers: &[String],
) -> AppResult<Vec<MarketFeedSymbolObservation>> {
    #[derive(Deserialize)]
    struct Ticker {
        symbol: String,
        provider: Option<String>,
        #[serde(with = "crate::time::unix_millis")]
        observed_at: DateTime<Utc>,
    }
    let symbols = symbols
        .iter()
        .map(|s| {
            ValidatedMarketSymbol::from_raw(s)
                .map(|s| s.as_str().to_owned())
                .map_err(|e| AppError::Validation(e.to_string()))
        })
        .collect::<AppResult<BTreeSet<_>>>()?;
    if symbols.is_empty() {
        return Ok(Vec::new());
    }
    let keys: Vec<String> = symbols
        .iter()
        .flat_map(|s| {
            [
                market_ticker_redis_key(s),
                format!("market:received:v1:{s}"),
            ]
        })
        .collect();
    let rows: Vec<Option<String>> = redis::cmd("MGET")
        .arg(keys)
        .query_async(&mut redis.clone())
        .await?;
    let mut observations = Vec::new();
    for (symbol, row) in symbols.iter().zip(rows.chunks_exact(2)) {
        let Some(payload) = &row[0] else {
            continue;
        };
        let ticker: Ticker = serde_json::from_str(payload)
            .map_err(|e| AppError::Internal(format!("invalid market ticker: {e}")))?;
        if ticker.symbol != *symbol
            || !ticker
                .provider
                .as_ref()
                .is_some_and(|p| providers.contains(p))
        {
            continue;
        }
        observations.push(MarketFeedSymbolObservation {
            symbol: symbol.clone(),
            observed_at: Some(ticker.observed_at),
            ingested_at: row[1]
                .as_ref()
                .and_then(|v| v.parse().ok())
                .and_then(DateTime::from_timestamp_millis),
        });
    }
    Ok(observations)
}
