//! 代理团队订单只读契约；复用逐用户业务快照，不暴露幂等键或资金操作入口。

use super::{
    ErrorResponse,
    agent_portal::{
        AgentUserMarginPositionResponse, AgentUserSecondsContractOrderResponse,
        AgentUserSpotOrderResponse,
    },
};
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

#[derive(ToSchema, Serialize)]
struct AgentTeamOrderIdentity {
    #[schema(required = true)]
    email: Option<String>,
    #[schema(minimum = 1, maximum = 9007199254740991u64)]
    owner_agent_id: u64,
    owner_agent_code: String,
}

#[derive(ToSchema)]
struct AgentTeamEarnOrder {
    id: u64,
    user_id: u64,
    product_id: u64,
    product_name: String,
    asset_id: u64,
    asset_symbol: String,
    /// Decimal 字符串，禁止转为浮点金额。
    amount: String,
    /// 申购时的年化比例快照，Decimal 字符串。
    apr_rate: String,
    #[schema(minimum = 1)]
    term_days: u32,
    #[schema(pattern = "^(subscribed|redeemed)$")]
    status: String,
    /// 对应 subscribed_at，Unix 毫秒时间戳。
    #[schema(format = Int64)]
    created_at: i64,
    #[schema(format = Int64)]
    matures_at: i64,
    #[schema(format = Int64, required = true)]
    redeemed_at: Option<i64>,
}

#[derive(ToSchema)]
struct AgentTeamNewCoinSubscription {
    issuance_version: u64,
    project_lifecycle_status: String,
    base_asset_precision: i32,
    quote_asset_precision: i32,
    id: u64,
    user_id: u64,
    project_id: u64,
    symbol: String,
    quote_asset: u64,
    quote_symbol: String,
    quote_amount: String,
    #[schema(format = Int64)]
    created_at: i64,
    issue_price: String,
    #[schema(pattern = "^(pending|allocated|partial_allocated|refunded|available)$")]
    status: String,
    #[schema(pattern = "^(manual_distribution|legacy_instant)$")]
    settlement_mode: String,
    frozen_quote_amount: String,
    #[schema(required = true)]
    settled_quote_amount: Option<String>,
    #[schema(required = true)]
    refunded_quote_amount: Option<String>,
    requested_quantity: String,
    allocated_quantity: String,
}

#[derive(ToSchema)]
struct AgentTeamNewCoinPurchase {
    id: u64,
    user_id: u64,
    project_id: u64,
    symbol: String,
    quote_asset: u64,
    quote_symbol: String,
    quote_amount: String,
    #[schema(format = Int64)]
    created_at: i64,
    #[schema(pattern = "^(pending|locked|available)$")]
    status: String,
    pair_id: u64,
    base_asset: u64,
    price: String,
    quantity: String,
    #[schema(required = true)]
    lock_position_id: Option<u64>,
}

#[derive(ToSchema)]
struct AgentTeamOrder<T> {
    #[serde(flatten)]
    identity: AgentTeamOrderIdentity,
    #[serde(flatten)]
    order: T,
}

#[derive(ToSchema)]
struct AgentTeamOrdersPage<T> {
    orders: Vec<AgentTeamOrder<T>>,
    #[schema(minimum = 0)]
    total: i64,
}

#[derive(ToSchema)]
#[serde(untagged)]
enum AgentTeamOrdersResponse {
    Seconds(AgentTeamOrdersPage<AgentUserSecondsContractOrderResponse>),
    Margin(AgentTeamOrdersPage<AgentUserMarginPositionResponse>),
    Earn(AgentTeamOrdersPage<AgentTeamEarnOrder>),
    Spot(AgentTeamOrdersPage<AgentUserSpotOrderResponse>),
    NewCoinSubscription(AgentTeamOrdersPage<AgentTeamNewCoinSubscription>),
    NewCoinPurchase(AgentTeamOrdersPage<AgentTeamNewCoinPurchase>),
}

