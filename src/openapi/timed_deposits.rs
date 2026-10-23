//! 限时链上充值的公开接口合同；金额均为币种数量字符串，时间均为 UTC 毫秒。
use crate::modules::wallet::presentation::timed_deposits::*;
use utoipa::OpenApi;

#[utoipa::path(get, path="/api/v1/wallet/deposit-options", tag="timed-deposits", security(("bearerAuth"=[])),
    responses((status=200,body=[DepositOption])))]
fn options() {}

#[utoipa::path(post, path="/api/v1/wallet/deposit-orders", tag="timed-deposits", security(("bearerAuth"=[])),
    request_body=CreateDepositOrder,
    responses((status=200,body=DepositOrder)))]
fn create() {}

#[utoipa::path(get, path="/api/v1/wallet/deposit-orders", tag="timed-deposits", security(("bearerAuth"=[])),
    params(("limit"=Option<u32>,Query),("offset"=Option<u32>,Query),("status"=Option<String>,Query),("network"=Option<String>,Query),("asset_symbol"=Option<String>,Query),("order_no"=Option<String>,Query)),
    responses((status=200,body=DepositPage<DepositOrder>)))]
fn list() {}

#[utoipa::path(get, path="/api/v1/wallet/deposit-orders/{id}", tag="timed-deposits", security(("bearerAuth"=[])),
    params(("id"=u64,Path)),
    responses((status=200,body=DepositOrder)))]
fn get() {}

#[utoipa::path(get, path="/admin/api/v1/wallet/deposit-settings", tag="timed-deposits", security(("bearerAuth"=[])),
    responses((status=200,body=DepositSettings)))]
fn settings() {}

#[utoipa::path(put, path="/admin/api/v1/wallet/deposit-settings/scanners/{network}", tag="timed-deposits", security(("bearerAuth"=[])),
    request_body=SaveScanner,
    params(("network"=String,Path)),
    responses((status=204,description="保存成功")))]
fn scanner() {}

#[utoipa::path(put, path="/admin/api/v1/wallet/deposit-settings/assets", tag="timed-deposits", security(("bearerAuth"=[])),
    request_body=SaveChainAsset,
    responses((status=204,description="保存成功")))]
fn asset() {}

#[utoipa::path(put, path="/admin/api/v1/wallet/deposit-settings/addresses/{id}", tag="timed-deposits", security(("bearerAuth"=[])),
    request_body=SaveSharedAddress,
    params(("id"=u64,Path)),
    responses((status=204,description="保存成功")))]
fn address() {}

#[utoipa::path(get, path="/admin/api/v1/wallet/deposit-orders", tag="timed-deposits", security(("bearerAuth"=[])),
    params(("limit"=Option<u32>,Query),("offset"=Option<u32>,Query),("status"=Option<String>,Query),("network"=Option<String>,Query),("asset_symbol"=Option<String>,Query),("order_no"=Option<String>,Query)),
    responses((status=200,body=DepositPage<DepositOrder>)))]
fn admin_list() {}

#[utoipa::path(get, path="/admin/api/v1/wallet/deposit-orders/{id}", tag="timed-deposits", security(("bearerAuth"=[])),
    params(("id"=u64,Path)),
    responses((status=200,body=DepositOrder)))]
fn admin_get() {}

#[utoipa::path(get, path="/admin/api/v1/wallet/deposit-receipts", tag="timed-deposits", security(("bearerAuth"=[])),
    params(("limit"=Option<u32>,Query),("offset"=Option<u32>,Query),("status"=Option<String>,Query),("network"=Option<String>,Query),("asset_symbol"=Option<String>,Query),("order_no"=Option<String>,Query)),
    responses((status=200,body=DepositPage<ReceiptView>)))]
fn receipts() {}

#[utoipa::path(post, path="/admin/api/v1/wallet/deposit-receipts/{id}/review", tag="timed-deposits", security(("bearerAuth"=[])),
    request_body=ReviewDepositReceipt,
    params(("id"=u64,Path)),
    responses((status=204,description="保存成功")))]
fn review() {}

#[derive(OpenApi)]
#[openapi(paths(
    options, create, list, get, settings, scanner, asset, address, admin_list, admin_get, receipts,
    review
))]
pub(super) struct TimedDepositsApiDoc;
