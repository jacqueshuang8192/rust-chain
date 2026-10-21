//! 秒合约到期缓存取价结算 worker。
//!
//! 普通 opened 订单到期即读取 Redis 当前报价，不等待历史窗口，也不按观察时间年龄拒绝。
//! 原始报价时间、缓存摘要、资金流水和订单终态原子提交；已提交重放不会重新选价或派奖。
//! 缓存不可用或事务失败按一秒重试，不伪造价格。保留显式历史入口供旧证据回归，不用于生产自动调度。

use crate::{
    error::{AppError, AppResult},
    modules::{
        events::{EventBroadcastHub, EventBroadcastMessage},
        seconds_contract::{
            infrastructure,
            repository::{
                SecondsContractSettlementExceptionWrite, SecondsContractWalletLedgerWrite,
            },
            service::{
                SETTLEMENT_PRICE_WINDOW_SECONDS as EVENT_PRICE_WINDOW_SECONDS,
                price_step_for_precision, resolve_settlement_outcome,
                seconds_contract_payout_amount, settlement_result_from_prices,
            },
        },
    },
    state::AppState,
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream};
use redis::aio::ConnectionManager;
use serde_json::json;
use sqlx::{MySql, Pool};
use std::env;
use tokio::time::{Duration, MissedTickBehavior, interval};
use tracing::{error, info, warn};

/// 事件时点价格窗口长度；右边界不包含在可选集合中。
pub const SETTLEMENT_PRICE_WINDOW_SECONDS: i64 = EVENT_PRICE_WINDOW_SECONDS;

/// 缺少事件时间快照时默认最多等待五分钟，超时后转人工审核。
pub const DEFAULT_MAX_SNAPSHOT_WAIT_SECONDS: u64 = 300;

const MAX_CONFIGURED_SNAPSHOT_WAIT_SECONDS: u64 = 86_400;
const MISSING_SETTLEMENT_SNAPSHOT_FAILURE_CODE: &str = "missing_settlement_snapshot";

/// 秒合约结算 worker 的无状态入口。
pub struct SecondsContractSettlementWorker;

/// 结算 worker 的启动参数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecondsContractSettlementWorkerConfig {
    /// 是否启用自动结算。
    pub enabled: bool,
    /// 扫描周期秒数。
    pub interval_seconds: u64,
    /// 单轮订单上限。
    pub batch_limit: u32,
    /// 事件窗口关闭后继续等待历史快照的最大秒数。
    pub max_snapshot_wait_seconds: u64,
}

impl SecondsContractSettlementWorkerConfig {
    /// 从环境变量读取配置，非法值回退到保守默认值。
    pub fn from_env() -> Self {
        Self {
            enabled: env_bool("SECONDS_CONTRACT_SETTLEMENT_ENABLED", true),
            interval_seconds: env_u64("SECONDS_CONTRACT_SETTLEMENT_INTERVAL_SECONDS", 1),
            batch_limit: env_u32("SECONDS_CONTRACT_SETTLEMENT_BATCH_LIMIT", 100),
            max_snapshot_wait_seconds: normalize_max_snapshot_wait_seconds(env_u64(
                "SECONDS_CONTRACT_SETTLEMENT_MAX_SNAPSHOT_WAIT_SECONDS",
                DEFAULT_MAX_SNAPSHOT_WAIT_SECONDS,
            )),
        }
    }
}

impl SecondsContractSettlementWorker {
    /// 执行一轮结算；MySQL 时钟确认到期，到期后立即读取 Redis 最新价。
    pub async fn run_once(
        &self,
        state: &AppState,
        now: DateTime<Utc>,
        limit: u32,
    ) -> AppResult<SecondsContractSettlementSummary> {
        run_once(state, now, limit).await
    }
}

