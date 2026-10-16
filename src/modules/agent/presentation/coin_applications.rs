//! 代理发币申请与审核传输契约；金额使用十进制文本，身份和审核结果不由代理传入。
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CoinApplicationRequest {
    pub symbol: String,
    pub name: String,
    pub precision_scale: i32,
    #[serde(deserialize_with = "crate::numeric::deserialize_decimal_text")]
    pub total_supply: BigDecimal,
    #[serde(deserialize_with = "crate::numeric::deserialize_decimal_text")]
    pub issue_price: BigDecimal,
    pub quote_asset_id: u64,
    #[serde(with = "crate::time::unix_millis")]
    pub listed_at: DateTime<Utc>,
    pub reason: String,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CoinApplicationReview {
    pub decision: String,
    pub reason: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CoinApplicationQuery {
    pub status: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct CoinApplication {
    pub id: u64,
    pub agent_id: u64,
    pub agent_admin_id: u64,
    pub symbol: String,
    pub name: String,
    pub precision_scale: i32,
    pub total_supply: BigDecimal,
    pub issue_price: BigDecimal,
    pub quote_asset_id: u64,
    pub quote_symbol: String,
    #[serde(with = "crate::time::unix_millis")]
    pub listed_at: DateTime<Utc>,
    pub reason: String,
    #[serde(skip)]
    pub idempotency_key: Vec<u8>,
    pub status: String,
    pub project_id: Option<u64>,
    pub reviewed_by: Option<u64>,
    pub review_reason: Option<String>,
    #[serde(with = "crate::time::unix_millis")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "crate::time::option_unix_millis")]
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CoinApplicationsResponse {
    pub applications: Vec<CoinApplication>,
    pub total: i64,
}
