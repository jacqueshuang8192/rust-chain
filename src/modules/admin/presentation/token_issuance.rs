//! 管理员新版发行及只读账务合同；金额保持十进制文本，已成交回购与待确认报价分离。
use super::*;
use crate::modules::new_coin::domain::issuance::IssuancePolicy;

/// 新建项目从既有资产开始，固定预热阶段；不接受调用方指定归属、成交价或旧版解禁字段。
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateTokenIssuanceRequest {
    pub asset_id: u64,
    pub quote_asset_id: u64,
    #[serde(deserialize_with = "crate::numeric::deserialize_decimal_text")]
    pub total_supply: BigDecimal,
    #[serde(deserialize_with = "crate::numeric::deserialize_decimal_text")]
    pub issue_price: BigDecimal,
    pub issuance_policy: IssuancePolicy,
    pub reason: String,
    pub idempotency_key: String,
}

/// 账务列表按项目、用户、买家直属代理和发行代理分别筛选；分页由用例限制。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdminTokenRecordsQuery {
    pub project_id: Option<u64>,
    pub user_id: Option<u64>,
    pub agent_id: Option<u64>,
    pub issuer_agent_id: Option<u64>,
    pub email: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// 操作记录保存真实执行者；代理、用户和调度器不能被标记为审核管理员。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdminTokenOperationsQuery {
    pub project_id: Option<u64>,
    pub actor_type: Option<String>,
    pub actor_id: Option<u64>,
    pub action: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// 持仓展示不可变成本及合同；有效解禁时刻来自实际阶段事件，到期不代表已卖出。
#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct AdminTokenHolding {
    pub id: u64,
    pub project_id: u64,
    pub symbol: String,
    pub quote_symbol: String,
    pub user_id: u64,
    pub email: Option<String>,
    pub owner_agent_id: Option<u64>,
    pub owner_agent_code: Option<String>,
    pub issuer_agent_id: Option<u64>,
    pub source_kind: String,
    pub source_id: u64,
    pub quantity: BigDecimal,
    pub cost: BigDecimal,
    pub policy_json: sqlx::types::Json<IssuancePolicy>,
    pub status: String,
    #[serde(with = "unix_millis")]
    pub acquired_at: DateTime<Utc>,
    #[serde(with = "option_unix_millis")]
    pub unlock_at: Option<DateTime<Utc>>,
    #[serde(with = "option_unix_millis")]
    pub sold_at: Option<DateTime<Utc>>,
}

/// 仅返回已执行卖出；gross、fee、net是同一成交凭证中的实际结算数值。
#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct AdminTokenBuyback {
    pub id: String,
    pub project_id: u64,
    pub symbol: String,
    pub quote_symbol: String,
    pub user_id: u64,
    pub email: Option<String>,
    pub owner_agent_id: Option<u64>,
    pub owner_agent_code: Option<String>,
    pub issuer_agent_id: Option<u64>,
    pub holding_id: u64,
    pub quantity: BigDecimal,
    pub price: BigDecimal,
    pub cost: BigDecimal,
    pub gross: BigDecimal,
    pub fee: BigDecimal,
    pub net: BigDecimal,
    pub source: String,
    pub source_version: String,
    pub status: String,
    #[serde(with = "unix_millis")]
    pub observed_at: DateTime<Utc>,
    #[serde(with = "unix_millis")]
    pub executed_at: DateTime<Utc>,
}

/// 全角色业务操作证据用于审计查看，不提供重放资金或改写历史的命令。
#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct AdminTokenOperation {
    pub id: u64,
    pub project_id: u64,
    pub symbol: String,
    pub actor_type: String,
    pub actor_id: Option<u64>,
    pub action: String,
    pub reason: String,
    pub payload_json: sqlx::types::Json<Value>,
    #[serde(with = "unix_millis")]
    pub created_at: DateTime<Utc>,
}