/// 单轮结算统计。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SecondsContractSettlementSummary {
    /// 已扫描订单数。
    pub scanned: u32,
    /// 成功结算数。
    pub settled: u32,
    /// 因窗口未关闭、缺历史或并发终态而保持 pending 的数量。
    pub skipped: u32,
    /// 数据或资金异常数量。
    pub failed: u32,
    /// 因快照缺失超过最大等待时长而首次转入人工审核的数量。
    pub manual_review: u32,
}

#[derive(Debug, sqlx::FromRow)]
struct DueSecondsContractOrder {
    order_id: u64,
}

/// 提交后才可广播的结算事件快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecondsContractSettlementEvent {
    user_id: u64,
    order_id: u64,
    product_id: u64,
    pair_id: u64,
    stake_asset: u64,
    direction: String,
    stake_amount: BigDecimal,
    payout_amount: BigDecimal,
    settlement_price: BigDecimal,
    settlement_price_tick_id: Option<u64>,
    settlement_price_source: String,
    settlement_price_observed_at: DateTime<Utc>,
    settlement_price_generation: Option<u64>,
    settlement_price_version: String,
    result: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SettlementOutcome {
    Settled(Box<SecondsContractSettlementEvent>),
    ManualReview,
    Pending,
    Skipped,
}

#[derive(Clone, Copy)]
enum SettlementPriceSource<'a> {
    History,
    Cache(&'a ConnectionManager),
}

impl SettlementPriceSource<'_> {
    fn eligibility_delay(self) -> i64 {
        match self {
            Self::History => SETTLEMENT_PRICE_WINDOW_SECONDS,
            Self::Cache(_) => 0,
        }
    }

    fn retry_seconds(self) -> i64 {
        match self {
            Self::History => 60,
            Self::Cache(_) => 1,
        }
    }
}

/// 比较开仓价与结算价得出胜负；保留公开入口供既有调用方和测试复用。
pub fn seconds_contract_settlement_result(
    direction: &str,
    entry_price: &BigDecimal,
    exit_price: &BigDecimal,
) -> AppResult<&'static str> {
    settlement_result_from_prices(direction, entry_price, exit_price)
}

/// 从应用状态取得 MySQL 和 Redis 执行一轮到期结算；数据库时钟确认到期，缓存提供实际价格。
pub async fn run_once(
    state: &AppState,
    _now: DateTime<Utc>,
    limit: u32,
) -> AppResult<SecondsContractSettlementSummary> {
    let pool = state.mysql.as_ref().ok_or_else(|| {
        AppError::Internal("mysql pool is required for seconds contract settlement".to_owned())
    })?;
    let now = database_now(pool).await?;
    let redis = state.redis.as_ref().ok_or_else(|| {
        AppError::Internal("redis is required for seconds contract settlement".to_owned())
    })?;
    run_once_with_cached_prices(pool, redis, state.event_broadcast_hub.as_ref(), now, limit).await
}

/// 显式历史证据兼容入口；只用于旧事件时间规则回归，生产调度使用 Redis 入口。
pub async fn run_once_with_pool(
    pool: &Pool<MySql>,
    now: DateTime<Utc>,
    limit: u32,
) -> AppResult<SecondsContractSettlementSummary> {
    run_once_with_pool_and_max_wait(pool, now, limit, DEFAULT_MAX_SNAPSHOT_WAIT_SECONDS).await
}

/// 在显式 MySQL 池上使用调用方指定的快照最大等待时长执行一轮，供边界与重启集成测试使用。
/// 等待时长夹在 1 秒到 24 小时之间，避免误配为 0 导致窗口一关闭就大量转人工审核。
pub async fn run_once_with_pool_and_max_wait(
    pool: &Pool<MySql>,
    now: DateTime<Utc>,
    limit: u32,
    max_snapshot_wait_seconds: u64,
) -> AppResult<SecondsContractSettlementSummary> {
    run_once_with_broadcast_and_max_wait(
        pool,
        None,
        now,
        limit,
        normalize_max_snapshot_wait_seconds(max_snapshot_wait_seconds),
    )
    .await
}

