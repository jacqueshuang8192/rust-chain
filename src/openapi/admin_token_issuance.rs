//! 管理员新版发行、持仓与已成交回购文档；金额为十进制文本，时间均为 UTC Unix 毫秒。
use super::{ErrorResponse, token_issuance::IssuancePolicy};
use utoipa::{OpenApi, ToSchema};

#[derive(ToSchema)]
#[serde(deny_unknown_fields)]
struct CreateAdminTokenIssuance {
    asset_id: u64,
    quote_asset_id: u64,
    total_supply: String,
    issue_price: String,
    issuance_policy: IssuancePolicy,
    reason: String,
    idempotency_key: String,
}

#[derive(ToSchema)]
struct AdminTokenProject {
    id: u64,
    asset_id: u64,
    symbol: String,
    lifecycle_status: String,
    total_supply: String,
    issue_price: String,
    quote_asset_id: Option<u64>,
    reserved_supply: String,
    allocated_supply: String,
    remaining_supply: String,
    listed_at: Option<i64>,
    actual_listed_at: Option<i64>,
    unlock_type: String,
    fixed_unlock_at: Option<i64>,
    relative_unlock_seconds: Option<u64>,
    unlock_fee_enabled: bool,
    unlock_fee_rate: Option<String>,
    unlock_fee_basis: Option<String>,
    unlock_fee_asset: Option<u64>,
    status: String,
    post_listing_purchase_enabled: bool,
    post_listing_pair_id: Option<u64>,
    post_listing_pair_status: Option<String>,
    post_listing_pair_symbol: Option<String>,
    issuance_policy: Option<IssuancePolicy>,
    issuer_agent_id: Option<u64>,
    issuer_agent_code: Option<String>,
    quote_symbol: Option<String>,
    next_transition_at: Option<i64>,
    next_transition_attempt_at: Option<i64>,
    transition_error: Option<String>,
    actual_purchase_at: Option<i64>,
}

#[derive(ToSchema)]
struct AdminTokenHolding {
    id: u64,
    project_id: u64,
    symbol: String,
    quote_symbol: String,
    user_id: u64,
    email: Option<String>,
    owner_agent_id: Option<u64>,
    owner_agent_code: Option<String>,
    issuer_agent_id: Option<u64>,
    source_kind: String,
    source_id: u64,
    quantity: String,
    cost: String,
    policy_json: IssuancePolicy,
    status: String,
    acquired_at: i64,
    unlock_at: Option<i64>,
    sold_at: Option<i64>,
}

#[derive(ToSchema)]
struct AdminTokenBuyback {
    id: String,
    project_id: u64,
    symbol: String,
    quote_symbol: String,
    user_id: u64,
    email: Option<String>,
    owner_agent_id: Option<u64>,
    owner_agent_code: Option<String>,
    issuer_agent_id: Option<u64>,
    holding_id: u64,
    quantity: String,
    price: String,
    cost: String,
    gross: String,
    fee: String,
    net: String,
    source: String,
    source_version: String,
    status: String,
    observed_at: i64,
    executed_at: i64,
}

#[derive(ToSchema)]
struct AdminTokenOperation {
    id: u64,
    project_id: u64,
    symbol: String,
    actor_type: String,
    actor_id: Option<u64>,
    action: String,
    reason: String,
    payload_json: serde_json::Value,
    created_at: i64,
}

#[derive(ToSchema)]
struct AdminTokenHoldings {
    holdings: Vec<AdminTokenHolding>,
    total: i64,
}

#[derive(ToSchema)]
struct AdminTokenBuybacks {
    buybacks: Vec<AdminTokenBuyback>,
    total: i64,
}

#[derive(ToSchema)]
struct AdminTokenOperations {
    operations: Vec<AdminTokenOperation>,
    total: i64,
}

/// 需要 new_coin.projects.write。固定预热；同管理员同键同规范化意图返回原项目（可已推进）。
/// 不创建可用余额；项目、固定交易对、停用生成器、零余额钱包与审计原子提交。
#[utoipa::path(post,path="/admin/api/v1/new-coins/issuance",tag="admin-token-issuance",security(("bearerAuth"=[])),request_body=CreateAdminTokenIssuance,
    responses((status=200,body=AdminTokenProject),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=422,description="金额必须为文本，未知字段无效"),(status=500,body=ErrorResponse)))]
fn create() {}

/// 需要 new_coin.locks.read；原合同与实际阶段解禁时刻，只读分页，held/sold。
#[utoipa::path(get,path="/admin/api/v1/new-coins/holdings",tag="admin-token-issuance",security(("bearerAuth"=[])),params(("project_id"=Option<u64>,Query),("user_id"=Option<u64>,Query),("agent_id"=Option<u64>,Query,description="买家直属代理，与发行代理分别筛选"),("issuer_agent_id"=Option<u64>,Query),("email"=Option<String>,Query),("status"=Option<String>,Query),("limit"=Option<u32>,Query,minimum=1,maximum=100),("offset"=Option<u32>,Query,maximum=100000)),responses((status=200,body=AdminTokenHoldings),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=500,body=ErrorResponse)))]
fn holdings() {}

/// 需要 new_coin.unlocks.read；只含已执行 sell，quoted 和 buy 不计入结果或总数。
#[utoipa::path(get,path="/admin/api/v1/new-coins/buybacks",tag="admin-token-issuance",security(("bearerAuth"=[])),params(("project_id"=Option<u64>,Query),("user_id"=Option<u64>,Query),("agent_id"=Option<u64>,Query,description="买家直属代理，与发行代理分别筛选"),("issuer_agent_id"=Option<u64>,Query),("email"=Option<String>,Query),("status"=Option<String>,Query),("limit"=Option<u32>,Query,minimum=1,maximum=100),("offset"=Option<u32>,Query,maximum=100000)),responses((status=200,body=AdminTokenBuybacks),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=500,body=ErrorResponse)))]
fn buybacks() {}

/// 需要 audit.logs.read；按真实角色 admin/agent/scheduler/user 精确检索，不触发重放。
#[utoipa::path(get,path="/admin/api/v1/new-coins/operations",tag="admin-token-issuance",security(("bearerAuth"=[])),params(("project_id"=Option<u64>,Query),("actor_type"=Option<String>,Query),("actor_id"=Option<u64>,Query),("action"=Option<String>,Query),("limit"=Option<u32>,Query,minimum=1,maximum=100),("offset"=Option<u32>,Query,maximum=100000)),responses((status=200,body=AdminTokenOperations),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=500,body=ErrorResponse)))]
fn operations() {}

#[derive(OpenApi)]
#[openapi(paths(create,holdings,buybacks,operations),components(schemas(CreateAdminTokenIssuance,AdminTokenProject,AdminTokenHolding,AdminTokenBuyback,AdminTokenOperation,AdminTokenHoldings,AdminTokenBuybacks,AdminTokenOperations)),tags((name="admin-token-issuance",description="管理员新版发行与账务查询")))]
pub(super) struct AdminTokenIssuanceApiDoc;
