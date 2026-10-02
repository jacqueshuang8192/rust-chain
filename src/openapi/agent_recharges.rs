//! 代理人工充值契约：令牌子树、独立免审授权与同事务账务审计，金额始终为十进制字符串。

use super::ErrorResponse;
use utoipa::{OpenApi, ToSchema};

#[derive(ToSchema)]
struct AgentRechargeRecord {
    #[schema(minimum = 1, maximum = 9007199254740991u64)]
    id: u64,
    #[schema(minimum = 1, maximum = 9007199254740991u64)]
    agent_id: u64,
    #[schema(minimum = 1, maximum = 9007199254740991u64)]
    agent_admin_id: u64,
    #[schema(minimum = 1, maximum = 9007199254740991u64)]
    user_id: u64,
    #[schema(minimum = 1, maximum = 9007199254740991u64)]
    asset_id: u64,
    asset_symbol: String,
    amount: String,
    reason: String,
    #[schema(pattern = "^(review|direct)$")]
    mode: String,
    #[schema(pattern = "^(pending|credited|rejected)$")]
    status: String,
    #[schema(required = true, minimum = 1, maximum = 9007199254740991u64)]
    reviewed_by: Option<u64>,
    #[schema(required = true)]
    review_reason: Option<String>,
    /// Unix 毫秒时间戳。
    #[schema(format = Int64)]
    created_at: i64,
    /// Unix 毫秒时间戳；未审核为 null。
    #[schema(format = Int64, required = true)]
    reviewed_at: Option<i64>,
    /// Unix 毫秒时间戳；未入账为 null。
    #[schema(format = Int64, required = true)]
    credited_at: Option<i64>,
}

#[derive(ToSchema)]
struct AgentRechargesResponse {
    recharges: Vec<AgentRechargeRecord>,
    #[schema(minimum = 0, maximum = 9007199254740991u64)]
    total: u64,
}

#[derive(ToSchema)]
struct AgentRechargeAsset {
    #[schema(minimum = 1, maximum = 9007199254740991u64)]
    asset_id: u64,
    symbol: String,
    #[schema(minimum = 0, maximum = 18)]
    precision_scale: i32,
}

#[derive(ToSchema)]
struct AgentRechargeOptions {
    direct_recharge_enabled: bool,
    assets: Vec<AgentRechargeAsset>,
}

#[derive(ToSchema)]
struct CreateAgentRechargeRequest {
    #[schema(minimum = 1, maximum = 9007199254740991u64)]
    asset_id: u64,
    /// 正十进制字符串，须符合当前资产精度与 DECIMAL(38,18)，不允许取整。
    amount: String,
    reason: String,
    /// 去除首尾空白后为 1..128 字节，同一代理账号内唯一；绑定用户、资产、金额与原因，未知结果复用原键。
    idempotency_key: String,
}

#[derive(ToSchema)]
struct ReviewAgentRechargeRequest {
    #[schema(pattern = "^(approve|reject)$")]
    decision: String,
    reason: String,
}

#[derive(ToSchema)]
struct AgentRechargePolicy {
    #[schema(minimum = 1, maximum = 9007199254740991u64)]
    agent_id: u64,
    direct_recharge_enabled: bool,
    #[schema(minimum = 0, maximum = 9007199254740991u64)]
    version: u64,
}

#[derive(ToSchema)]
struct UpdateAgentRechargePolicyRequest {
    direct_recharge_enabled: bool,
    #[schema(minimum = 0, maximum = 9007199254740991u64)]
    expected_version: u64,
    reason: String,
}

/// 返回当前代理自身免审状态与有效资产精度；上级授权不继承。
#[utoipa::path(get, path = "/agent/api/v1/recharges/options", tag = "agent-recharges",
    security(("bearerAuth" = [])),
    responses((status = 200, body = AgentRechargeOptions),
        (status = 401, body = ErrorResponse), (status = 403, body = ErrorResponse),
        (status = 500, body = ErrorResponse)))]
fn options() {}

/// 令牌确定发起账号和子树；默认 pending 不动钱包，免审 direct 原子入现货可用余额，不扣代理钱包。
/// 幂等键绑定完整请求；撤销免审后原成功请求仍返回原记录，不重复入账。
#[utoipa::path(post, path = "/agent/api/v1/users/{user_id}/recharge", tag = "agent-recharges",
    params(("user_id" = u64, Path)), request_body = CreateAgentRechargeRequest,
    security(("bearerAuth" = [])),
    responses((status = 200, body = AgentRechargeRecord),
        (status = 400, description = "参数、精度或容量不合法", body = ErrorResponse),
        (status = 401, body = ErrorResponse), (status = 403, body = ErrorResponse),
        (status = 404, description = "目标不存在或不在子树", body = ErrorResponse),
        (status = 409, description = "同键请求参数冲突或状态不可执行", body = ErrorResponse),
        (status = 422, description = "JSON 字段缺失、金额不是字符串或十进制文本无法无损解析"),
        (status = 500, body = ErrorResponse)))]
