//! 短期 ticker 清理与状态元数据刷新；不在实时摄取或 HTTP 请求路径执行。
use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use chrono::{DateTime, Utc};

// 仅清理遗留历史表；新行情不再写入该表。
const PRICE_RETENTION_SECONDS: i64 = 86_400;
use sqlx::{MySql, Pool, QueryBuilder};
use tokio::time::{Duration, MissedTickBehavior};

/// 小批扫描并清理过期 tick；永久保护订单引用、未完成订单窗口和每个源的最后连续性锚点。
/// 游标仅在事务提交后推进；同库多实例由行锁串行化删除，不清空队列、不触碰钱包。
pub async fn prune_price_batch(pool: &Pool<MySql>, after_id: u64) -> AppResult<(u64, u64)> {
    let mut tx = pool.begin().await?;
    let cutoff: DateTime<Utc> =
        sqlx::query_scalar("SELECT TIMESTAMPADD(SECOND, ?, CURRENT_TIMESTAMP(6))")
            .bind(-PRICE_RETENTION_SECONDS)
            .fetch_one(&mut *tx)
            .await?;
    // 只扫描过期候选并用主键游标推进，避免反复检查当天行情或被永久保留证据挡住。
    let candidates: Vec<u64> = sqlx::query_scalar(
        "SELECT id FROM market_price_ticks WHERE id > ? AND observed_at < ? ORDER BY id LIMIT 500",
    )
    .bind(after_id)
    .bind(cutoff.naive_utc())
    .fetch_all(&mut *tx)
    .await?;
    let Some(last) = candidates.last().copied() else {
        tx.commit().await?;
        return Ok((0, 0));
    };
    let mut query = QueryBuilder::<MySql>::new(
        "SELECT ticks.id FROM market_price_ticks ticks WHERE ticks.id IN (",
    );
    let mut separated = query.separated(",");
    for id in &candidates {
        separated.push_bind(id);
    }
    query.push(") AND ticks.observed_at < ").push_bind(cutoff.naive_utc())
        .push(" AND ticks.ingested_at < ").push_bind(cutoff.naive_utc())
        .push(r#" AND NOT EXISTS (SELECT 1 FROM seconds_contract_orders orders WHERE orders.settlement_price_tick_id = ticks.id)
        AND EXISTS (SELECT 1 FROM market_price_ticks newer WHERE newer.symbol = ticks.symbol AND newer.source = ticks.source
            AND (newer.observed_at > ticks.observed_at OR (newer.observed_at = ticks.observed_at AND newer.id > ticks.id)))
        AND NOT EXISTS (SELECT 1 FROM seconds_contract_orders orders
            JOIN trading_pairs pairs ON pairs.id = orders.pair_id
            WHERE orders.status IN ('opened', 'manual_review')
            AND REPLACE(REPLACE(REPLACE(UPPER(pairs.symbol), '-', ''), '/', ''), '_', '') = ticks.symbol
            AND ticks.observed_at >= orders.expires_at AND ticks.observed_at < DATE_ADD(orders.expires_at, INTERVAL 5 SECOND))
        ORDER BY ticks.id FOR UPDATE"#);
    let ids: Vec<u64> = query.build_query_scalar().fetch_all(&mut *tx).await?;
    let deleted = if ids.is_empty() {
        0
    } else {
        let mut delete = QueryBuilder::<MySql>::new("DELETE FROM market_price_ticks WHERE id IN (");
        let mut separated = delete.separated(",");
        for id in ids {
            separated.push_bind(id);
        }
        delete
            .push(")")
            .build()
            .execute(&mut *tx)
            .await?
            .rows_affected()
    };
    tx.commit().await?;
    Ok((last, deleted))
}

/// 独立后台清理循环，失败不推进游标；每轮最多检查 5000 行，不持有无限增长的内存队列。
pub async fn run_retention_loop(state: AppState) -> AppResult<()> {
    let pool = state
        .mysql
        .as_ref()
        .ok_or_else(|| AppError::Internal("price retention requires MySQL".into()))?;
    let mut timer = tokio::time::interval(Duration::from_secs(10));
    timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut cursor = 0;
    loop {
        timer.tick().await;
        for _ in 0..10 {
            match prune_price_batch(pool, cursor).await {
                Ok((next, deleted)) => {
                    cursor = next;
                    if deleted > 0 {
                        tracing::info!(deleted, "过期短期行情已清理，订单凭据与连续性锚点保留");
                    }
                    if next == 0 {
                        break;
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "短期行情清理失败，保留数据等待重试");
                    break;
                }
            }
        }
    }
}

/// 状态刷新与清理分属独立任务；SQL 迟缓不会拖住请求或实时行情，旧缓存最终显式过期。
pub async fn run_status_loop(state: AppState) -> AppResult<()> {
    let pool = state
        .mysql
        .as_ref()
        .ok_or_else(|| AppError::Internal("market status requires MySQL".into()))?;
    let redis = state
        .redis
        .as_ref()
        .ok_or_else(|| AppError::Internal("market status requires Redis".into()))?;
    let mut timer = tokio::time::interval(Duration::from_secs(5));
    timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        timer.tick().await;
        match tokio::time::timeout(
            Duration::from_secs(10),
            crate::modules::admin::application::refresh_market_status_cache(pool, redis),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(%error, "行情状态后台刷新失败"),
            Err(_) => tracing::warn!("行情状态后台刷新超时"),
        }
    }
}
