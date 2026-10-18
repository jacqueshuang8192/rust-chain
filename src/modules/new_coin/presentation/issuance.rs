//! 新版发行阶段、批次派发和成交报价的传输契约；身份和资金结果均由服务端提供。
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StageRequest {
    pub expected_stage: String,
    pub target_stage: String,
    pub reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AllocationRequest {
    pub subscription_ids: Vec<u64>,
    #[serde(deserialize_with = "crate::numeric::deserialize_decimal_text")]
    pub percent: BigDecimal,
    pub reason: String,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TradeQuoteRequest {
    pub side: String,
    #[serde(default, deserialize_with = "optional_quantity")]
    pub quantity: Option<BigDecimal>,
    pub holding_id: Option<u64>,
    pub idempotency_key: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub(crate) struct TradeQuote {
    pub id: String,
    pub user_id: u64,
    pub project_id: u64,
    pub holding_id: Option<u64>,
    pub side: String,
    pub quantity: BigDecimal,
    pub price: BigDecimal,
    pub gross: BigDecimal,
    pub cost: BigDecimal,
    pub fee: BigDecimal,
    pub net: BigDecimal,
    pub source: String,
    pub source_version: String,
    #[serde(with = "crate::time::unix_millis")]
    pub observed_at: DateTime<Utc>,
    #[serde(with = "crate::time::unix_millis")]
    pub expires_at: DateTime<Utc>,
    pub status: String,
    pub receipt_json: Option<sqlx::types::Json<serde_json::Value>>,
    #[serde(skip)]
    pub intent_json: sqlx::types::Json<serde_json::Value>,
}

/// 持仓以主键游标分页；可按项目筛选，用户范围仍由令牌固定。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HoldingsQuery {
    pub project_id: Option<u64>,
    pub before_id: Option<u64>,
}

/// 发行项目按本人范围以主键游标分页。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProjectsQuery {
    pub before_id: Option<u64>,
}

/// 新报价只接受精确十进制文本；缺省/null用于卖出整批持仓，数值型 JSON 不可恢复精度。
fn optional_quantity<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<BigDecimal>, D::Error> {
    Option::<String>::deserialize(deserializer)?
        .map(|s| crate::numeric::parse_decimal_input(&s).map_err(serde::de::Error::custom))
        .transpose()
}
