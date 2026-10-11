//! 外部行情的原子缓存与待处理队列；队列满时拒绝新快照，不裁剪尚未处理的价格事件。

use super::*;
use crate::error::{AppError, AppResult};
use crate::modules::market::MarketDataProvider;
use serde::Deserialize;

pub(crate) const EXTERNAL_QUEUE_CAPACITY: usize = 50_000;

/// 已通过适配器校验的原始观察；后台消费只处理历史与订单，绝不回写实时价格。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum ExternalMarketEvent {
    Ticker {
        snapshot: MarketTickerSnapshot,
        generation: u64,
    },
    Depth(MarketDepthSnapshot),
    Kline(MarketKlineSnapshot),
}

/// 队列额外保留源时间的完整精度；公开快照的毫秒序列化不能降低 MySQL 归档事件键的微秒精度。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QueuedMarketEvent {
    pub(crate) event: ExternalMarketEvent,
    pub(crate) observed_at: DateTime<Utc>,
}

/// 三条业务 FIFO 加遗留归档排空通道；新 ticker 仅进入现货和杠杆，盘口进入现货，蜡烛进入历史。
#[derive(Debug, Clone, Copy)]
pub(crate) enum ExternalMarketLane {
    Archive,
    Spot,
    Margin,
    Kline,
}

impl ExternalMarketLane {
    pub(crate) const ALL: [Self; 4] = [Self::Archive, Self::Spot, Self::Margin, Self::Kline];

    /// 稳定版本化 key 允许进程重启后继续处理原队头；不使用配置 generation 作为队列名。
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Archive => "market:external-work:v1:archive",
            Self::Spot => "market:external-work:v1:spot",
            Self::Margin => "market:external-work:v1:margin",
            Self::Kline => "market:external-work:v1:kline",
        }
    }
}

impl ExternalMarketEvent {
    /// 保留供应商事件时间用于排队延迟诊断及订单创建时间上界，不伪装成本地接收时间。
    pub(crate) fn observed_at(&self) -> DateTime<Utc> {
        match self {
            Self::Ticker { snapshot, .. } => snapshot.observed_at(),
            Self::Depth(snapshot) => snapshot.observed_at(),
            Self::Kline(snapshot) => snapshot.observed_at(),
        }
    }

    /// 返回规范化交易对，仅供日志和历史处理，不允许队列数据指定任意 Redis key。
    pub(crate) fn symbol(&self) -> &str {
        match self {
            Self::Ticker { snapshot, .. } => snapshot.symbol(),
            Self::Depth(snapshot) => snapshot.symbol(),
            Self::Kline(snapshot) => snapshot.symbol(),
        }
    }
}

// 所有类型/容量/时序检查必须在第一次写入前完成：Lua 执行错误不会回滚已写命令。
static ADMIT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        r#"
local current = redis.call('GET', KEYS[1])
local observed = tonumber(ARGV[1])
local incoming_open = tonumber(ARGV[2])
local current_open = nil
local current_observed = nil
if current then
    local ok, decoded = pcall(cjson.decode, current)
    if not ok or (incoming_open == 0 and type(decoded.observed_at) ~= 'number') or
       (incoming_open > 0 and type(decoded.open_time) ~= 'number') then
        return redis.error_reply('invalid external cached observation')
    end
    current_open = tonumber(decoded.open_time) or 0
    current_observed = tonumber(decoded.observed_at)
end
if incoming_open > 0 then
    local sequence = redis.call('GET', KEYS[2])
    if sequence then
        local a, b = string.match(sequence, '^(%-?%d+):(%-?%d+)$')
        if not a then return redis.error_reply('invalid external kline sequence') end
        current_open, current_observed = tonumber(a), tonumber(b)
    end
end
if current_open and (current_open > incoming_open or
   (current_open == incoming_open and current_observed and current_observed >= observed)) then
    if current_open == incoming_open and current_observed == observed and current == ARGV[3] then
        return 2
    end
    return 0
end
for i = 4, #KEYS do
    if redis.call('LLEN', KEYS[i]) >= tonumber(ARGV[5]) then
        return redis.error_reply('external market work queue is full')
    end
end
for i = 4, #KEYS do redis.call('RPUSH', KEYS[i], ARGV[4]) end
redis.call('SET', KEYS[1], ARGV[3])
if ARGV[6] == '1' then
    local now = redis.call('TIME')
    redis.call('SET', KEYS[3], tostring(tonumber(now[1]) * 1000 + math.floor(tonumber(now[2]) / 1000)))
