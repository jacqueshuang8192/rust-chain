//! 钱包的 OpenAPI 契约：覆盖用户端充值目录与提现申请，以及后台的提现审核、充值账务与提现网络管理。
//! 提现按状态机推进，审核通过、驳回、广播、确认与失败各对应一个端点，状态不匹配统一返回冲突。
//! 资金在申请时冻结，只有确认成功才最终扣除；驳回与广播前失败会解冻退回，已广播的不允许自动解冻。
//! 链上充值以观察事件的方式幂等入账，链重组则通过冲正端点处理，余额不足时转入人工处理。

use super::*;

#[derive(ToSchema)]
pub(super) struct CreateWithdrawalRequest {
    quote_id: String,
    asset_symbol: String,
    network: Option<String>,
    address: String,
    amount: String,
    fee: String,
    idempotency_key: String,
    fund_password: Option<String>,
    totp_code: Option<String>,
}

#[derive(ToSchema)]
pub(super) struct CreateWithdrawalQuoteRequest {
    asset_symbol: String,
    network: String,
    amount: String,
}

#[derive(ToSchema)]
pub(super) struct WithdrawalQuoteResponse {
    quote_id: String,
    asset_symbol: String,
    network: String,
    amount: String,
    fee: String,
    net: String,
    total_reserved: String,
    fee_config_version: String,
    #[schema(format = Int64)]
    expires_at: i64,
}

#[derive(ToSchema)]
pub(super) struct WithdrawalRequestResponse {
    id: u64,
    quote_id: String,
    status: String,
    asset_symbol: String,
    network: String,
    amount: String,
    fee: String,
    net: String,
    total_reserved: String,
    fee_config_version: String,
    #[schema(format = Int64)]
    expires_at: i64,
    security_method: SecurityVerificationMethod,
}

#[derive(ToSchema)]
pub(super) struct WalletWithdrawalResponse {
    id: u64,
    user_id: u64,
    asset_id: u64,
    asset_symbol: String,
    network: Option<String>,
    address: String,
    amount: String,
    fee: String,
    total_reserved: String,
    status: String,
    security_method: String,
    idempotency_key: String,
    gateway_request_id: String,
    withdrawal_quote_id: Option<String>,
    tx_hash: Option<String>,
    block_height: Option<u64>,
    confirmations: u32,
    failure_reason: Option<String>,
    broadcast_error_class: Option<String>,
    broadcast_last_error: Option<String>,
    broadcast_resolution: Option<String>,
    #[schema(format = Int64)]
    acceptance_evidence_at: Option<i64>,
    review_reason: Option<String>,
    required_approvals: u32,
    approval_count: i64,
    reviewed_by: Option<u64>,
    broadcasted_by: Option<u64>,
    confirmed_by: Option<u64>,
    failed_by: Option<u64>,
    retry_count: u32,
    gateway_query_count: u32,
    #[schema(format = Int64)]
    reviewed_at: Option<i64>,
    #[schema(format = Int64)]
    broadcast_at: Option<i64>,
    #[schema(format = Int64)]
    confirmed_at: Option<i64>,
    #[schema(format = Int64)]
    failed_at: Option<i64>,
    #[schema(format = Int64)]
    released_at: Option<i64>,
    #[schema(format = Int64)]
    last_gateway_query_at: Option<i64>,
    #[schema(format = Int64)]
    manual_review_at: Option<i64>,
    #[schema(format = Int64)]
    created_at: i64,
}

#[derive(ToSchema)]
pub(super) struct WalletWithdrawalsResponse {
    withdrawals: Vec<WalletWithdrawalResponse>,
}

#[derive(ToSchema)]
pub(super) struct AdminWalletWithdrawalsResponse {
    withdrawals: Vec<WalletWithdrawalResponse>,
    total: i64,
}

#[derive(ToSchema)]
pub(super) struct ReviewWithdrawalRequest {
    reason: Option<String>,
}

#[derive(ToSchema)]
pub(super) struct BroadcastWithdrawalRequest {
    tx_hash: String,
    block_height: Option<u64>,
    confirmations: Option<u32>,
}

#[derive(ToSchema)]
pub(super) struct ConfirmWithdrawalRequest {
    block_height: Option<u64>,
    confirmations: Option<u32>,
}

#[derive(ToSchema)]
pub(super) struct FailWithdrawalRequest {
    reason: String,
}

#[derive(ToSchema)]
pub(super) struct ReverseDepositRequest {
    reason: String,
}

#[derive(ToSchema)]
pub(super) struct WalletDepositEventResponse {
    id: u64,
    user_id: u64,
    asset_id: u64,
    asset_symbol: String,
    network: String,
    address: String,
    memo: Option<String>,
    tx_hash: String,
    event_index: u32,
    amount: String,
    fee_amount: String,
    block_height: Option<u64>,
    confirmations: u32,
    required_confirmations: u32,
    status: String,
    failure_reason: Option<String>,
    #[schema(format = Int64)]
    credited_at: Option<i64>,
    #[schema(format = Int64)]
    reversed_at: Option<i64>,
    #[schema(format = Int64)]
    created_at: i64,
}

