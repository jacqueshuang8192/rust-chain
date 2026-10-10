//! 外部行情慢副作用消费者：四条持久 FIFO 独立推进，失败保留队头，取消完成在途项后退出。

use crate::{
    error::{AppError, AppResult},
    modules::{
        margin::application::execute_triggered_margin_limit_orders_with_hub_at,
        market::{
            MarketDataProvider,
            adapters::MarketIngestionService,
            infrastructure::cache::external::{
                ExternalMarketEvent, ExternalMarketLane, QueuedMarketEvent,
            },
        },
        spot::application::execute_triggered_spot_limit_orders_with_hub_at,
    },
    state::AppState,
};
use redis::{AsyncCommands, Script};
use sha2::{Digest, Sha256};
use sqlx::{Connection, MySqlConnection, QueryBuilder};
use std::sync::LazyLock;
use tokio::{
    task::JoinSet,
    time::{Duration, Instant, sleep, timeout},
};
use tokio_util::sync::CancellationToken;

static ACK: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        r#"
for i = 1, #ARGV do
    if redis.call('LINDEX', KEYS[1], i - 1) ~= ARGV[i] then return 0 end
end
redis.call('LTRIM', KEYS[1], #ARGV, -1)
return 1
"#,
    )
});

/// 独立监督全部消费通道；任一任务异常退出就取消同组，等待在途操作结束，不遗留游离任务。
pub(super) async fn run(state: AppState, cancellation: CancellationToken) -> AppResult<()> {
    let cancellation = cancellation.child_token();
    let ingestion = MarketIngestionService::from_state(&state)?;
    let mut tasks = JoinSet::new();
    for lane in ExternalMarketLane::ALL {
        let state = state.clone();
        let ingestion = ingestion.clone();
        let cancellation = cancellation.clone();
        tasks.spawn(async move { run_lane(state, ingestion, lane, cancellation).await });
    }
    let mut failure = None;
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(())) if cancellation.is_cancelled() => {}
            Ok(Ok(())) => {
                failure = Some(AppError::Internal(
                    "market work consumer stopped unexpectedly".into(),
                ));
                cancellation.cancel();
            }
            Ok(Err(error)) => {
                failure = Some(error);
                cancellation.cancel();
            }
            Err(error) => {
                failure = Some(AppError::Internal(format!(
                    "market work consumer join: {error}"
                )));
                cancellation.cancel();
            }
        }
    }
    failure.map_or(Ok(()), Err)
}

async fn run_lane(
    state: AppState,
    ingestion: MarketIngestionService,
    lane: ExternalMarketLane,
    cancellation: CancellationToken,
) -> AppResult<()> {
    let pool = state.mysql.as_ref().ok_or_else(|| {
        AppError::Internal("mysql is required for external market consumers".into())
    })?;
    let mut retry_seconds = 1;
    while !cancellation.is_cancelled() {
        // 独立物理连接承载会话锁，不能将持锁连接归还业务池，否则其他请求可能继承锁。
        let result = async {
            let mut lock = timeout(
                Duration::from_secs(10),
                MySqlConnection::connect_with(&pool.connect_options()),
            )
            .await
            .map_err(|_| AppError::Internal("market consumer lock connection timeout".into()))??;
            let acquired: Option<i64> =
                sqlx::query_scalar("SELECT GET_LOCK(SHA2(CONCAT(DATABASE(), ':', ?), 256), 0)")
                    .bind(lane.key())
                    .fetch_one(&mut lock)
                    .await?;
            if acquired != Some(1) {
                lock.close().await?;
                if acquired != Some(0) {
                    return Err(AppError::Internal(
                        "market consumer session lock was not available".into(),
                    ));
                }
                return Ok(());
            }
            let result = consume_locked(&state, &ingestion, lane, &mut lock, &cancellation).await;
            let closed = lock.close().await;
            result?;
            closed?;
            Ok::<_, AppError>(())
        }
        .await;
        match result {
            Ok(()) => retry_seconds = 1,
            Err(error) => {
                tracing::warn!(queue = lane.key(), retry_seconds, %error,
                    "行情后台消费失败，保留未确认事件并重试");
                retry_seconds = (retry_seconds * 2).min(30);
            }
        }
        tokio::select! {
            _ = cancellation.cancelled() => break,
            _ = sleep(Duration::from_secs(retry_seconds)) => {}
        }
    }
    Ok(())
}