end
if incoming_open > 0 then
    redis.call('SET', KEYS[2], ARGV[2] .. ':' .. ARGV[1])
end
return 1
"#,
    )
});

impl RedisMarketCache {
    /// 原子接受外部行情并记录全部待处理副作用；旧/回放 ticker 不再归档，也不重复触发资金处理。
    /// 仅用于单机 Redis（与现有多键 CAS 一致）；队列无 TTL，需持久化及 noeviction 部署策略。
    pub(crate) async fn admit_external(
        &self,
        event: &ExternalMarketEvent,
    ) -> AppResult<MarketCacheWriteOutcome> {
        self.admit_external_with_capacity(event, EXTERNAL_QUEUE_CAPACITY)
            .await
    }

    /// 注入容量用于边界回归；所有队列先检查后写入，任何一个满载都拒绝这次缓存更新。
    pub(crate) async fn admit_external_with_capacity(
        &self,
        event: &ExternalMarketEvent,
        capacity: usize,
    ) -> AppResult<MarketCacheWriteOutcome> {
        let encode = |value| {
            serde_json::to_string(value).map_err(|error| {
                AppError::Internal(format!("external market serialization: {error}"))
            })
        };
        let (key, sequence, open, payload, lanes) = match event {
            ExternalMarketEvent::Ticker {
                snapshot,
                generation,
            } => {
                crate::numeric::ensure_amount_storage(snapshot.last_price(), "external ticker")?;
                crate::time::ensure_timestamp_storage(
                    &snapshot.observed_at(),
                    "external ticker observation",
                )?;
                if *generation == 0
                    || snapshot.provider() == MarketDataProvider::Strategy
                    || snapshot.last_price() <= &BigDecimal::from(0)
                {
                    return Err(AppError::Validation(
                        "invalid external ticker provenance".into(),
                    ));
                }
                let entry = MarketTickerCacheEntry::from_snapshot(snapshot)
                    .map_err(|e| AppError::Validation(e.to_string()))?;
                (
                    entry.redis_key().to_owned(),
                    entry.redis_key().to_owned(),
                    0,
                    serde_json::to_value(entry),
                    vec![ExternalMarketLane::Spot, ExternalMarketLane::Margin],
                )
            }
            ExternalMarketEvent::Depth(snapshot) => {
                if snapshot.provider() == MarketDataProvider::Strategy {
                    return Err(AppError::Validation(
                        "strategy depth requires provenance-aware ingestion".into(),
                    ));
                }
                let entry = MarketDepthCacheEntry::from_snapshot(snapshot)
                    .map_err(|e| AppError::Validation(e.to_string()))?;
                let key = market_depth_redis_key(snapshot.symbol());
                (
                    key.clone(),
                    key,
                    0,
                    serde_json::to_value(entry),
                    vec![ExternalMarketLane::Spot],
                )
            }
            ExternalMarketEvent::Kline(snapshot) => {
                if snapshot.provider() == MarketDataProvider::Strategy {
                    return Err(AppError::Validation(
                        "strategy kline requires provenance-aware ingestion".into(),
                    ));
                }
                let entry = MarketKlineCacheEntry::from_snapshot(snapshot)
                    .map_err(|e| AppError::Validation(e.to_string()))?;
                (
                    market_kline_redis_key(snapshot.symbol(), snapshot.interval()),
                    market_kline_sequence_redis_key(snapshot.symbol(), snapshot.interval()),
                    snapshot.open_time().timestamp_millis(),
                    serde_json::to_value(entry),
                    vec![ExternalMarketLane::Kline],
                )
            }
        };
        let payload = payload.map_err(|e| AppError::Internal(e.to_string()))?;
        let payload = encode(&payload)?;
        let job = serde_json::to_string(&QueuedMarketEvent {
            event: event.clone(),
            observed_at: event.observed_at(),
        })
        .map_err(|e| AppError::Internal(e.to_string()))?;
        let mut invocation = ADMIT.prepare_invoke();
        invocation
            .key(key)
            .key(sequence)
            .key(format!("market:received:v1:{}", event.symbol()));
        for lane in lanes {
            invocation.key(lane.key());
        }
        let result: i64 = invocation
            .arg(event.observed_at().timestamp_millis())
            .arg(open)
            .arg(payload)
            .arg(job)
            .arg(capacity)
            .arg(if matches!(event, ExternalMarketEvent::Ticker { .. }) {
                1
            } else {
                0
            })
            .invoke_async(&mut self.manager.clone())
            .await?;
        Ok(cache_write_outcome(result))
    }
}