/// 在显式 MySQL 与 Redis 上执行到期缓存结算，供独立调度和集成回归使用。
pub async fn run_once_with_dependencies(
    pool: &Pool<MySql>,
    redis: &ConnectionManager,
    now: DateTime<Utc>,
    limit: u32,
) -> AppResult<SecondsContractSettlementSummary> {
    run_once_with_cached_prices(pool, redis, None, now, limit).await
}

/// 读取 Redis 最新价，提交后只向原用户发布一次事件。
/// 不等待历史窗口；缺失、损坏和依赖错误一秒后重试，不猜测价格。
pub async fn run_once_with_cached_prices(
    pool: &Pool<MySql>,
    redis: &ConnectionManager,
    hub: Option<&EventBroadcastHub>,
    now: DateTime<Utc>,
    limit: u32,
) -> AppResult<SecondsContractSettlementSummary> {
    run_once_with_price_source(
        pool,
        hub,
        now,
        limit,
        DEFAULT_MAX_SNAPSHOT_WAIT_SECONDS,
        SettlementPriceSource::Cache(redis),
    )
    .await
}

/// 扫描并逐单结算；每单独立事务，缺历史在默认上限内退避，超龄则转人工审核且不改变资金。
pub async fn run_once_with_broadcast(
    pool: &Pool<MySql>,
    hub: Option<&EventBroadcastHub>,
    now: DateTime<Utc>,
    limit: u32,
) -> AppResult<SecondsContractSettlementSummary> {
    run_once_with_broadcast_and_max_wait(pool, hub, now, limit, DEFAULT_MAX_SNAPSHOT_WAIT_SECONDS)
        .await
}

/// 扫描并逐单结算；每单独立事务，缺历史在上限内退避，超龄则原子转人工审核。
async fn run_once_with_broadcast_and_max_wait(
    pool: &Pool<MySql>,
    hub: Option<&EventBroadcastHub>,
    now: DateTime<Utc>,
    limit: u32,
    max_snapshot_wait_seconds: u64,
) -> AppResult<SecondsContractSettlementSummary> {
    run_once_with_price_source(
        pool,
        hub,
        now,
        limit,
        max_snapshot_wait_seconds,
        SettlementPriceSource::History,
    )
    .await
}

async fn run_once_with_price_source(
    pool: &Pool<MySql>,
    hub: Option<&EventBroadcastHub>,
    now: DateTime<Utc>,
    limit: u32,
    max_snapshot_wait_seconds: u64,
    price_source: SettlementPriceSource<'_>,
) -> AppResult<SecondsContractSettlementSummary> {
    let rows = fetch_due_orders(
        pool,
        now,
        seconds_contract_settlement_scan_limit(limit),
        price_source.eligibility_delay(),
    )
    .await?;
    let mut summary = SecondsContractSettlementSummary::default();

    // 限制并发，给请求和其他资金任务保留连接；历史兼容入口保留逐单顺序。
    let concurrency = match price_source {
        SettlementPriceSource::Cache(_) => 4,
        SettlementPriceSource::History => 1,
    };
    let mut attempts = stream::iter(rows)
        .map(|row| async move {
            let result = settle_order_by_id(
                pool,
                row.order_id,
                now,
                max_snapshot_wait_seconds,
                price_source,
            )
            .await;
            (row.order_id, result)
        })
        .buffer_unordered(concurrency);
    let mut retry_error = None;
    while let Some((order_id, result)) = attempts.next().await {
        summary.scanned += 1;
        let retry = match result {
            Ok(SettlementOutcome::Settled(event)) => {
                summary.settled += 1;
                publish_settlement_event(hub, &event);
                false
            }
            Ok(SettlementOutcome::ManualReview) => {
                summary.manual_review += 1;
                false
            }
            Ok(SettlementOutcome::Pending) => {
                summary.skipped += 1;
                true
            }
            Ok(SettlementOutcome::Skipped) => {
                summary.skipped += 1;
                false
            }
            Err(error) => {
                summary.failed += 1;
                warn!(order_id, %error, "秒合约结算失败，将按重试时间重新处理");
                true
            }
        };
        if retry
            && let Err(error) =
                reschedule_settlement_attempt(pool, order_id, now, price_source.retry_seconds())
                    .await
        {
            // 必须收完其他在途事务及提交后消息，不能因一笔调度回写失败丢弃整个并发批次。
            retry_error = Some(error);
        }
    }
    if let Some(error) = retry_error {
        return Err(error);
    }
    Ok(summary)
}