#[derive(ToSchema)]
pub(super) struct WalletDepositsResponse {
    deposits: Vec<WalletDepositEventResponse>,
    total: i64,
}

#[derive(ToSchema)]
pub(super) struct WithdrawFeeTierResponse {
    min_amount: String,
    max_amount: Option<String>,
    fee_rate_percent: String,
}

#[derive(ToSchema)]
pub(super) struct DepositAssetResponse {
    symbol: String,
    name: String,
    logo_url: Option<String>,
    precision_scale: i32,
    deposit_enabled: bool,
    withdraw_enabled: bool,
    min_deposit_amount: String,
    deposit_fee: String,
    withdraw_fee: String,
    withdraw_fee_tiers: Vec<WithdrawFeeTierResponse>,
}

#[derive(ToSchema)]
pub(super) struct DepositAssetsResponse {
    assets: Vec<DepositAssetResponse>,
}

/// 返回当前开放普通链上充值的资产清单，供充值页渲染可选币种与网络。
/// 清单由后台资产配置决定，未开放充值的资产不会出现，前端不需要再做一次过滤。
#[utoipa::path(
    get,
    path = "/api/v1/wallet/deposit-assets",
    tag = "wallet",
    summary = "查询当前支持普通充值的资产",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "查询成功", body = DepositAssetsResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "鉴权 scope 不匹配", body = ErrorResponse),
        (status = 500, description = "服务内部错误", body = ErrorResponse)
    )
)]
fn list_deposit_assets() {}

/// 返回当前开放提现的资产清单，与充值清单各自独立，可能出现只能充值不能提现的资产。
/// 响应结构复用充值资产的定义，字段含义一致，差别只在筛选口径。
#[utoipa::path(
    get,
    path = "/api/v1/wallet/withdraw-assets",
    tag = "wallet",
    summary = "查询当前支持提现的资产",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "查询成功", body = DepositAssetsResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "鉴权 scope 不匹配", body = ErrorResponse),
        (status = 500, description = "服务内部错误", body = ErrorResponse)
    )
)]
fn list_withdraw_assets() {}

/// 用当前资产精度与阶梯费率生成有限期报价，提交时必须带回同一 quote_id。
#[utoipa::path(
    post,
    path = "/api/v1/wallet/withdrawals/quote",
    tag = "wallet",
    summary = "生成服务端权威提现报价",
    request_body = CreateWithdrawalQuoteRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "报价成功", body = WithdrawalQuoteResponse),
        (status = 400, description = "参数或资产配置错误", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 500, description = "服务内部错误", body = ErrorResponse)
    )
)]
fn create_withdrawal_quote() {}

/// 提交提现申请，按后台安全策略校验资金密码等要件后冻结相应资金并进入待审核状态。
/// 资金密码错误返回未认证，安全要件缺失返回参数错误；本步只冻结资金，不做任何链上广播。
#[utoipa::path(
    post,
    path = "/api/v1/wallet/withdrawals",
    tag = "wallet",
    summary = "创建提现申请并按后台策略完成安全校验",
    request_body = CreateWithdrawalRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "创建成功", body = WithdrawalRequestResponse),
        (status = 400, description = "参数错误或安全校验缺失", body = ErrorResponse),
        (status = 401, description = "未登录或资金密码错误", body = ErrorResponse),
        (status = 403, description = "鉴权 scope 不匹配", body = ErrorResponse),
        (status = 500, description = "服务内部错误", body = ErrorResponse)
    )
)]
fn create_withdrawal_request() {}

/// 查询当前用户自己的提现申请，可按状态过滤并限制返回条数，用于提现记录页。
/// 只返回令牌对应用户的数据，不接受用户标识参数，因此无法借此查看他人提现。
#[utoipa::path(
    get,
    path = "/api/v1/wallet/withdrawals",
    tag = "wallet",
    summary = "查询当前用户提现申请",
    params(
        ("status" = Option<String>, Query, description = "提现状态"),
        ("limit" = Option<u32>, Query, description = "返回数量")
    ),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "查询成功", body = WalletWithdrawalsResponse),
        (status = 400, description = "状态参数错误", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 500, description = "服务内部错误", body = ErrorResponse)
    )
)]
fn list_user_withdrawals() {}

/// 后台分页查询全站提现申请，可按状态与用户过滤，是提现审核工作台的数据来源。
/// 返回的后台视图比用户端多出审核相关字段，需要后台作用域令牌才能访问。
#[utoipa::path(
    get,
    path = "/admin/api/v1/wallet/withdrawals",
    tag = "admin-wallet",
    summary = "后台查询提现申请",
    params(
        ("status" = Option<String>, Query, description = "提现状态"),
        ("user_id" = Option<u64>, Query, description = "用户 ID"),
        ("limit" = Option<u32>, Query, description = "返回数量"),
        ("offset" = Option<u32>, Query, description = "分页偏移")
    ),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "查询成功", body = AdminWalletWithdrawalsResponse),
        (status = 400, description = "参数错误", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "无后台权限", body = ErrorResponse)
    )
)]
fn list_admin_wallet_withdrawals() {}

