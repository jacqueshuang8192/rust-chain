//! 跟随来源只读 Redis 外部最新价；不请求交易所，不把过期或平台自生成行情当作参考。

use super::*;
use crate::modules::market::{
    sanitize_symbol,
    synthetic_follow::{FollowObservation, select_follow_reference},
};
use chrono::{DateTime, Utc};

/// 精确交易对的可用参考证据；缺失/停用/过期作为正常降级原因，存储故障仍返回错误。
pub(crate) struct FollowReferenceRead {
    pub symbol: String,
    pub observation: Option<FollowObservation>,
    pub reason: Option<String>,
}

/// 读取不晚于实际观察时刻的缓存正价格；优先保留已有健康供应商，换供应商由纯状态机重锚。
/// 拒绝自引用及非 active external 引用，未来/过期证据不推进跟随；来源时间保留缓存原值而非伪造成本地时间。
pub(crate) async fn load_follow_reference(
    pool: &Pool<MySql>,
    cache: &crate::modules::market::RedisMarketCache,
    pair_id: u64,
    reference_pair_id: u64,
    now: DateTime<Utc>,
    stale_after_seconds: u32,
    preferred_source: Option<&str>,
) -> AppResult<FollowReferenceRead> {
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT symbol,status,market_type FROM trading_pairs WHERE id=?",
    )
    .bind(reference_pair_id)
    .fetch_optional(pool)
    .await?;
    let (symbol, valid) = row.map_or_else(
        || (String::new(), false),
        |(symbol, status, kind)| {
            (
                symbol,
                pair_id != reference_pair_id && status == "active" && kind == "external",
            )
        },
    );
    let unavailable = |reason: &str| FollowReferenceRead {
        symbol: symbol.clone(),
        observation: None,
        reason: Some(reason.into()),
    };
    if !valid {
        return Ok(unavailable(
            "参考交易对不存在、已停用或不是外部行情，正在独立震荡",
        ));
    }
    let normalized = sanitize_symbol(&symbol);
    let mut candidates = Vec::with_capacity(3);
    if let Some(tick) = cache.latest_price(&symbol).await?
        && matches!(tick.source.as_str(), "bitget" | "htx" | "coinbase")
    {
        candidates.push(FollowObservation {
            event_key: tick.source_version,
            source: tick.source,
            symbol: normalized.clone(),
            price: tick.price,
            observed_at: tick.observed_at,
        });
    }
    let selection = select_follow_reference(
        &symbol,
        &candidates,
        preferred_source,
        now,
        stale_after_seconds,
    );
    let Some(tick) = selection.observation else {
        return Ok(FollowReferenceRead {
            symbol,
            observation: None,
            reason: selection.reason,
        });
    };
    Ok(FollowReferenceRead {
        symbol,
        reason: None,
        observation: Some(tick),
    })
}

#[cfg(test)]
#[path = "../../../../../tests/unit_src/src_market_latest_follow_tests.rs"]
mod tests;
