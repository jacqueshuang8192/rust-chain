//! 只读充提查询的已验证条件，与具体 SQL 和 HTTP 传输无关。

use super::AgentListPage;
use chrono::{DateTime, Utc};

/// 固定记录类型选择既有业务表，禁止用户输入决定任意表名。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FundingKind {
    Deposits,
    Withdrawals,
}

/// 子树范围只能由身份派生；这些筛选仅进一步收窄，并已通过输入上界校验。
pub(crate) struct FundingFilter {
    pub kind: FundingKind,
    pub team_scope: String,
    pub agent_id: Option<u64>,
    pub user_id: Option<u64>,
    pub record_id: Option<u64>,
    pub email: Option<String>,
    pub order_no: Option<String>,
    pub asset_symbol: Option<String>,
    pub network: Option<String>,
    pub source: Option<String>,
    pub status: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub page: AgentListPage,
}