/// 审核通过一笔提现申请，使其从待审核推进到可广播状态，资金保持冻结不做实际扣减。
/// 当前状态不允许该迁移时返回冲突，避免重复审核导致状态回退。
#[utoipa::path(
    post,
    path = "/admin/api/v1/wallet/withdrawals/{id}/approve",
    tag = "admin-wallet",
    summary = "审核通过提现申请",
    params(("id" = u64, Path, description = "提现申请 ID")),
    request_body = ReviewWithdrawalRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "审核成功", body = WalletWithdrawalResponse),
        (status = 409, description = "状态冲突", body = ErrorResponse)
    )
)]
fn approve_admin_wallet_withdrawal() {}

/// 驳回提现申请并把先前冻结的资金解冻退回用户可用余额，是审核环节的终态之一。
/// 状态不允许驳回时返回冲突，防止对已经广播的申请误做解冻。
#[utoipa::path(
    post,
    path = "/admin/api/v1/wallet/withdrawals/{id}/reject",
    tag = "admin-wallet",
    summary = "拒绝提现并解冻资金",
    params(("id" = u64, Path, description = "提现申请 ID")),
    request_body = ReviewWithdrawalRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "拒绝成功", body = WalletWithdrawalResponse),
        (status = 409, description = "状态冲突", body = ErrorResponse)
    )
)]
fn reject_admin_wallet_withdrawal() {}

/// 登记提现已在链上广播的结果，通常需要带上交易哈希，使申请进入等待确认阶段。
/// 登记之后资金不能再自动解冻，后续只能走确认成功或转人工处理两条路。
#[utoipa::path(
    post,
    path = "/admin/api/v1/wallet/withdrawals/{id}/broadcast",
    tag = "admin-wallet",
    summary = "登记提现链上广播结果",
    params(("id" = u64, Path, description = "提现申请 ID")),
    request_body = BroadcastWithdrawalRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "登记成功", body = WalletWithdrawalResponse),
        (status = 409, description = "状态冲突", body = ErrorResponse)
    )
)]
fn broadcast_admin_wallet_withdrawal() {}

/// 确认提现在链上最终成功，把此前冻结的资金正式扣除，完成整笔提现的资金闭环。
/// 这是不可逆的终态操作，状态不符时返回冲突，不会造成重复扣款。
#[utoipa::path(
    post,
    path = "/admin/api/v1/wallet/withdrawals/{id}/confirm",
    tag = "admin-wallet",
    summary = "确认提现并最终扣除冻结资金",
    params(("id" = u64, Path, description = "提现申请 ID")),
    request_body = ConfirmWithdrawalRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "确认成功", body = WalletWithdrawalResponse),
        (status = 409, description = "状态冲突", body = ErrorResponse)
    )
)]
fn confirm_admin_wallet_withdrawal() {}

/// 把尚未广播的提现标记为失败并解冻资金，用于签名失败等链下环节出错的情形。
/// 已广播的申请不允许走这条自动解冻路径，会直接返回冲突要求人工核对链上状态。
#[utoipa::path(
    post,
    path = "/admin/api/v1/wallet/withdrawals/{id}/fail",
    tag = "admin-wallet",
    summary = "标记广播前提现失败并解冻资金",
    params(("id" = u64, Path, description = "提现申请 ID")),
    request_body = FailWithdrawalRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "失败处理成功", body = WalletWithdrawalResponse),
        (status = 409, description = "已广播请求不可自动解冻", body = ErrorResponse)
    )
)]
fn fail_admin_wallet_withdrawal() {}

/// 后台分页查询链上充值事件，可按用户过滤，用于核对入账情况与排查漏充。
/// 返回的是充值事件记录本身，包含确认数与处理状态，而不是钱包余额流水。
#[utoipa::path(
    get,
    path = "/admin/api/v1/wallet/deposits",
    tag = "admin-wallet",
    summary = "后台查询链上充值事件",
    params(
        ("user_id" = Option<u64>, Query, description = "用户 ID"),
        ("limit" = Option<u32>, Query, description = "返回数量"),
        ("offset" = Option<u32>, Query, description = "分页偏移")
    ),
    security(("bearerAuth" = [])),
    responses((status = 200, description = "查询成功", body = WalletDepositsResponse))
)]
fn list_admin_wallet_deposits() {}

/// 在链重组导致已入账充值失效时发起冲正，可能直接扣回，也可能因余额不足转入人工处理。
/// 充值当前状态不允许冲正时返回冲突，避免对未入账或已冲正的事件重复操作。
#[utoipa::path(
    post,
    path = "/admin/api/v1/wallet/deposits/{id}/reverse",
    tag = "admin-wallet",
    summary = "链重组充值冲正",
    params(("id" = u64, Path, description = "充值事件 ID")),
    request_body = ReverseDepositRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "冲正成功或进入人工处理", body = WalletDepositEventResponse),
        (status = 409, description = "充值状态不允许冲正", body = ErrorResponse)
    )
)]
fn reverse_admin_wallet_deposit() {}
