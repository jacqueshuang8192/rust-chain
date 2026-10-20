//! 第二版发币合同、代理批次和用户报价的公开镜像；真实身份只从认证令牌取得。
use super::ErrorResponse;
use utoipa::{OpenApi, ToSchema};

#[derive(ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct IssuancePolicy {
    #[schema(minimum = 2, maximum = 2)]
    version: u8,
    /// 配置人的 IANA 时区；所有 at 均为 UTC Unix 毫秒。
    timezone: String,
    purchase_initial_price: String,
    /// 参考展示值，不改变认购或上市行情。
    listing_reference_price: String,
    subscription: PhaseNode,
    distribution: PhaseNode,
    purchase: PhaseNode,
    listing: PhaseNode,
    lock: HoldingLock,
    fee: ExitFee,
}
#[derive(ToSchema)]
struct PhaseNode {
    automatic: bool,
    at: i64,
}
#[derive(ToSchema, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum HoldingLock {
    None,
    OnPurchase,
    OnListing,
    Weeks { count: u32 },
    Months { count: u32 },
    Fixed { at: i64 },
}
#[derive(ToSchema, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ExitFee {
    None,
    Profit { rate: String },
    Gross { rate: String },
    Fixed { amount: String },
}
#[derive(ToSchema)]
struct TokenProject {
    id: u64,
    asset_id: u64,
    quote_asset_id: Option<u64>,
    symbol: String,
    lifecycle_status: String,
    status: String,
    issuer_agent_id: Option<u64>,
    post_listing_pair_id: Option<u64>,
    issuance_policy: Option<IssuancePolicy>,
    actual_purchase_at: Option<i64>,
    actual_listed_at: Option<i64>,
    next_transition_at: Option<i64>,
    transition_error: Option<String>,
}
#[derive(ToSchema)]
struct TokenProjects {
    projects: Vec<TokenProject>,
    next_before_id: Option<u64>,
}
#[derive(ToSchema)]
#[serde(deny_unknown_fields)]
struct StageInput {
    expected_stage: String,
    target_stage: String,
    reason: String,
}
#[derive(ToSchema)]
#[serde(deny_unknown_fields)]
struct AllocationInput {
    #[schema(min_items = 1, max_items = 100)]
    subscription_ids: Vec<u64>,
    /// 0..100 十进制文本，每批明确指定；代币向下取整，未获配部分按原价退款。
    percent: String,
    reason: String,
    idempotency_key: String,
}
#[derive(ToSchema)]
struct AllocationResult {
    subscription_id: u64,
    project_id: u64,
    user_id: u64,
    percent: String,
    quantity: String,
    settled_quote_amount: String,
    refunded_quote_amount: String,
    holding_id: Option<u64>,
    status: String,
}
#[derive(ToSchema)]
struct AllocationReceipt {
    idempotency_key: String,
    results: Vec<AllocationResult>,
}
#[derive(ToSchema)]
#[serde(deny_unknown_fields)]
struct QuoteInput {
    #[schema(pattern = "^(buy|sell)$")]
    side: String,
    /// 买入填数量，卖出不填。
    quantity: Option<String>,
    /// 卖出整个持仓批次；买入不填。
    holding_id: Option<u64>,
    idempotency_key: String,
}
#[derive(ToSchema)]
#[serde(deny_unknown_fields)]
struct MarketPurchaseInput {
    quantity: String,
    idempotency_key: String,
}
#[derive(ToSchema)]
struct MarketPurchaseReceipt {
    idempotency_key: String,
    receipt: TradeReceipt,
}
#[derive(ToSchema)]
struct TradeReceipt {
    quote_id: String,
    project_id: u64,
    side: String,
    quantity: String,
    price: String,
    gross: String,
    fee: String,
    net: String,
    holding_id: u64,
    executed_at: i64,
}
#[derive(ToSchema)]
struct TradeQuote {
    id: String,
    user_id: u64,
    project_id: u64,
    holding_id: Option<u64>,
    side: String,
    quantity: String,
    price: String,
    gross: String,
    cost: String,
    fee: String,
    net: String,
    source: String,
    source_version: String,
    observed_at: i64,
    expires_at: i64,
    status: String,
    receipt_json: Option<TradeReceipt>,
}
#[derive(ToSchema)]
struct Holding {
    id: u64,
    user_id: u64,
    project_id: u64,
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
struct HoldingView {
    holding: Holding,
    symbol: String,
    quote_symbol: String,
    pair_id: Option<u64>,
    pair_symbol: Option<String>,
    lifecycle_status: String,
    eligible: bool,
    unlock_at: Option<i64>,
    market_price: Option<String>,
    market_value: Option<String>,
    profit: Option<String>,
}
#[derive(ToSchema)]
struct HoldingsPage {
    holdings: Vec<HoldingView>,
    checked_at: i64,
    next_before_id: Option<u64>,
}

/// 认证代理本人发行项目，按项目ID倒序，最多100条；与买家团队归属无关。
#[utoipa::path(get,path="/agent/api/v1/token-projects",tag="token-issuance",security(("bearerAuth"=[])),params(("before_id"=Option<u64>,Query)),responses((status=200,body=TokenProjects),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=500,body=ErrorResponse)))]
fn projects() {}
/// 发行代理按实际阶段顺序推进；自动派发节点仅开放派发阶段。已完成目标重放不重复执行。
#[utoipa::path(post,path="/agent/api/v1/token-projects/{id}/stage",tag="token-issuance",security(("bearerAuth"=[])),params(("id"=u64,Path)),request_body=StageInput,responses((status=200,body=TokenProject),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=422,description="无效输入"),(status=500,body=ErrorResponse)))]
fn stage() {}
/// 仅下级买家子树的显式选中订单，同批比例一次结算；同门户同键重试返回原凭证，任何失败整批回滚。
#[utoipa::path(post,path="/agent/api/v1/new-coin-allocations",tag="token-issuance",security(("bearerAuth"=[])),request_body=AllocationInput,responses((status=200,body=AllocationReceipt),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=403,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=422,description="无效输入"),(status=500,body=ErrorResponse)))]
fn allocate() {}
/// 本人发行持仓，每页最多100条，next_before_id为空表示结束。无有效平台行情时估值为空；eligible只表示锁仓已成熟，不保证行情或平台余额充足。
#[utoipa::path(get,path="/api/v1/new-coins/holdings",tag="token-issuance",security(("bearerAuth"=[])),params(("project_id"=Option<u64>,Query),("before_id"=Option<u64>,Query)),responses((status=200,body=HoldingsPage),(status=401,body=ErrorResponse),(status=500,body=ErrorResponse)))]
fn holdings() {}
/// 认购阶段按实时平台报价买入；解禁卖出按原持仓合同扣费。15秒有效，旧键只重放原报价而不刷新价格。
#[utoipa::path(post,path="/api/v1/new-coins/{symbol}/quotes",tag="token-issuance",security(("bearerAuth"=[])),params(("symbol"=String,Path)),request_body=QuoteInput,responses((status=200,body=TradeQuote),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=422,description="无效输入"),(status=500,body=ErrorResponse)))]
fn quote() {}
/// 确认该用户原报价；锁仓核销、平台回购、费用与到账原子完成。已成交报价无条件重放原收据，未成交过期报价返回409。
#[utoipa::path(post,path="/api/v1/new-coins/quotes/{id}/confirm",tag="token-issuance",security(("bearerAuth"=[])),params(("id"=String,Path)),responses((status=200,body=TradeReceipt),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=500,body=ErrorResponse)))]
fn confirm() {}
/// 认购按提交时有效平台行情直接买入；价格由服务端获取，原子结算并以原键重放。
#[utoipa::path(post,path="/api/v1/new-coins/{symbol}/market-purchases",tag="token-issuance",security(("bearerAuth"=[])),params(("symbol"=String,Path)),request_body=MarketPurchaseInput,responses((status=200,body=MarketPurchaseReceipt),(status=400,body=ErrorResponse),(status=401,body=ErrorResponse),(status=404,body=ErrorResponse),(status=409,body=ErrorResponse),(status=422,description="无效输入"),(status=500,body=ErrorResponse)))]
fn market_purchase() {}
#[derive(OpenApi)]
#[openapi(paths(projects,stage,allocate,holdings,quote,confirm,market_purchase),components(schemas(IssuancePolicy,PhaseNode,HoldingLock,ExitFee,TokenProject,TokenProjects,StageInput,AllocationInput,AllocationResult,AllocationReceipt,QuoteInput,MarketPurchaseInput,MarketPurchaseReceipt,TradeReceipt,TradeQuote,Holding,HoldingView,HoldingsPage)),tags((name="token-issuance",description="实时新币发行、分批派发与原子回购")))]
pub(super) struct TokenIssuanceApiDoc;