async fn consume_locked(
    state: &AppState,
    ingestion: &MarketIngestionService,
    lane: ExternalMarketLane,
    lock: &mut MySqlConnection,
    cancellation: &CancellationToken,
) -> AppResult<()> {
    let mut redis = state.redis.clone().ok_or_else(|| {
        AppError::Internal("redis is required for external market consumers".into())
    })?;
    let batch = if matches!(lane, ExternalMarketLane::Archive) {
        100
    } else {
        1
    };
    let mut last_report = Instant::now();
    while !cancellation.is_cancelled() {
        let payloads: Vec<String> = redis.lrange(lane.key(), 0, batch - 1).await?;
        if payloads.is_empty() {
            tokio::select! {
                _ = cancellation.cancelled() => break,
                _ = sleep(Duration::from_millis(100)) => {}
            }
            continue;
        }
        verify_lock(lock, lane).await?;
        let events = payloads
            .iter()
            .map(|payload| {
                serde_json::from_str::<QueuedMarketEvent>(payload).map_err(|error| {
                    AppError::Internal(format!("invalid queued market observation: {error}"))
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        let started = Instant::now();
        if matches!(lane, ExternalMarketLane::Archive) {
            archive(state, &events).await?;
        } else {
            process(state, ingestion, lane, &events[0]).await?;
        }
        // 提交后确认前重新核对会话锁；故障期间不删除任务，重放由业务终态和归档唯一键保证幂等。
        verify_lock(lock, lane).await?;
        acknowledge(&mut redis, lane, &payloads).await?;
        if last_report.elapsed() >= Duration::from_secs(10) {
            let queued: usize = redis.llen(lane.key()).await?;
            let source_age_ms = chrono::Utc::now()
                .signed_duration_since(events[0].observed_at)
                .num_milliseconds();
            tracing::debug!(
                queue = lane.key(),
                queued,
                source_age_ms,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "行情后台消费进度"
            );
            if queued >= 1_000 {
                tracing::warn!(
                    queue = lane.key(),
                    queued,
                    source_age_ms,
                    "行情后台处理队列积压"
                );
            }
            last_report = Instant::now();
        }
    }
    Ok(())
}

async fn verify_lock(lock: &mut MySqlConnection, lane: ExternalMarketLane) -> AppResult<()> {
    let owned: Option<i64> = timeout(
        Duration::from_secs(10),
        sqlx::query_scalar(
            "SELECT IS_USED_LOCK(SHA2(CONCAT(DATABASE(), ':', ?), 256)) = CONNECTION_ID()",
        )
        .bind(lane.key())
        .fetch_one(lock),
    )
    .await
    .map_err(|_| AppError::Internal("market consumer lock verification timeout".into()))??;
    if owned != Some(1) {
        return Err(AppError::Conflict(
            "market consumer lost its session lock".into(),
        ));
    }
    Ok(())
}

async fn acknowledge(
    redis: &mut redis::aio::ConnectionManager,
    lane: ExternalMarketLane,
    payloads: &[String],
) -> AppResult<()> {
    let mut invocation = ACK.prepare_invoke();
    invocation.key(lane.key());
    for payload in payloads {
        invocation.arg(payload);
    }
    let acknowledged: i64 = invocation.invoke_async(redis).await?;
    if acknowledged != 1 {
        return Err(AppError::Conflict(
            "market work queue head changed before acknowledgement".into(),
        ));
    }
    Ok(())
}

async fn archive(state: &AppState, events: &[QueuedMarketEvent]) -> AppResult<()> {
    let pool = state
        .mysql
        .as_ref()
        .ok_or_else(|| AppError::Internal("mysql is missing".into()))?;
    let mut rows = Vec::with_capacity(events.len());
    for event in events {
        let ExternalMarketEvent::Ticker {
            snapshot,
            generation,
        } = &event.event
        else {
            return Err(AppError::Validation("non-ticker in archive queue".into()));
        };
        let source = match snapshot.provider() {
            MarketDataProvider::Bitget => "bitget",
            MarketDataProvider::Htx => "htx",
            MarketDataProvider::Coinbase => "coinbase",
            MarketDataProvider::Strategy => {
                return Err(AppError::Validation("strategy in external queue".into()));
            }
        };
        let canonical = format!(
            "{}|{}|{}|{}",
            source,
            snapshot.symbol(),
            event.observed_at.timestamp_micros(),
            snapshot.last_price().normalized()
        );
        rows.push((
            snapshot,
            generation,
            source,
            hex::encode(Sha256::digest(canonical.as_bytes())),
            event.observed_at,
        ));
    }
    let mut query = QueryBuilder::new(
        "INSERT INTO market_price_ticks (event_key, symbol, price, source, observed_at, generation, source_version) ",
    );
    query.push_values(
        &rows,
        |mut row, (snapshot, generation, source, key, observed_at)| {
            row.push_bind(key)
                .push_bind(snapshot.symbol())
                .push_bind(snapshot.last_price())
                .push_bind(*source)
                .push_bind(observed_at.naive_utc())
                .push_bind(**generation)
                .push_bind(key);
        },
    );
    query.push(" ON DUPLICATE KEY UPDATE event_key = VALUES(event_key)");
    query.build().execute(pool).await?;
    Ok(())
}

async fn process(
    state: &AppState,
    ingestion: &MarketIngestionService,
    lane: ExternalMarketLane,
    event: &QueuedMarketEvent,
) -> AppResult<()> {
    let pool = state
        .mysql
        .as_ref()
        .ok_or_else(|| AppError::Internal("mysql is missing".into()))?;
    let hub = state.event_broadcast_hub.as_ref();
    let at = Some(event.observed_at.naive_utc());
    match (lane, &event.event) {
        (ExternalMarketLane::Spot, ExternalMarketEvent::Ticker { snapshot, .. }) => {
            execute_triggered_spot_limit_orders_with_hub_at(
                pool,
                snapshot.symbol(),
                snapshot.last_price(),
                hub,
                at,
            )
            .await?;
        }
        (ExternalMarketLane::Spot, ExternalMarketEvent::Depth(snapshot)) => {
            if let Some(price) = snapshot.asks().iter().map(|level| &level.price).min() {
                execute_triggered_spot_limit_orders_with_hub_at(
                    pool,
                    snapshot.symbol(),
                    price,
                    hub,
                    at,
                )
                .await?;
            }
        }
        (ExternalMarketLane::Margin, ExternalMarketEvent::Ticker { snapshot, .. }) => {
            execute_triggered_margin_limit_orders_with_hub_at(
                pool,
                snapshot.symbol(),
                snapshot.last_price(),
                hub,
                at,
            )
            .await?;
        }
        (ExternalMarketLane::Kline, ExternalMarketEvent::Kline(snapshot)) => {
            ingestion.persist_external_kline(snapshot).await?;
        }
        _ => {
            return Err(AppError::Validation(
                "market observation is in the wrong work queue".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/unit_src/src_workers_market_feed_side_effects_tests.rs"]
mod tests;
