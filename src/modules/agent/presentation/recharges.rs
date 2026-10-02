//! 代理人工充值的传输契约；金额保留十进制字符串，主体只从认证上下文取得。

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentRechargeRequest {
    pub asset_id: u64,
    #[serde(deserialize_with = "crate::numeric::deserialize_decimal_text")]
    pub amount: BigDecimal,
    pub reason: String,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewAgentRechargeRequest {
    pub decision: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateAgentRechargePolicy {
    pub direct_recharge_enabled: bool,
    pub expected_version: u64,
    pub reason: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct AgentRechargePolicy {
    pub agent_id: u64,
    pub direct_recharge_enabled: bool,
    pub version: u64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct RechargeAsset {
    pub asset_id: u64,
    pub symbol: String,
    pub precision_scale: i32,
}

#[derive(Debug, Serialize)]
pub(crate) struct AgentRechargeOptions {
    pub direct_recharge_enabled: bool,
    pub assets: Vec<RechargeAsset>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub(crate) struct AgentRechargeRecord {
    pub id: u64,
    pub agent_id: u64,
    pub agent_admin_id: u64,
    pub user_id: u64,
    pub asset_id: u64,
    pub asset_symbol: String,
    pub amount: BigDecimal,
    pub reason: String,
    #[serde(skip)]
    pub idempotency_key: Vec<u8>,
    pub mode: String,
    pub status: String,
    pub reviewed_by: Option<u64>,
    pub review_reason: Option<String>,
    #[serde(with = "crate::time::unix_millis")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "crate::time::option_unix_millis")]
    pub reviewed_at: Option<DateTime<Utc>>,
    #[serde(with = "crate::time::option_unix_millis")]
    pub credited_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentRechargeQuery {
    pub status: Option<String>,
    pub user_id: Option<u64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdminAgentRechargeQuery {
    pub status: Option<String>,
    pub user_id: Option<u64>,
    pub agent_id: Option<u64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AgentRechargesResponse {
    pub recharges: Vec<AgentRechargeRecord>,
    pub total: i64,
}
