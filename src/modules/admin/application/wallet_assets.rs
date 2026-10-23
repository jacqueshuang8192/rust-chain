use super::*;

/// 按规范化符号、资产类型和状态筛选资产，并返回 Logo、精度、充提规则和阶梯费的分页结果与总数。
/// 非空筛选执行写入同款校验，分页统一裁剪；查询不锁资产或聚合钱包余额。
pub(crate) async fn list_admin_assets(
    pool: Option<Pool<MySql>>,
    query: AdminAssetQuery,
) -> AppResult<AdminAssetsResponse> {
    let symbol = query
        .symbol
        .and_then(optional_string)
        .map(|value| normalize_asset_symbol(&value))
        .transpose()?;
    let asset_type = query
        .asset_type
        .and_then(optional_string)
        .map(|value| validate_asset_type(&value))
        .transpose()?;
    let status = query
        .status
        .and_then(optional_string)
        .map(|value| validate_asset_status(&value))
        .transpose()?;
    let pool = admin_mysql_pool(pool)?;
    let (assets, total) = list_admin_assets_from_store(
        &pool,
        AdminAssetListFilter {
            symbol,
            asset_type,
            status,
            limit: route_limit(query.limit),
            offset: route_offset(query.offset),
        },
    )
    .await?;
    Ok(AdminAssetsResponse { assets, total })
}

/// 按资产 ID 读取符号、精度、类型、状态、充提开关、费用和阶梯费完整配置。
/// 查询不加资产锁；记录缺失返回未找到，SQL/JSON 解码失败返回错误，不读取任何用户余额。
pub(crate) async fn get_admin_asset(
    pool: Option<Pool<MySql>>,
    asset_id: u64,
) -> AppResult<AdminAssetResponse> {
    let pool = admin_mysql_pool(pool)?;
    load_admin_asset_from_store(&pool, asset_id).await
}

