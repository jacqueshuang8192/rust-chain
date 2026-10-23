//! 限时充值、只读监听配置及人工核对接口契约。

use crate::time::{option_unix_millis, unix_millis};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateDepositOrder {
    pub asset_symbol: String,
    pub network: String,
    #[serde(deserialize_with = "crate::numeric::deserialize_decimal_text")]
    #[schema(value_type = String)]
    pub amount: BigDecimal,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ToSchema)]
pub struct DepositOrder {
    pub asset_kind: String,
    pub token_address: String,
    pub id: u64,
    pub order_no: String,
    pub user_id: u64,
    pub asset_symbol: String,
    pub network: String,
    pub address: String,
    #[schema(value_type = String)]
    pub amount: BigDecimal,
    #[schema(value_type = String)]
    pub fee_amount: BigDecimal,
    #[schema(value_type = Option<String>)]
    pub paid_amount: Option<BigDecimal>,
    pub required_confirmations: u32,
    pub status: String,
    pub tx_hash: Option<String>,
    #[serde(with = "unix_millis")]
    #[schema(value_type = i64)]
    pub created_at: DateTime<Utc>,
    #[serde(with = "unix_millis")]
    #[schema(value_type = i64)]
    pub expires_at: DateTime<Utc>,
    #[serde(with = "option_unix_millis")]
    #[schema(value_type = Option<i64>)]
    pub paid_at: Option<DateTime<Utc>>,
    #[serde(with = "unix_millis")]
    #[schema(value_type = i64)]
    pub server_time: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct DepositOption {
    pub asset_symbol: String,
    pub network: String,
    pub display_name: String,
    pub precision_scale: i32,
    #[schema(value_type = String)]
    pub min_deposit_amount: BigDecimal,
    #[schema(value_type = String)]
    pub deposit_fee: BigDecimal,
    pub required_confirmations: u32,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct DepositPageQuery {
    pub order_no: Option<String>,
    pub network: Option<String>,
    pub asset_symbol: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DepositPage<T> {
    pub items: Vec<T>,
    pub total: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct ScannerView {
    pub network: String,
    pub enabled: bool,
    pub endpoint_configured: bool,
    pub api_key_configured: bool,
    pub confirmations: u32,
    pub revision: u64,
    #[serde(with = "option_unix_millis")]
    #[schema(value_type = Option<i64>)]
    pub last_scan_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct ChainAssetView {
    pub id: u64,
    pub network: String,
    pub asset_symbol: String,
    pub asset_kind: String,
    pub token_address: String,
    pub decimals: i32,
    pub enabled: bool,
    pub revision: u64,
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct SharedAddressView {
    pub pool_id: u64,
    pub network: String,
    pub address: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DepositSettings {
    pub scanners: Vec<ScannerView>,
    pub assets: Vec<ChainAssetView>,
    pub addresses: Vec<SharedAddressView>,
    pub platform_assets: Vec<DepositPlatformAsset>,
}

/// 充值配置专用币种目录，仅返回选择映射和判断平台充值开关所需的公开字段。
#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct DepositPlatformAsset {
    pub symbol: String,
    pub status: String,
    pub deposit_enabled: bool,
    pub precision_scale: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveScanner {
    pub enabled: bool,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub confirmations: u32,
    pub expected_revision: u64,
    pub reason: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveChainAsset {
    pub network: String,
    pub asset_symbol: String,
    pub asset_kind: String,
    pub token_address: String,
    pub decimals: i32,
    pub enabled: bool,
    pub expected_revision: u64,
    pub reason: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportSharedAddresses {
    pub network: String,
    pub addresses: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveSharedAddress {
    pub enabled: bool,
    pub reason: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewDepositReceipt {
    pub action: String,
    pub order_id: Option<u64>,
    pub reason: String,
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct ReceiptView {
    pub id: u64,
    pub network: String,
    pub tx_hash: String,
    pub event_index: u32,
    pub asset_kind: String,
    pub token_address: String,
    pub address: String,
    #[schema(value_type = String)]
    pub amount: BigDecimal,
    pub block_height: u64,
    pub block_hash: String,
    pub confirmations: u32,
    #[serde(with = "unix_millis")]
    #[schema(value_type = i64)]
    pub block_time: DateTime<Utc>,
    pub status: String,
    pub review_reason: Option<String>,
    pub order_id: Option<u64>,
    pub review_note: Option<String>,
}
