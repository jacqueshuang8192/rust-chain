//! 后台提现网络目录配置 DTO；保留原网络接口字段。

use super::*;

#[derive(Debug, Deserialize)]
pub(crate) struct AdminDepositNetworkConfigQuery {
    pub(crate) network: Option<String>,
    pub(crate) address_group_code: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) asset_symbol: Option<String>,
    pub(crate) limit: Option<u32>,
    pub(crate) offset: Option<u32>,
}

impl PresentationLayer for AdminDepositNetworkConfigQuery {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateDepositNetworkConfigRequest {
    pub(crate) network: String,
    pub(crate) display_name: String,
    pub(crate) address_group_code: String,
    pub(crate) address_group_name: Option<String>,
    pub(crate) asset_symbols: Option<Vec<String>>,
    pub(crate) status: Option<String>,
    pub(crate) sort_order: Option<i32>,
    pub(crate) reason: Option<String>,
}

impl PresentationLayer for CreateDepositNetworkConfigRequest {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateDepositNetworkConfigRequest {
    pub(crate) network: String,
    pub(crate) display_name: String,
    pub(crate) address_group_code: String,
    pub(crate) address_group_name: Option<String>,
    pub(crate) asset_symbols: Option<Vec<String>>,
    pub(crate) status: String,
    pub(crate) sort_order: i32,
    pub(crate) reason: Option<String>,
}

impl PresentationLayer for UpdateDepositNetworkConfigRequest {}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub(crate) struct AdminDepositNetworkConfigResponse {
    pub(crate) id: u64,
    pub(crate) network: String,
    pub(crate) display_name: String,
    pub(crate) address_group_code: String,
    pub(crate) address_group_name: Option<String>,
    pub(crate) asset_symbols: SqlxJson<Vec<String>>,
    pub(crate) status: String,
    pub(crate) sort_order: i32,
    #[serde(with = "unix_millis")]
    pub(crate) created_at: DateTime<Utc>,
    #[serde(with = "unix_millis")]
    pub(crate) updated_at: DateTime<Utc>,
}

impl PresentationLayer for AdminDepositNetworkConfigResponse {}

#[derive(Debug, Serialize)]
pub(crate) struct AdminDepositNetworkConfigResponseList {
    pub(crate) configs: Vec<AdminDepositNetworkConfigResponse>,
    pub(crate) total: i64,
}

impl PresentationLayer for AdminDepositNetworkConfigResponseList {}
