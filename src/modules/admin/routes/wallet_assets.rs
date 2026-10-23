//! 承载资产定义、钱包账户与流水查询以及提现网络目录管理的 HTTP 传输入口。
//!
//! 这组入口横跨「配置」和「资金」两类语义：资产与充值网络属于配置，钱包账户和流水属于资金只读视图，
//! 地址池则介于两者之间，因为地址状态直接决定用户能拿到哪个充值地址。路由本身只做提取与转发，
//! 余额一致性、地址占用与回收的状态机、以及每次写操作的审计留痕全部由应用用例在事务内保证。
//! 本文件不提供任何直接改动用户余额的入口，人工加币在用户资源路由中另行注册。

use super::*;

/// 构建资产、钱包查询、充值网络与地址池的后台传输路由。
///
/// 路由仅负责管理员鉴权、Path/Query/JSON 解析和响应状态映射；涉及余额、流水、地址占用与
/// 审计的不变量全部由应用用例维护。删除资产成功仍映射为 204，其余错误沿既有统一错误响应返回。
/// 地址池单条创建与批量创建注册为两个路径，二者共享同一套网络准入校验但事务边界不同。
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/assets", get(list_assets).post(create_asset))
        .route(
            "/assets/:id",
            get(get_asset).patch(update_asset).delete(delete_asset),
        )
        .route("/wallet/accounts", get(list_wallet_accounts))
        .route("/wallet/ledger", get(list_wallet_ledger))
        .route(
            "/deposit-network-configs",
            get(list_deposit_network_configs).post(create_deposit_network_config),
        )
        .route(
            "/deposit-network-configs/:id",
            patch(update_deposit_network_config),
        )
}

/// 处理 GET /assets，按符号、资产类型和状态筛选平台资产。
/// 筛选值会经过与写入相同的规范化和枚举校验，非法取值直接报校验错误；查询不聚合任何用户余额。
async fn list_assets(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Query(query): Query<AdminAssetQuery>,
) -> AppResult<Json<AdminAssetsResponse>> {
    Ok(Json(
        list_assets_use_case(state.mysql.clone(), query).await?,
    ))
}

/// 处理 GET /assets/:id，读取单个资产的精度、类型、状态、充提开关、费用与阶梯费配置。
/// 查询不加资产锁，资产缺失返回未找到；响应只描述资产定义本身，不含任何用户持仓数据。
async fn get_asset(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Path(asset_id): Path<u64>,
) -> AppResult<Json<AdminAssetResponse>> {
    Ok(Json(
        get_asset_use_case(state.mysql.clone(), asset_id).await?,
    ))
}

/// 处理 PATCH /assets/:id，更新资产展示、精度、类型、状态与充提费用规则。
/// 请求必须携带审计原因；未提交的充提开关、金额和阶梯费沿用锁定后的旧值，资产符号本身不可修改。
/// 调低精度不会重算已有钱包余额，把资产改为停用也不会撤销在途的充提请求，两者都需要另行处理。
async fn update_asset(
    AdminAuth(claims): AdminAuth,
    State(state): State<AppState>,
    Path(asset_id): Path<u64>,
    Json(request): Json<UpdateAssetRequest>,
) -> AppResult<Json<AdminAssetResponse>> {
    let admin_id = admin_id_from_subject(&claims.sub)?;
    Ok(Json(
        update_asset_use_case(state.mysql.clone(), admin_id, asset_id, request).await?,
    ))
}

/// 处理 POST /assets，登记新资产并为全部现有用户批量初始化该资产的钱包账户。
/// 这是本组入口中唯一会顺带写入钱包账户表的写操作，资产插入与账户初始化在同一事务提交，
/// 因此不会出现资产已存在却有用户缺账户的中间态；初始化只建空账户，不产生任何余额流水。
async fn create_asset(
    AdminAuth(claims): AdminAuth,
    State(state): State<AppState>,
    Json(request): Json<CreateAssetRequest>,
) -> AppResult<Json<AdminAssetResponse>> {
    let admin_id = admin_id_from_subject(&claims.sub)?;
    Ok(Json(
        create_asset_use_case(state.mysql.clone(), admin_id, request).await?,
    ))
}

