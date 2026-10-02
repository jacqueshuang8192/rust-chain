//! 项目行情草稿与发布合同；所有金额为精确十进制，发布只引用已保存的草稿版本。

use super::*;
use crate::modules::market::synthetic_default::DefaultMarketParameters;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SaveNewCoinMarketDraftRequest {
    pub(crate) pair_id: u64,
    pub(crate) expected_revision: u32,
    pub(crate) expected_market_version: u32,
    #[serde(
        default,
        deserialize_with = "crate::numeric::deserialize_optional_decimal"
    )]
    pub(crate) initial_price: Option<BigDecimal>,
    pub(crate) config: DefaultMarketParameters,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PublishNewCoinMarketRequest {
    pub(crate) pair_id: u64,
    pub(crate) expected_revision: u32,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NewCoinMarketDraftResponse {
    pub(crate) pair_id: u64,
    pub(crate) revision: u32,
    pub(crate) base_version: u32,
    pub(crate) initial_price: Option<BigDecimal>,
    pub(crate) config: DefaultMarketParameters,
    pub(crate) published_version: Option<u32>,
    #[serde(with = "unix_millis")]
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct NewCoinMarketPairResponse {
    pub(crate) id: u64,
    pub(crate) symbol: String,
    pub(crate) status: String,
    pub(crate) market_type: String,
    pub(crate) price_precision: i32,
    pub(crate) qty_precision: i32,
}

#[derive(Debug, Serialize)]
pub(crate) struct NewCoinMarketSetupResponse {
    pub(crate) project_id: u64,
    pub(crate) pairs: Vec<NewCoinMarketPairResponse>,
    pub(crate) draft: Option<NewCoinMarketDraftResponse>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NewCoinMarketPublishResponse {
    pub(crate) revision: u32,
    pub(crate) published_version: u32,
    pub(crate) replayed: bool,
}