/// 按固定周期持续执行结算；单轮错误只记录，下一轮继续恢复。
pub async fn run_loop(state: AppState, interval_seconds: u64, limit: u32) -> AppResult<()> {
    let mut ticker = interval(Duration::from_secs(interval_seconds.max(1)));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    info!(
        interval_seconds = interval_seconds.max(1),
        batch_limit = seconds_contract_settlement_limit(limit),
        "秒合约结算任务已启动"
    );
    if interval_seconds > 1 {
        warn!(
            interval_seconds,
            "秒合约扫描间隔大于一秒，到期处理可能延迟；请检查环境配置"
        );
    }
    loop {
        ticker.tick().await;
        // 满批且有进展时连续排空积压，不让每一批额外等待一个扫描周期。
        for _ in 0..10 {
            let started = std::time::Instant::now();
            match run_once(&state, Utc::now(), limit).await {
                Ok(summary) => {
                    if summary.scanned > 0 {
                        info!(
                            scanned = summary.scanned,
                            settled = summary.settled,
                            manual_review = summary.manual_review,
                            skipped = summary.skipped,
                            failed = summary.failed,
                            elapsed_ms = started.elapsed().as_millis() as u64,
                            "秒合约结算批次完成"
                        );
                    }
                    if summary.scanned < seconds_contract_settlement_limit(limit)
                        || summary.settled == 0
                    {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
                Err(error) => {
                    error!(%error, "秒合约结算批次失败");
                    break;
                }
            }
        }
    }
}

/// 生产调度只使用 MySQL UTC 时钟；显式时间仅保留在 `run_once_with_pool` 测试入口。
async fn database_now(pool: &Pool<MySql>) -> AppResult<DateTime<Utc>> {
    let now = sqlx::query_scalar::<_, chrono::NaiveDateTime>("SELECT CURRENT_TIMESTAMP(6)")
        .fetch_one(pool)
        .await?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(now, Utc))
}

async fn fetch_due_orders(
    pool: &Pool<MySql>,
    now: DateTime<Utc>,
    limit: u32,
    eligibility_delay: i64,
) -> AppResult<Vec<DueSecondsContractOrder>> {
    sqlx::query_as::<_, DueSecondsContractOrder>(
        r#"SELECT id AS order_id
           FROM seconds_contract_orders
           WHERE status = 'opened'
             AND expires_at <= DATE_SUB(?, INTERVAL ? SECOND)
             AND (next_settlement_attempt_at IS NULL OR next_settlement_attempt_at <= ?)
           ORDER BY COALESCE(next_settlement_attempt_at, expires_at) ASC, id ASC
           LIMIT ?
           FOR UPDATE SKIP LOCKED"#,
    )
    .bind(now.naive_utc())
    .bind(eligibility_delay)
    .bind(now.naive_utc())
    .bind(limit.clamp(1, 500) as i64)
    .fetch_all(pool)
    .await
    .map_err(AppError::from)
}

async fn reschedule_settlement_attempt(
    pool: &Pool<MySql>,
    order_id: u64,
    now: DateTime<Utc>,
    retry_seconds: i64,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE seconds_contract_orders SET next_settlement_attempt_at = ? WHERE id = ? AND status = 'opened'",
    )
    .bind((now + chrono::TimeDelta::seconds(retry_seconds)).naive_utc())
    .bind(order_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// 在订单锁之后选择事件价格，按订单→钱包固定锁序完成结算。
async fn settle_order_by_id(
    pool: &Pool<MySql>,
    order_id: u64,
    processing_time: DateTime<Utc>,
    max_snapshot_wait_seconds: u64,
    price_source: SettlementPriceSource<'_>,
) -> AppResult<SettlementOutcome> {
    let mut tx = pool.begin().await?;
    let order = match infrastructure::try_lock_settlement_order(&mut tx, order_id).await? {
        Some(order) => order,
        None => {
            tx.rollback().await?;
            return Ok(SettlementOutcome::Skipped);
        }
    };
    if order.status == "settled" {
        tx.commit().await?;
        return Ok(SettlementOutcome::Skipped);
    }
    if order.status != "opened" {
        tx.rollback().await?;
        return Ok(SettlementOutcome::Skipped);
    }
    let window_closes_at =
        order.expires_at + chrono::TimeDelta::seconds(price_source.eligibility_delay());
    if processing_time < window_closes_at {
        tx.rollback().await?;
        return Ok(SettlementOutcome::Pending);
    }
    let selected_snapshot = match price_source {
        SettlementPriceSource::Cache(redis) => {
            Some(infrastructure::cached_settlement_price(Some(redis), &order.symbol).await?)
        }
        SettlementPriceSource::History => {
            infrastructure::select_settlement_price_snapshot(
                &mut tx,
                &order.symbol,
                order.expires_at,
            )
            .await?
        }
    };
    let Some(snapshot) = selected_snapshot else {
        let max_wait =
            chrono::TimeDelta::seconds(i64::try_from(max_snapshot_wait_seconds).map_err(|_| {
                AppError::Validation(
                    "seconds contract snapshot wait exceeds supported range".to_owned(),
                )
            })?);
        let manual_review_at = window_closes_at
            .checked_add_signed(max_wait)
            .ok_or_else(|| {
                AppError::Validation(
                    "seconds contract manual review deadline is outside valid range".to_owned(),
                )
            })?;
        if processing_time >= manual_review_at {
            infrastructure::move_order_to_manual_review(
                &mut tx,
                &SecondsContractSettlementExceptionWrite {
                    order_id: order.id,
                    failure_code: MISSING_SETTLEMENT_SNAPSHOT_FAILURE_CODE,
                    detected_at: processing_time,
                    window_start: order.expires_at,
                    window_end: window_closes_at,
                },
            )
            .await?;
            tx.commit().await?;
            return Ok(SettlementOutcome::ManualReview);
        }
        tx.rollback().await?;
        return Ok(SettlementOutcome::Pending);
    };
    let entry_price = order.entry_price.as_ref().ok_or_else(|| {
        AppError::Validation("seconds contract entry price is required for settlement".to_owned())
    })?;
    // 控制在订单锁之后读取：代理设置控制同样要抢这把锁，读到即代表该单最终生效的规则。
    let order_control = infrastructure::select_order_control(&mut tx, order.id).await?;
    let user_control = if order_control.is_none() {
        infrastructure::select_user_control(&mut tx, order.user_id).await?
    } else {
        None
    };
    // 判定价尾数按该单下单时固化的交易对精度换算；存量订单缺快照时才回落到交易对当前精度。
    let price_precision =
        match infrastructure::select_order_price_precision(&mut tx, order.id).await? {
            Some(snapshot) => snapshot,
            None => infrastructure::load_pair_price_precision(&mut tx, order.pair_id).await?,
        };
    let price_step = price_step_for_precision(price_precision)?;
    let resolution = resolve_settlement_outcome(
        order.id,
        &order.direction,
        entry_price,
        &snapshot.price,
        order_control.as_deref(),
        user_control.as_deref(),
        &price_step,
    )?;
    let result = resolution.result.as_str();
    if resolution.controlled {
        info!(
            order_id = order.id,
            result,
            market_result = %resolution.market_result,
            market_price = %snapshot.price,
            settlement_price = %resolution.settlement_price,
            "秒合约订单按控输控赢结果结算"
        );
    }
    let market_price = (resolution.controlled && resolution.settlement_price != snapshot.price)
        .then_some(snapshot.price.clone());
    let precision = infrastructure::load_asset_precision_scale(&mut tx, order.stake_asset).await?;
    let payout_amount =
        seconds_contract_payout_amount(&order.stake_amount, &order.payout_rate, result, precision);
    if payout_amount > 0 {
        let wallet =
            infrastructure::lock_wallet_row(&mut tx, order.user_id, order.stake_asset).await?;
        let available_after = wallet.available.clone() + payout_amount.clone();
        infrastructure::update_wallet_available(
            &mut tx,
            order.user_id,
            order.stake_asset,
            &available_after,
        )
        .await?;
        infrastructure::insert_wallet_ledger(
            &mut tx,
            SecondsContractWalletLedgerWrite {
                user_id: order.user_id,
                asset_id: order.stake_asset,
                change_type: "seconds_contract_settle_win",
                amount: payout_amount.clone(),
                available_after,
                frozen_after: wallet.frozen,
                locked_after: wallet.locked,
                ref_id: order.id.to_string(),
            },
        )
        .await?;
    }
    infrastructure::insert_seconds_journal_in_tx(
        &mut tx,
        order.id,
        order.stake_asset,
        &order.stake_amount,
        Some(&payout_amount),
    )
    .await?;
    infrastructure::mark_order_settled(
        &mut tx,
        order.id,
        result,
        &resolution.settlement_price,
        market_price.as_ref(),
        &snapshot,
    )
    .await?;
    let event = SecondsContractSettlementEvent {
        user_id: order.user_id,
        order_id: order.id,
        product_id: order.product_id,
        pair_id: order.pair_id,
        stake_asset: order.stake_asset,
        direction: order.direction,
        stake_amount: order.stake_amount,
        payout_amount,
        settlement_price: resolution.settlement_price.clone(),
        settlement_price_tick_id: snapshot.id,
        settlement_price_source: snapshot.source,
        settlement_price_observed_at: snapshot.observed_at,
        settlement_price_generation: snapshot.generation,
        settlement_price_version: snapshot.source_version,
        result: result.to_owned(),
    };
    tx.commit().await?;
    Ok(SettlementOutcome::Settled(Box::new(event)))
}

fn publish_settlement_event(
    hub: Option<&EventBroadcastHub>,
    event: &SecondsContractSettlementEvent,
) {
    if let Some(hub) = hub {
        hub.publish(EventBroadcastMessage::private_user(
            event.user_id,
            json!({
                "type": "seconds_contract.order.settled",
                "order_id": event.order_id,
                "product_id": event.product_id,
                "pair_id": event.pair_id,
                "stake_asset": event.stake_asset,
                "direction": event.direction,
                "stake_amount": event.stake_amount,
                "settlement_price": event.settlement_price,
                "settlement_price_tick_id": event.settlement_price_tick_id,
                "settlement_price_source": event.settlement_price_source,
                "settlement_price_observed_at": event.settlement_price_observed_at.timestamp_millis(),
                "settlement_price_generation": event.settlement_price_generation,
                "settlement_price_version": event.settlement_price_version,
                "payout_amount": event.payout_amount,
                "result": event.result,
                "status": "settled",
            })
            .to_string(),
        ));
    }
}

fn seconds_contract_settlement_limit(limit: u32) -> u32 {
    limit.clamp(1, 100)
}

fn seconds_contract_settlement_scan_limit(limit: u32) -> u32 {
    seconds_contract_settlement_limit(limit)
}

fn normalize_max_snapshot_wait_seconds(value: u64) -> u64 {
    value.clamp(1, MAX_CONFIGURED_SNAPSHOT_WAIT_SECONDS)
}

fn env_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_u32(key: &str, default: u32) -> u32 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(default)
}

#[cfg(test)]
#[path = "../../tests/unit_src/src_workers_seconds_contract_settlement_tests.rs"]
mod tests;
