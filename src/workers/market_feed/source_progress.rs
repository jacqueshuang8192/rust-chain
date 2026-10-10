//! 识别持续接收旧报文的假活连接；只判断推送信封时间，不把蜡烛开盘或历史成交时间当作链路时间。

use super::{AppError, AppResult, Duration, Instant, MarketFeedFrame, MarketFeedProvider};
use chrono::{DateTime, Utc};
use serde_json::Value;

/// 当前供应商连接的源时间观察窗，与入站静默心跳状态独立。
pub(super) struct SourceProgress {
    max_age: Duration,
    grace: Duration,
    lag_since: Option<Instant>,
    lagged_frames: u32,
}

impl Default for SourceProgress {
    /// 容忍短时抖动；推送时间持续落后30秒且跨越10秒、至少3帧才触发连接恢复。
    fn default() -> Self {
        Self::new(Duration::from_secs(30), Duration::from_secs(10))
    }
}

impl SourceProgress {
    /// 注入时间边界便于确定性回归；状态仅属于当前连接，重连重新观察，不改变缓存或交易价格规则。
    pub(super) fn new(max_age: Duration, grace: Duration) -> Self {
        Self {
            max_age,
            grace,
            lag_since: None,
            lagged_frames: 0,
        }
    }

    /// 在当前帧的摄取与广播完成后检查源进度；不取消在途写入，不删除已进入持久队列的副作用。
    /// 心跳/缺失信封时间不作为新行情，历史行时间一律忽略；新鲜信封会清除暂时落后状态。
    pub(super) fn check(
        &mut self,
        frame: &MarketFeedFrame,
        wall_now: DateTime<Utc>,
        now: Instant,
    ) -> AppResult<()> {
        let Some(source_time) = envelope_time(frame) else {
            return Ok(());
        };
        let age = wall_now.signed_duration_since(source_time);
        if age.to_std().unwrap_or_default() <= self.max_age {
            self.lag_since = None;
            self.lagged_frames = 0;
            return Ok(());
        }
        let since = *self.lag_since.get_or_insert(now);
        self.lagged_frames = self.lagged_frames.saturating_add(1);
        let duration = now.saturating_duration_since(since);
        if self.lagged_frames < 3 || duration < self.grace {
            return Ok(());
        }
        tracing::warn!(provider = ?frame.provider(), source_age_ms = age.num_milliseconds(),
            lag_duration_ms = duration.as_millis() as u64,
            "外部行情持续落后，结束旧连接并重新订阅");
        Err(AppError::Internal(format!(
            "market feed source backlog: push is {}ms old for {}ms",
            age.num_milliseconds(),
            duration.as_millis()
        )))
    }
}

/// 只提取供应商推送信封中的时刻；不能回退到data内的成交、ticker快照或K线开盘时间。
fn envelope_time(frame: &MarketFeedFrame) -> Option<DateTime<Utc>> {
    let value: Value = serde_json::from_str(frame.payload()).ok()?;
    match frame.provider() {
        MarketFeedProvider::Bitget | MarketFeedProvider::Htx => {
            let timestamp = value.get("ts")?;
            let milliseconds = timestamp
                .as_i64()
                .or_else(|| timestamp.as_str()?.parse().ok())?;
            (milliseconds > 0)
                .then(|| DateTime::from_timestamp_millis(milliseconds))
                .flatten()
        }
        MarketFeedProvider::Coinbase => {
            DateTime::parse_from_rfc3339(value.get("timestamp")?.as_str()?)
                .ok()
                .map(|time| time.with_timezone(&Utc))
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/unit_src/src_workers_market_feed_source_progress_tests.rs"]
mod tests;
