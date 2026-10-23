//! 代理只读充提记录；金额以十进制字符串传输，禁止透出渠道凭据或提现审批内部字段。

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentFundingQuery {
    pub team_scope: Option<String>,
    pub agent_id: Option<u64>,
    pub user_id: Option<u64>,
    pub record_id: Option<u64>,
    pub email: Option<String>,
    pub order_no: Option<String>,
    pub asset_symbol: Option<String>,
    pub network: Option<String>,
    pub source: Option<String>,
    pub status: Option<String>,
    pub created_from: Option<i64>,
    pub created_to: Option<i64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub(crate) struct AgentFundingRecord {
    pub id: u64,
    pub source: String,
    #[schema(required = true)]
    pub order_no: Option<String>,
    pub user_id: u64,
    #[schema(required = true)]
    pub email: Option<String>,
    pub owner_agent_id: u64,
    pub owner_agent_code: String,
    pub asset_symbol: String,
    #[schema(required = true)]
    pub network: Option<String>,
    #[schema(required = true)]
    pub address: Option<String>,
    #[schema(value_type = Option<String>, required = true)]
    pub amount: Option<BigDecimal>,
    #[schema(value_type = Option<String>, required = true)]
    pub paid_amount: Option<BigDecimal>,
    #[schema(value_type = Option<String>, required = true)]
    pub credited_amount: Option<BigDecimal>,
    #[schema(value_type = String)]
    pub fee: BigDecimal,
    #[schema(value_type = Option<String>, required = true)]
    pub total_reserved: Option<BigDecimal>,
    #[schema(value_type = Option<String>, required = true)]
    pub fiat_amount: Option<BigDecimal>,
    #[schema(required = true)]
    pub fiat_currency: Option<String>,
    pub status: String,
    #[schema(required = true)]
    pub tx_hash: Option<String>,
    #[serde(with = "crate::time::unix_millis")]
    #[schema(value_type = i64)]
    pub created_at: DateTime<Utc>,
    #[serde(with = "crate::time::option_unix_millis")]
    #[schema(value_type = Option<i64>, required = true)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(with = "crate::time::option_unix_millis")]
    #[schema(value_type = Option<i64>, required = true)]
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct AgentFundingPage {
    pub items: Vec<AgentFundingRecord>,
    #[schema(minimum = 0)]
    pub total: i64,
}
