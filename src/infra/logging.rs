//! 统一面向运维的中文单行日志；保留稳定业务标识，不输出请求 span 中的令牌或完整载荷。

use std::fmt;
use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{
    EnvFilter,
    fmt::{FmtContext, FormatEvent, FormatFields, format::Writer},
    layer::SubscriberExt,
    registry::LookupSpan,
    util::SubscriberInitExt,
};

/// API 和迁移程序共用日志入口；普通 info 配置只开启本项目的信息日志，依赖库保留告警。
/// 显式模块过滤配置仍受 RUST_LOG 控制；格式化不读取请求 span，避免查询参数中的凭据进入日志。
pub fn init() {
    let requested = std::env::var("RUST_LOG").unwrap_or_default();
    let filter = if requested.trim().is_empty() || requested.trim() == "info" {
        EnvFilter::new("warn,exchange_api=info,exchange_migrate=info")
    } else {
        EnvFilter::try_new(requested)
            .unwrap_or_else(|_| EnvFilter::new("warn,exchange_api=info,exchange_migrate=info"))
    };
    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .event_format(ChineseLogFormat)
                .with_ansi(false),
        )
        .init();
}

/// 只格式化事件自身字段：时间、中文等级、模块、消息与中文参数稳定分区。
pub struct ChineseLogFormat;

impl<S, N> FormatEvent<S, N> for ChineseLogFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut fields = LogFields::default();
        event.record(&mut fields);
        let level = match *event.metadata().level() {
            tracing::Level::ERROR => "错误",
            tracing::Level::WARN => "警告",
            tracing::Level::INFO => "信息",
            tracing::Level::DEBUG => "调试",
            tracing::Level::TRACE => "跟踪",
        };
        let target = event.metadata().target();
        let module = target.strip_prefix("exchange_api::").unwrap_or(target);
        write!(
            writer,
            "{} [{level}] {}",
            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            fields.message
        )?;
        for (name, value) in fields.values {
            write!(writer, " | {}={value}", field_label(&name))?;
        }
        writeln!(writer, " | 模块={module}")
    }
}

#[derive(Default)]
struct LogFields {
    message: String,
    values: Vec<(String, String)>,
}

impl LogFields {
    fn record(&mut self, field: &Field, value: String) {
        let name = field.name();
        let value = if [
            "password",
            "token",
            "access_token",
            "refresh_token",
            "secret",
            "authorization",
            "payload",
        ]
        .contains(&name)
        {
            "[已隐藏]".to_owned()
        } else {
            crate::migration::sanitize_log_text(&value)
        };
        if name == "message" {
            self.message = value;
        } else {
            self.values.push((name.to_owned(), value));
        }
    }
}

impl Visit for LogFields {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record(field, value.to_owned());
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record(field, format!("{value:?}"));
    }
}

fn field_label(name: &str) -> &str {
    match name {
        "scanned" => "扫描数",
        "settled" => "结算数",
        "failed" => "失败数",
        "skipped" => "跳过数",
        "manual_review" => "转人工数",
        "elapsed_ms" => "耗时毫秒",
        "source_age_ms" => "行情距今毫秒",
        "lag_duration_ms" => "行情持续落后毫秒",
        "queued" => "待处理事件数",
        "accepted" => "已接受",
        "error" | "join_error" | "error_chain" => "错误详情",
        "reason" => "原因",
        "order_id" => "订单ID",
        "user_id" => "用户ID",
        "position_id" => "仓位ID",
        "pair_id" => "交易对ID",
        "symbol" | "pair_symbol" => "交易对",
        "strategy_id" => "策略ID",
        "subscription_id" => "申购ID",
        "commission_id" => "佣金ID",
        "quote_id" => "报价ID",
        "margin_asset" => "保证金资产",
        "provider" | "providers" => "行情源",
        "generation" => "运行代次",
        "interval_seconds" => "间隔秒数",
        "batch_limit" => "批次上限",
        "delay_seconds" | "retry_delay_seconds" | "retry_seconds" => "重试等待秒数",
        "addr" => "监听地址",
        "mysql" => "数据库已连接",
        "mongo" => "历史库已连接",
        "redis" => "缓存已连接",
        "published" => "推送数",
        "leased" => "领取数",
        "redeemed" => "赎回数",
        "released" => "解禁数",
        "blocked_fee" => "待缴费用数",
        "accrued" => "计息数",
        "liquidated" => "清算数",
        "healthy" => "正常数",
        "marked" => "标记数",
        "collected" => "回收数",
        "waiting_balance" => "余额不足数",
        "due_at" => "到期时间",
        "network" => "网络",
        "closed" => "关盘数",
        "withdrawal_scanned" => "提现扫描数",
        "withdrawal_broadcasted" => "提现广播数",
        "withdrawal_retried" => "提现重试数",
        "withdrawal_failed" => "提现失败数",
        "withdrawal_confirmed" => "提现确认数",
        "withdrawal_manual_review" => "提现待人工数",
        "deposit_observed" => "充值观察数",
        "gateway_failed" => "网关失败数",
        "event_dead_lettered" => "事件死信数",
        "operation" => "操作",
        "side" => "方向",
        "max_strategies_per_round" => "单轮策略上限",
        "recovery_interval_seconds" => "恢复间隔秒数",
        "attempt" => "尝试次数",
        "max_attempts" => "最大尝试次数",
        "server_version" => "数据库版本",
        "migrations_table_exists" => "迁移表存在",
        "migration_count" => "迁移数量",
        "latest_version" => "最新版本",
        "dirty_versions" => "未完成迁移版本",
        "queue_name" | "queue" => "队列名称",
        "consumer_tag" => "消费者标识",
        "consumer_name" => "消费者名称",
        "payload" => "载荷",
        "password" => "密码",
        "token" | "access_token" | "refresh_token" => "令牌",
        "authorization" => "认证信息",
        "secret" => "密钥",
        _ => name,
    }
}

#[cfg(test)]
#[path = "../../tests/unit_src/src_infra_logging_tests.rs"]
mod tests;