fn submit() {}

/// 仅返回当前代理账号发起且目标仍在当前子树的申请；列表与 total 同时约束范围。
#[utoipa::path(get, path = "/agent/api/v1/recharges", tag = "agent-recharges",
    params(("status" = Option<String>, Query, description = "pending | credited | rejected"),
        ("user_id" = Option<u64>, Query), ("limit" = Option<u32>, Query),
        ("offset" = Option<u32>, Query)),
    security(("bearerAuth" = [])),
    responses((status = 200, body = AgentRechargesResponse),
        (status = 400, body = ErrorResponse), (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse), (status = 500, body = ErrorResponse)))]
fn history() {}

/// 独立要求 agents.recharges.read；管理员跨代理审核队列按相同条件分页和计数。
#[utoipa::path(get, path = "/admin/api/v1/agent-recharges", tag = "agent-recharges",
    params(("status" = Option<String>, Query, description = "pending | credited | rejected"),
        ("agent_id" = Option<u64>, Query), ("user_id" = Option<u64>, Query),
        ("limit" = Option<u32>, Query), ("offset" = Option<u32>, Query)),
    security(("bearerAuth" = [])),
    responses((status = 200, body = AgentRechargesResponse),
        (status = 400, body = ErrorResponse), (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse), (status = 500, body = ErrorResponse)))]
fn queue() {}

/// 独立要求 agents.recharges.read；按不可变申请 ID 查询权威结果，不依赖队列分页或当前用户归属。
/// 用于确认其他管理员已处理的终态，只读不会重放审批或再次入账。
#[utoipa::path(get, path = "/admin/api/v1/agent-recharges/{id}", tag = "agent-recharges",
    params(("id" = u64, Path)), security(("bearerAuth" = [])),
    responses((status = 200, body = AgentRechargeRecord),
        (status = 400, body = ErrorResponse), (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse), (status = 404, body = ErrorResponse),
        (status = 500, body = ErrorResponse)))]
fn detail() {}

/// 独立要求 agents.recharges.review；批准时锁内重查账号、代理祖先状态与用户归属。
/// 同审核人、同动作、同原因终态重放返回原结果，其余终态操作冲突；改派后允许拒绝清理。
/// 钱包、流水、平衡分录、申请终态与必需审计同事务提交。
#[utoipa::path(post, path = "/admin/api/v1/agent-recharges/{id}/review", tag = "agent-recharges",
    params(("id" = u64, Path)), request_body = ReviewAgentRechargeRequest,
    security(("bearerAuth" = [])),
    responses((status = 200, body = AgentRechargeRecord),
        (status = 400, body = ErrorResponse), (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse), (status = 404, body = ErrorResponse),
        (status = 409, description = "终态冲突或当前申请不可批准", body = ErrorResponse),
        (status = 422, description = "JSON 字段缺失或类型错误"),
        (status = 500, body = ErrorResponse)))]
fn review() {}

/// 授权读取沿用 agents.read；默认 false，授权只影响该代理，不继承到下级。
#[utoipa::path(get, path = "/admin/api/v1/agents/{id}/recharge-policy", tag = "agent-recharges",
    params(("id" = u64, Path)), security(("bearerAuth" = [])),
    responses((status = 200, body = AgentRechargePolicy),
        (status = 401, body = ErrorResponse), (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse), (status = 500, body = ErrorResponse)))]
fn get_policy() {}

/// 独立要求 agents.recharges.authorize；agents.write 不授予此权限。
/// 必需原因及预期版本；撤销即时影响新申请，既有 pending 不自动入账。
#[utoipa::path(patch, path = "/admin/api/v1/agents/{id}/recharge-policy", tag = "agent-recharges",
    params(("id" = u64, Path)), request_body = UpdateAgentRechargePolicyRequest,
    security(("bearerAuth" = [])),
    responses((status = 200, body = AgentRechargePolicy),
        (status = 400, body = ErrorResponse), (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse), (status = 404, body = ErrorResponse),
        (status = 409, description = "授权版本冲突", body = ErrorResponse),
        (status = 422, description = "JSON 字段缺失或类型错误"),
        (status = 500, body = ErrorResponse)))]
fn update_policy() {}

#[derive(OpenApi)]
#[openapi(paths(options, submit, history, queue, detail, review, get_policy, update_policy),
    components(schemas(AgentRechargeRecord, AgentRechargesResponse, AgentRechargeAsset,
        AgentRechargeOptions, CreateAgentRechargeRequest, ReviewAgentRechargeRequest,
        AgentRechargePolicy, UpdateAgentRechargePolicyRequest)),
    tags((name = "agent-recharges", description = "代理人工充值、审核与独立免审授权")))]
pub(super) struct AgentRechargesApiDoc;
