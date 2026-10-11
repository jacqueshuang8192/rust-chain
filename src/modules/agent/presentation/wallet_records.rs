//! 团队用户钱包变动与资金流水的只读传输模型，金额保留十进制字符串。

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct AgentWalletRecordsQuery {
    pub(crate) account_type: Option<String>,
    pub(crate) asset_id: Option<u64>,
    pub(crate) change_type: Option<String>,
    pub(crate) balance_type: Option<String>,
    pub(crate) ref_type: Option<String>,
    pub(crate) ref_id: Option<String>,
    pub(crate) limit: Option<u32>,
    pub(crate) offset: Option<u32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct AgentWalletRecord {
    pub(crate) id: u64,
    pub(crate) account_type: String,
    pub(crate) user_id: u64,
    pub(crate) user_email: Option<String>,
    pub(crate) asset_id: u64,
    pub(crate) asset_symbol: String,
    pub(crate) change_type: String,
    pub(crate) amount: BigDecimal,
    pub(crate) balance_type: String,
    pub(crate) balance_after: BigDecimal,
    pub(crate) available_after: BigDecimal,
    pub(crate) frozen_after: BigDecimal,
    pub(crate) locked_after: BigDecimal,
    pub(crate) ref_type: String,
    pub(crate) ref_id: String,
    #[serde(with = "crate::time::unix_millis")]
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AgentWalletRecordsResponse {
    pub(crate) ledger: Vec<AgentWalletRecord>,
    pub(crate) total: i64,
}