/// 默认读取当前代理及全部下级代理用户，列表和 total 同时受令牌子树与筛选限制。
/// agent_id 只选择当前子树中的代理分支（含其下级）；父级、兄弟或其他树返回空集。
/// 按创建时间和记录 ID 倒序。理财使用申购时间，不计息、不赎回；所有接口均不修改资金或订单。
/// margin 的 pending 表示 opened 且 entry_price=null，opened 筛选仅包含已成交仓位，响应仍用存储状态。
/// 同一次响应的 orders 全部属于 kind 指定业务。所有金额为 Decimal 字符串，所有时间为 Unix 毫秒。
#[utoipa::path(get, path = "/agent/api/v1/orders/{kind}", tag = "agent-portal",
    security(("bearerAuth" = [])),
    params(
        ("kind" = String, Path, description = "seconds | margin | earn | spot | new-coin-subscriptions | new-coin-purchases"),
        ("team_scope" = Option<String>, Query, description = "all（默认）| own（本级）| descendants（下级）"),
        ("agent_id" = Option<u64>, Query, description = "代理分支 ID，含其下级；正安全整数"),
        ("user_id" = Option<u64>, Query, description = "用户 ID，正安全整数"),
        ("email" = Option<String>, Query, description = "邮箱字面包含，不解析通配符，最多254字节"),
        ("order_id" = Option<u64>, Query, description = "内部记录 ID，非展示订单号，正安全整数"),
        ("product_id" = Option<u64>, Query, description = "产品 ID，仅 seconds/margin/earn，正安全整数"),
        ("project_id" = Option<u64>, Query, description = "新币项目ID，仅 new-coin-subscriptions/new-coin-purchases，正安全整数"),
        ("symbol" = Option<String>, Query, description = "交易对，忽略大小写和 - / _ 分隔符；不适用于 earn；新币为币种代码（仅忽略大小写）"),
        ("status" = Option<String>, Query, description = "省略为全部；seconds: opened|settled|manual_review|refunded；margin: pending|opened|closed|canceled|liquidated；earn: subscribed|redeemed；spot: pending|open|partially_filled|filled|cancelled|rejected；new-coin-subscriptions: pending|allocated|partial_allocated|refunded|available（历史）；new-coin-purchases: pending|locked|available"),
        ("direction" = Option<String>, Query, description = "seconds: up|down；margin: long|short；spot: buy|sell；earn 与新币不支持"),
        ("result" = Option<String>, Query, description = "仅 seconds: win|loss"),
        ("created_from" = Option<i64>, Query, description = "开始时间（含），Unix 毫秒，0..253402300799999"),
        ("created_to" = Option<i64>, Query, description = "结束时间（不含），必须大于开始时间"),
        ("limit" = Option<u32>, Query, description = "默认20，1..100"),
        ("offset" = Option<u32>, Query, description = "默认0，0..100000")
    ),
    responses(
        (status = 200, body = AgentTeamOrdersResponse),
        (status = 400, description = "未知字段、非法筛选或越界分页"),
        (status = 401, description = "会话或代理祖先链不可用", body = ErrorResponse),
        (status = 403, description = "不是代理身份", body = ErrorResponse),
        (status = 404, description = "订单类型不存在", body = ErrorResponse),
        (status = 500, body = ErrorResponse)
    ))]
fn list_orders() {}

#[derive(OpenApi)]
#[openapi(paths(list_orders), components(schemas(
    AgentTeamOrdersResponse, AgentTeamOrderIdentity, AgentTeamEarnOrder,
    AgentTeamNewCoinSubscription, AgentTeamNewCoinPurchase,
    AgentTeamOrder<AgentTeamNewCoinSubscription>, AgentTeamOrder<AgentTeamNewCoinPurchase>,
    AgentTeamOrdersPage<AgentTeamNewCoinSubscription>, AgentTeamOrdersPage<AgentTeamNewCoinPurchase>,
    AgentTeamOrder<AgentUserSecondsContractOrderResponse>,
    AgentTeamOrder<AgentUserMarginPositionResponse>,
    AgentTeamOrder<AgentTeamEarnOrder>,
    AgentTeamOrder<AgentUserSpotOrderResponse>,
    AgentTeamOrdersPage<AgentUserSecondsContractOrderResponse>,
    AgentTeamOrdersPage<AgentUserMarginPositionResponse>,
    AgentTeamOrdersPage<AgentTeamEarnOrder>,
    AgentTeamOrdersPage<AgentUserSpotOrderResponse>
)))]
pub(super) struct AgentTeamOrdersApiDoc;