/// 处理 DELETE /assets/:id，删除已停用且无业务引用的资产，成功返回 204 空响应。
/// 该入口虽为 DELETE 但仍需 JSON 请求体来携带必填审计原因；资产状态不是停用时直接返回校验错误。
/// 应用层会先清掉该资产下的零余额钱包账户再检查剩余引用，因此仅有空账户不会阻止资产退场，
/// 但只要还存在非零余额或其他引用就整体回滚，不会留下被清了一半账户的资产。
async fn delete_asset(
    AdminAuth(claims): AdminAuth,
    State(state): State<AppState>,
    Path(asset_id): Path<u64>,
    Json(request): Json<DeleteAssetRequest>,
) -> AppResult<StatusCode> {
    let admin_id = admin_id_from_subject(&claims.sub)?;
    delete_asset_use_case(state.mysql.clone(), admin_id, asset_id, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// 处理 GET /wallet/accounts，按用户、邮箱和资产检索钱包账户余额。
/// include_empty 与 include_internal 两个开关均缺省为 false，即默认隐藏零余额账户和内部账号；
/// 查询不加钱包锁也不计算跨页合计，读取期间余额可能被并发交易改写。
async fn list_wallet_accounts(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Query(query): Query<AdminWalletAccountQuery>,
) -> AppResult<Json<AdminWalletAccountsResponse>> {
    Ok(Json(
        list_wallet_accounts_use_case(state.mysql.clone(), query).await?,
    ))
}

/// 处理 GET /wallet/ledger，按用户、邮箱、资产、变更类型和引用类型检索钱包流水。
/// 与账户余额入口互为对账两侧：这里给出的是逐笔变动记录，可用变更类型和引用类型定位某类业务的入账来源。
/// 查询只读，不会补写缺失流水，也不会按流水重算账户余额。
async fn list_wallet_ledger(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Query(query): Query<AdminWalletLedgerQuery>,
) -> AppResult<Json<AdminWalletLedgerResponseList>> {
    Ok(Json(
        list_wallet_ledger_use_case(state.mysql.clone(), query).await?,
    ))
}

/// 处理 GET /deposit-network-configs，按网络、地址组、状态和资产符号检索提现网络配置。
/// 各筛选项均按写入口径先规范化再匹配；查询不锁配置也不锁地址池，更不会去探测链上网络的可用性。
async fn list_deposit_network_configs(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Query(query): Query<AdminDepositNetworkConfigQuery>,
) -> AppResult<Json<AdminDepositNetworkConfigResponseList>> {
    Ok(Json(
        list_deposit_network_configs_use_case(state.mysql.clone(), query).await?,
    ))
}

/// 处理 POST /deposit-network-configs，新增一条充值网络及其可接收资产白名单。
/// 请求必须携带审计原因；资产符号的存在性在开启事务之前先行确认，随后配置写入与审计同事务提交。
/// 该配置决定后续地址入池时允许挂哪些资产以及缺省地址组，因此它必须先于对应网络的地址创建。
async fn create_deposit_network_config(
    AdminAuth(claims): AdminAuth,
    State(state): State<AppState>,
    Json(request): Json<CreateDepositNetworkConfigRequest>,
) -> AppResult<Json<AdminDepositNetworkConfigResponse>> {
    let admin_id = admin_id_from_subject(&claims.sub)?;
    Ok(Json(
        create_deposit_network_config_use_case(state.mysql.clone(), admin_id, request).await?,
    ))
}

/// 处理 PATCH /deposit-network-configs/:id，更新充值网络的地址组、资产白名单、状态与排序。
/// 请求必须携带审计原因，且是整体覆盖：未提交的字段不会保留旧值，需要连同不变部分一起提交。
/// 收窄资产白名单不会回头校验或下线已按旧白名单入池的地址，需要另行核对地址池。
async fn update_deposit_network_config(
    AdminAuth(claims): AdminAuth,
    State(state): State<AppState>,
    Path(config_id): Path<u64>,
    Json(request): Json<UpdateDepositNetworkConfigRequest>,
) -> AppResult<Json<AdminDepositNetworkConfigResponse>> {
    let admin_id = admin_id_from_subject(&claims.sub)?;
    Ok(Json(
        update_deposit_network_config_use_case(state.mysql.clone(), admin_id, config_id, request)
            .await?,
    ))
}