/// 创建资产并为所有现有用户初始化该资产的钱包账户，返回最终资产配置。
/// 请求须提供合法符号、名称、0～18 精度和非负费用；类型/状态/充提开关缺省为 coin/active/true，权限由调用方保证。
/// 事务依次插入资产、回读、批量创建缺失钱包账户和写 after 审计；唯一键或任一步失败整体回滚。
/// 本用例无幂等键，重复请求依赖资产唯一约束失败；不会生成余额流水或外部链上资产。
pub(crate) async fn create_admin_asset(
    pool: Option<Pool<MySql>>,
    admin_id: u64,
    request: CreateAssetRequest,
) -> AppResult<AdminAssetResponse> {
    validate_create_asset_request(&request)?;
    let symbol = normalize_asset_symbol(&request.symbol)?;
    let name = validate_asset_name(&request.name)?;
    let logo_url = validate_optional_image_url(request.logo_url, "asset logo_url")?;
    let asset_type = request
        .asset_type
        .as_deref()
        .map(validate_asset_type)
        .transpose()?
        .unwrap_or_else(|| "coin".to_owned());
    let status = request
        .status
        .as_deref()
        .map(validate_asset_status)
        .transpose()?
        .unwrap_or_else(|| "active".to_owned());
    let deposit_enabled = request.deposit_enabled.unwrap_or(true);
    let withdraw_enabled = request.withdraw_enabled.unwrap_or(true);
    let margin_transfer_enabled = request.margin_transfer_enabled.unwrap_or(false);
    let min_deposit_amount = request
        .min_deposit_amount
        .unwrap_or_else(|| BigDecimal::from(0));
    let deposit_fee = request.deposit_fee.unwrap_or_else(|| BigDecimal::from(0));
    let withdraw_fee = request.withdraw_fee.unwrap_or_else(|| BigDecimal::from(0));
    let withdraw_fee_tiers =
        normalize_asset_withdraw_fee_tiers(request.withdraw_fee_tiers.unwrap_or_default())?;
    validate_asset_fee_settings(&min_deposit_amount, &deposit_fee, &withdraw_fee)?;
    let pool = admin_mysql_pool(pool)?;

    // 资产创建和钱包账户初始化必须同事务提交，避免用户缺少新资产账户。
    let mut tx = pool.begin().await?;
    let asset_id = insert_admin_asset_in_tx(
        &mut tx,
        AdminAssetInsert {
            symbol,
            name,
            logo_url,
            precision_scale: request.precision_scale,
            asset_type,
            status,
            deposit_enabled,
            withdraw_enabled,
            margin_transfer_enabled,
            min_deposit_amount,
            deposit_fee,
            withdraw_fee,
            withdraw_fee_tiers,
        },
    )
    .await?;
    let asset = load_admin_asset_in_tx(&mut tx, asset_id).await?;
    create_wallet_accounts_for_asset_in_tx(&mut tx, asset.id).await?;
    insert_admin_audit_log_entry_in_tx(
        &mut tx,
        admin_id,
        AdminAuditLogEntry {
            action: "asset.create",
            target_type: "asset",
            target_id: asset.id,
            before_json: None,
            after_json: Some(asset_audit_json(&asset)),
            reason: request.reason,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(asset)
}

/// 更新资产展示、精度、类型、状态、充提开关和费用规则，并返回最终配置。
/// 请求须提供审计原因；未给出的充提开关、金额和阶梯费沿用锁定旧值，符号不可修改。
/// 事务先锁资产，再合并可选字段、校验最终费用、更新、回读并写 before/after 审计；失败整体回滚。
/// 相同配置重放仍写审计；变更精度不会重算钱包余额，禁用也不会撤销在途充提请求。
pub(crate) async fn update_admin_asset(
    pool: Option<Pool<MySql>>,
    admin_id: u64,
    asset_id: u64,
    request: UpdateAssetRequest,
) -> AppResult<AdminAssetResponse> {
    validate_update_asset_request(&request)?;
    let name = validate_asset_name(&request.name)?;
    let asset_type = validate_asset_type(&request.asset_type)?;
    let status = validate_asset_status(&request.status)?;
    let logo_url = validate_optional_image_url(request.logo_url, "asset logo_url")?;
    let reason = required_admin_audit_reason(request.reason)?;
    let pool = admin_mysql_pool(pool)?;

    // 先锁定资产旧值再更新，确保资产配置和审计 before/after 对应同一次事务。
    let mut tx = pool.begin().await?;
    let before = lock_admin_asset_in_tx(&mut tx, asset_id).await?;
    let deposit_enabled = request.deposit_enabled.unwrap_or(before.deposit_enabled);
    let withdraw_enabled = request.withdraw_enabled.unwrap_or(before.withdraw_enabled);
    let margin_transfer_enabled = request
        .margin_transfer_enabled
        .unwrap_or(before.margin_transfer_enabled);
    let min_deposit_amount = request
        .min_deposit_amount
        .unwrap_or_else(|| before.min_deposit_amount.clone());
    let deposit_fee = request
        .deposit_fee
        .unwrap_or_else(|| before.deposit_fee.clone());
    let withdraw_fee = request
        .withdraw_fee
        .unwrap_or_else(|| before.withdraw_fee.clone());
    let withdraw_fee_tiers = match request.withdraw_fee_tiers {
        Some(tiers) => normalize_asset_withdraw_fee_tiers(tiers)?,
        None => before.withdraw_fee_tiers.0.clone(),
    };
    validate_asset_fee_settings(&min_deposit_amount, &deposit_fee, &withdraw_fee)?;
    update_admin_asset_in_tx(
        &mut tx,
        asset_id,
        AdminAssetUpdate {
            name,
            logo_url,
            precision_scale: request.precision_scale,
            asset_type,
            status,
            deposit_enabled,
            withdraw_enabled,
            margin_transfer_enabled,
            min_deposit_amount,
            deposit_fee,
            withdraw_fee,
            withdraw_fee_tiers,
        },
    )
    .await?;
    let after = load_admin_asset_in_tx(&mut tx, asset_id).await?;
    insert_admin_audit_log_entry_in_tx(
        &mut tx,
        admin_id,
        AdminAuditLogEntry {
            action: "asset.config.update",
            target_type: "asset",
            target_id: after.id,
            before_json: Some(asset_audit_json(&before)),
            after_json: Some(asset_audit_json(&after)),
            reason: Some(reason),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

/// 删除已停用且无业务引用的资产，并清理该资产所有零余额钱包账户。
/// 请求须提供审计原因；事务先锁资产并要求 status=disabled，再删除零余额账户、检查剩余引用、删除资产并写 before 审计。
/// 非零钱包或其他引用会阻止删除；检查、清理、资产删除和审计任一步失败均整体回滚，不会留下部分账户清理。
/// 删除不具幂等性，成功后重放返回未找到；本函数不删除链上数据或上传的 Logo 对象。
pub(crate) async fn delete_admin_asset(
    pool: Option<Pool<MySql>>,
    admin_id: u64,
    asset_id: u64,
    request: DeleteAssetRequest,
) -> AppResult<()> {
    let reason = required_admin_audit_reason(request.reason)?;
    let pool = admin_mysql_pool(pool)?;

    // 删除前先清理零余额钱包账户，再检查引用，避免仅由空钱包账户阻止资产退场。
    let mut tx = pool.begin().await?;
    let before = lock_admin_asset_in_tx(&mut tx, asset_id).await?;
    if before.status != "disabled" {
        return Err(AppError::Validation(
            "asset must be disabled before deletion".to_owned(),
        ));
    }
    delete_zero_balance_wallet_accounts_for_asset_in_tx(&mut tx, asset_id).await?;
    ensure_asset_has_no_references_in_tx(&mut tx, asset_id).await?;
    delete_admin_asset_in_tx(&mut tx, asset_id).await?;
    insert_admin_audit_log_entry_in_tx(
        &mut tx,
        admin_id,
        AdminAuditLogEntry {
            action: "asset.delete",
            target_type: "asset",
            target_id: asset_id,
            before_json: Some(asset_audit_json(&before)),
            after_json: None,
            reason: Some(reason),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// 按用户、邮箱和资产筛选钱包账户，并可选择包含空余额账户和内部用户，返回分页余额与总数。
/// 两个 include 开关缺省 false，查询不锁钱包或计算跨页合计；读取期间余额可被并发交易更新。
pub(crate) async fn list_admin_wallet_accounts(
    pool: Option<Pool<MySql>>,
    query: AdminWalletAccountQuery,
) -> AppResult<AdminWalletAccountsResponse> {
    let pool = admin_mysql_pool(pool)?;
    let (accounts, total) = list_admin_wallet_accounts_from_store(
        &pool,
        AdminWalletAccountListFilter {
            user_id: query.user_id,
            email: query.email,
            asset_id: query.asset_id,
            include_empty: query.include_empty.unwrap_or(false),
            include_internal: query.include_internal.unwrap_or(false),
            limit: route_limit(query.limit),
            offset: route_offset(query.offset),
        },
    )
    .await?;
    Ok(AdminWalletAccountsResponse { accounts, total })
}

/// 按用户、邮箱、资产、变更类型和引用类型筛选钱包流水，并可包含内部用户记录。
/// 返回当前页和同组谓词总数；查询不锁钱包、补写流水或重算余额，分页统一裁剪。
pub(crate) async fn list_admin_wallet_ledger(
    pool: Option<Pool<MySql>>,
    query: AdminWalletLedgerQuery,
) -> AppResult<AdminWalletLedgerResponseList> {
    let account_type = query.account_type.unwrap_or_else(|| "spot".to_owned());
    if !matches!(account_type.as_str(), "spot" | "margin") {
        return Err(AppError::Validation(
            "account_type must be spot or margin".to_owned(),
        ));
    }
    if query
        .balance_type
        .as_deref()
        .is_some_and(|value| !matches!(value, "available" | "frozen" | "locked"))
    {
        return Err(AppError::Validation(
            "unsupported wallet balance_type".to_owned(),
        ));
    }
    let pool = admin_mysql_pool(pool)?;
    let (ledger, total) = list_admin_wallet_ledger_from_store(
        &pool,
        AdminWalletLedgerListFilter {
            account_type,
            balance_type: query.balance_type,
            ref_id: query.ref_id,
            user_id: query.user_id,
            email: query.email,
            asset_id: query.asset_id,
            change_type: query.change_type,
            ref_type: query.ref_type,
            include_internal: query.include_internal.unwrap_or(false),
            limit: route_limit(query.limit),
            offset: route_offset(query.offset),
        },
    )
    .await?;
    Ok(AdminWalletLedgerResponseList { ledger, total })
}

/// 按规范化网络、地址组、状态和资产符号筛选提现网络配置，并返回分页结果与总数。
/// 非空筛选执行写入同款格式校验；查询不锁配置或地址池，也不探测链上网络状态。
pub(crate) async fn list_admin_deposit_network_configs(
    pool: Option<Pool<MySql>>,
    query: AdminDepositNetworkConfigQuery,
) -> AppResult<AdminDepositNetworkConfigResponseList> {
    let network = query
        .network
        .and_then(optional_string)
        .map(|value| normalize_deposit_network(&value))
        .transpose()?;
    let address_group_code = query
        .address_group_code
        .and_then(optional_string)
        .map(|value| validate_address_group_code(&value))
        .transpose()?;
    let status = query
        .status
        .and_then(optional_string)
        .map(|value| validate_deposit_network_config_status(&value))
        .transpose()?;
    let asset_symbol = query
        .asset_symbol
        .and_then(optional_string)
        .map(|value| normalize_asset_symbol(&value))
        .transpose()?;
    let pool = admin_mysql_pool(pool)?;
    let (configs, total) = list_admin_deposit_network_configs_from_store(
        &pool,
        AdminDepositNetworkConfigListFilter {
            network,
            address_group_code,
            status,
            asset_symbol,
            limit: route_limit(query.limit),
            offset: route_offset(query.offset),
        },
    )
    .await?;
    Ok(AdminDepositNetworkConfigResponseList { configs, total })
}

/// 创建充值网络及其可接收资产范围，供后续地址池准入与分配使用。
/// 调用方须已完成管理员鉴权并提供审计原因；网络、地址组、状态及资产符号会先规范化校验。
/// 事务内完成配置插入、回读和后台审计，资产存在性在事务前确认，任一步失败均不提交配置。
/// 本用例没有请求幂等键；重复创建由数据库唯一约束报错，不会复用已有配置。
pub(crate) async fn create_admin_deposit_network_config(
    pool: Option<Pool<MySql>>,
    admin_id: u64,
    request: CreateDepositNetworkConfigRequest,
) -> AppResult<AdminDepositNetworkConfigResponse> {
    let reason = required_admin_audit_reason(request.reason)?;
    let network = normalize_deposit_network(&request.network)?;
    let display_name = validate_deposit_network_display_name(&request.display_name)?;
    let address_group_code = validate_address_group_code(&request.address_group_code)?;
    let address_group_name =
        validate_optional_length(request.address_group_name, "address_group_name", 128)?;
    let asset_symbols = normalize_deposit_asset_symbols(None, request.asset_symbols)?;
    let status = request
        .status
        .as_deref()
        .map(validate_deposit_network_config_status)
        .transpose()?
        .unwrap_or_else(|| "active".to_owned());
    let sort_order = request.sort_order.unwrap_or(0);
    let pool = admin_mysql_pool(pool)?;
    ensure_asset_symbols_exist(&pool, &asset_symbols).await?;

    // 网络配置写入与审计同事务提交，避免提现准入读取到无审计的配置变更。
    let mut tx = pool.begin().await?;
    let config_id = insert_admin_deposit_network_config_in_tx(
        &mut tx,
        AdminDepositNetworkConfigWrite {
            network,
            display_name,
            address_group_code,
            address_group_name,
            asset_symbols,
            status,
            sort_order,
        },
    )
    .await?;
    let created = load_deposit_network_config_in_tx(&mut tx, config_id).await?;
    insert_admin_audit_log_entry_in_tx(
        &mut tx,
        admin_id,
        AdminAuditLogEntry {
            action: "deposit_network_config.create",
            target_type: "deposit_network_config",
            target_id: created.id,
            before_json: None,
            after_json: Some(deposit_network_config_audit_json(&created)),
            reason: Some(reason),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(created)
}

/// 更新充值网络的地址组、资产白名单、状态和排序，并返回最终快照。
/// 调用方须已完成管理员鉴权并提供审计原因；所有输入和资产符号须先通过准入校验。
/// 事务按“锁定旧配置、更新、回读、写审计”执行，确保地址池读取的配置与审计一致。
/// 每次成功调用都会新增审计；失败整体回滚，不提供跨请求幂等复用。
pub(crate) async fn update_admin_deposit_network_config(
    pool: Option<Pool<MySql>>,
    admin_id: u64,
    config_id: u64,
    request: UpdateDepositNetworkConfigRequest,
) -> AppResult<AdminDepositNetworkConfigResponse> {
    let reason = required_admin_audit_reason(request.reason)?;
    let network = normalize_deposit_network(&request.network)?;
    let display_name = validate_deposit_network_display_name(&request.display_name)?;
    let address_group_code = validate_address_group_code(&request.address_group_code)?;
    let address_group_name =
        validate_optional_length(request.address_group_name, "address_group_name", 128)?;
    let asset_symbols = normalize_deposit_asset_symbols(None, request.asset_symbols)?;
    let status = validate_deposit_network_config_status(&request.status)?;
    let pool = admin_mysql_pool(pool)?;
    ensure_asset_symbols_exist(&pool, &asset_symbols).await?;

    // 先锁定旧网络配置再更新，确保提现网络配置审计 before/after 对应同一次事务。
    let mut tx = pool.begin().await?;
    let before = lock_deposit_network_config_in_tx(&mut tx, config_id).await?;
    update_admin_deposit_network_config_in_tx(
        &mut tx,
        config_id,
        AdminDepositNetworkConfigWrite {
            network,
            display_name,
            address_group_code,
            address_group_name,
            asset_symbols,
            status,
            sort_order: request.sort_order,
        },
    )
    .await?;
    let after = load_deposit_network_config_in_tx(&mut tx, config_id).await?;
    insert_admin_audit_log_entry_in_tx(
        &mut tx,
        admin_id,
        AdminAuditLogEntry {
            action: "deposit_network_config.update",
            target_type: "deposit_network_config",
            target_id: after.id,
            before_json: Some(deposit_network_config_audit_json(&before)),
            after_json: Some(deposit_network_config_audit_json(&after)),
            reason: Some(reason),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}
