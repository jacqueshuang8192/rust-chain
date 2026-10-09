//! 团队订单的只读查询与响应；身份补充字段不替换各业务原有订单快照。

use super::{
    AgentUserMarginPositionResponse, AgentUserSecondsContractOrderResponse,
    AgentUserSpotOrderResponse,
};
use crate::time::{option_unix_millis, unix_millis};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 所有参数只能缩小当前令牌子树；时间范围为创建时间左闭右开区间。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentTeamOrdersQuery {
    pub(crate) team_scope: Option<String>,
    pub(crate) agent_id: Option<u64>,
    pub(crate) user_id: Option<u64>,
    pub(crate) email: Option<String>,
    pub(crate) order_id: Option<u64>,
    pub(crate) product_id: Option<u64>,
    pub(crate) symbol: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) direction: Option<String>,
    pub(crate) result: Option<String>,
    pub(crate) created_from: Option<i64>,
    pub(crate) created_to: Option<i64>,
    pub(crate) limit: Option<u32>,
    pub(crate) offset: Option<u32>,
}

/// 每条业务记录携带当前用户邮箱和直属归属代理，不暴露邀请链之外的账号资料。
#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct AgentTeamOrder<T> {
    pub(crate) email: Option<String>,
    pub(crate) owner_agent_id: u64,
    pub(crate) owner_agent_code: String,
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub(crate) order: T,
}

/// 行与总数采用相同筛选；翻页不额外查询各用户，也不汇总不同资产的金额。
#[derive(Debug, Serialize)]
pub(crate) struct AgentTeamOrderPage<T> {
    pub(crate) orders: Vec<AgentTeamOrder<T>>,
    pub(crate) total: i64,
}

/// 各订单端点维持各自业务字段，传输层只统一分页包络。
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum AgentTeamOrdersResponse {
    Seconds(AgentTeamOrderPage<AgentUserSecondsContractOrderResponse>),
    Margin(AgentTeamOrderPage<AgentUserMarginPositionResponse>),
    Spot(AgentTeamOrderPage<AgentUserSpotOrderResponse>),
    Earn(AgentTeamOrderPage<AgentTeamEarnOrderResponse>),
}

/// 理财仅显示申购时快照，不计算尚未落库的收益，不返回幂等键等内部字段。
#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct AgentTeamEarnOrderResponse {
    pub(crate) id: u64,
    pub(crate) user_id: u64,
    pub(crate) product_id: u64,
    pub(crate) product_name: String,
    pub(crate) asset_id: u64,
    pub(crate) asset_symbol: String,
    pub(crate) amount: BigDecimal,
    pub(crate) apr_rate: BigDecimal,
    pub(crate) term_days: u32,
    pub(crate) status: String,
    #[serde(with = "unix_millis")]
    pub(crate) created_at: DateTime<Utc>,
    #[serde(with = "unix_millis")]
    pub(crate) matures_at: DateTime<Utc>,
    #[serde(with = "option_unix_millis")]
    pub(crate) redeemed_at: Option<DateTime<Utc>>,
}
