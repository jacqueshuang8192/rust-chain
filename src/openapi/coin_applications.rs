//! 代理发币申请、本人历史与管理员审核契约；审批前不公开资产和项目。
use super::ErrorResponse;
use utoipa::{OpenApi, ToSchema};
#[derive(ToSchema)]
struct CoinApplicationInput {
    symbol: String,
    name: String,
    #[schema(minimum = 0, maximum = 18)]
    precision_scale: i32,
    total_supply: String,
    issue_price: String,
    quote_asset_id: u64,
    /// Unix毫秒；计划时间不会触发实际上市。
    listed_at: i64,
    reason: String,
    idempotency_key: String,
}
#[derive(ToSchema)]
struct CoinApplicationRecord {
    id: u64,
    agent_id: u64,
    agent_admin_id: u64,
    symbol: String,
    name: String,
    precision_scale: i32,
    total_supply: String,
    issue_price: String,
    quote_asset_id: u64,
    quote_symbol: String,
    listed_at: i64,
    reason: String,
    #[schema(pattern = "^(pending|approved|rejected)$")]
    status: String,
    #[schema(required = true)]
    project_id: Option<u64>,
    #[schema(required = true)]
    reviewed_by: Option<u64>,
    #[schema(required = true)]
    review_reason: Option<String>,
    created_at: i64,
    #[schema(required = true)]
    reviewed_at: Option<i64>,
}
#[derive(ToSchema)]
struct CoinApplicationPage {
    applications: Vec<CoinApplicationRecord>,
    total: u64,
}
#[derive(ToSchema)]
struct CoinApplicationDecision {
    #[schema(pattern = "^(approve|reject)$")]
    decision: String,
    reason: String,
}
#[derive(ToSchema)]
struct CoinQuoteAsset {
    asset_id: u64,
    symbol: String,
    precision_scale: i32,
}

/// 启用计价资产目录；仅提示，写入时重验。
#[utoipa::path(get,path="/agent/api/v1/coin-applications/options",tag="coin-applications",security(("bearerAuth"=[])),
responses((status=200,body=[CoinQuoteAsset]),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=422,description="无效JSON字段或金额类型"),(status=500,body=ErrorResponse)))]
fn options() {}

/// 代理提交不可变申请；同账号同键重试返回原结果，不创建正式资产或项目。
#[utoipa::path(post,path="/agent/api/v1/coin-applications",tag="coin-applications",security(("bearerAuth"=[])),
request_body=CoinApplicationInput,
responses((status=200,body=CoinApplicationRecord),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=422,description="无效JSON字段或金额类型"),(status=500,body=ErrorResponse)))]
fn submit() {}

/// 当前认证门户账号本人的申请；不可传入agent_id扩大范围。
#[utoipa::path(get,path="/agent/api/v1/coin-applications",tag="coin-applications",security(("bearerAuth"=[])),
params(("status"=Option<String>,Query,description="pending|approved|rejected"),("limit"=Option<u32>,Query,minimum=1,maximum=100),("offset"=Option<u32>,Query,minimum=0,maximum=100000)),
responses((status=200,body=CoinApplicationPage),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=422,description="无效JSON字段或金额类型"),(status=500,body=ErrorResponse)))]
fn own() {}

/// 本人明细；其他账号申请与不存在统一404。
#[utoipa::path(get,path="/agent/api/v1/coin-applications/{id}",tag="coin-applications",security(("bearerAuth"=[])),
params(("id"=u64,Path,description="申请编号")),
responses((status=200,body=CoinApplicationRecord),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=422,description="无效JSON字段或金额类型"),(status=500,body=ErrorResponse)))]
fn own_detail() {}

/// 管理员队列，要求new_coin.applications.read。
#[utoipa::path(get,path="/admin/api/v1/agent-coin-applications",tag="coin-applications",security(("bearerAuth"=[])),
params(("status"=Option<String>,Query,description="pending|approved|rejected"),("limit"=Option<u32>,Query,minimum=1,maximum=100),("offset"=Option<u32>,Query,minimum=0,maximum=100000)),
responses((status=200,body=CoinApplicationPage),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=422,description="无效JSON字段或金额类型"),(status=500,body=ErrorResponse)))]
fn queue() {}

/// 管理员权威明细，要求new_coin.applications.read。
#[utoipa::path(get,path="/admin/api/v1/agent-coin-applications/{id}",tag="coin-applications",security(("bearerAuth"=[])),
params(("id"=u64,Path,description="申请编号")),
responses((status=200,body=CoinApplicationRecord),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=422,description="无效JSON字段或金额类型"),(status=500,body=ErrorResponse)))]
fn detail() {}

/// 要求new_coin.applications.review；批准原子创建资产和预热项目，驳回不发布；终态只允许原审核人同决定同原因重放。
#[utoipa::path(post,path="/admin/api/v1/agent-coin-applications/{id}/review",tag="coin-applications",security(("bearerAuth"=[])),
params(("id"=u64,Path,description="申请编号")),
request_body=CoinApplicationDecision,
responses((status=200,body=CoinApplicationRecord),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=422,description="无效JSON字段或金额类型"),(status=500,body=ErrorResponse)))]
fn review() {}

#[derive(OpenApi)]
#[openapi(paths(options,submit,own,own_detail,queue,detail,review),components(schemas(CoinApplicationInput,CoinApplicationRecord,CoinApplicationPage,CoinApplicationDecision,CoinQuoteAsset)),tags((name="coin-applications",description="代理发行审核")))]
pub(super) struct CoinApplicationsApiDoc;
