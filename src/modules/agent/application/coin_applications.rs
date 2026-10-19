//! 代理申请及管理员审核用例；审核前不创建公开资源，审核结果、资产、项目和审计原子提交。
use super::{agent_context, agent_mysql_pool};
use crate::{
    error::{AppError, AppResult},
    modules::{
        admin::{
            application::create_admin_new_coin_project_in_tx,
            infrastructure::{
                AdminAssetInsert, AdminAuditLogEntry, create_wallet_accounts_for_asset_in_tx,
                insert_admin_asset_in_tx, insert_admin_audit_log_entry_in_tx,
            },
            presentation::CreateNewCoinProjectRequest,
            service::{
                normalize_admin_recharge_idempotency_key, normalize_asset_symbol,
                validate_asset_name, validate_asset_precision,
            },
        },
        agent::{
            infrastructure::{
                coin_applications as store, load_agent_access_scope_for_admin,
                recharges::recharge_assets,
            },
            presentation::{coin_applications::*, recharges::RechargeAsset},
            service::agent_admin_id_from_subject,
        },
        new_coin::service::ensure_new_coin_amount_precision,
    },
};
use serde_json::json;
use sqlx::{MySql, Pool};

fn safe_id(id: u64) -> AppResult<()> {
    if id == 0 || id > 9_007_199_254_740_991 {
        return Err(AppError::Validation("编号超出范围".into()));
    }
    Ok(())
}
fn reason(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 500 {
        return Err(AppError::Validation("原因需为1至500字".into()));
    }
    Ok(value.into())
}
fn check_query(q: &CoinApplicationQuery) -> AppResult<()> {
    if q.status
        .as_ref()
        .is_some_and(|s| !matches!(s.as_str(), "pending" | "approved" | "rejected"))
        || !(1..=100).contains(&q.limit.unwrap_or(20))
        || q.offset.unwrap_or(0) > 100_000
    {
        return Err(AppError::Validation("申请筛选或分页无效".into()));
    }
    Ok(())
}

/// 返回启用计价资产目录；身份和所有祖先均需有效，后续提交与审批仍锁内重验。
pub(crate) async fn options(
    mysql: Option<Pool<MySql>>,
    subject: &str,
) -> AppResult<Vec<RechargeAsset>> {
    let (pool, _) = agent_context(mysql, subject).await?;
    recharge_assets(&pool).await
}

/// 申请只保存不可变发行输入；同门户同键重放比较完整内容，未知结果重试不会重复建申请。
/// 先锁代理链和账号，再建立幂等快照、锁资产；不产生公开项目、余额或认购副作用。
pub(crate) async fn submit(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    version: u64,
    mut req: CoinApplicationRequest,
) -> AppResult<CoinApplication> {
    req.symbol = normalize_asset_symbol(&req.symbol)?;
    req.name = validate_asset_name(&req.name)?;
    req.logo_url =
        crate::modules::admin::service::validate_optional_image_url(req.logo_url, "logo_url")?;
    validate_asset_precision(req.precision_scale)?;
    safe_id(req.quote_asset_id)?;
    req.reason = reason(&req.reason)?;
    req.idempotency_key = normalize_admin_recharge_idempotency_key(&req.idempotency_key)?;
    crate::time::ensure_timestamp_storage(&req.listed_at, "listed_at")?;
    for amount in [&req.total_supply, &req.issue_price] {
        crate::numeric::ensure_amount_storage(amount, "发行金额")?;
        if amount <= &bigdecimal::BigDecimal::from(0) {
            return Err(AppError::Validation("发行总量和发行价必须大于0".into()));
        }
    }
    ensure_new_coin_amount_precision(&req.total_supply, req.precision_scale, "total_supply")?;
    let actor = agent_admin_id_from_subject(subject)?;
    let (pool, scope) = agent_context(mysql, subject).await?;
    let mut tx = pool.begin().await?;
    store::lock_actor(&mut tx, actor, &scope, Some(version)).await?;
    if let Some(row) = store::by_key(&mut tx, actor, &req.idempotency_key).await? {
        if row.agent_id != scope.agent_id
            || row.symbol != req.symbol
            || row.name != req.name
            || row.logo_url != req.logo_url
            || row.precision_scale != req.precision_scale
            || row.total_supply != req.total_supply
            || row.issue_price != req.issue_price
            || row.quote_asset_id != req.quote_asset_id
            || row.listed_at != req.listed_at
            || row.reason != req.reason
            || row.idempotency_key != req.idempotency_key.as_bytes()
            || row.issuance_policy.as_ref().map(|p| &p.0) != req.issuance_policy.as_ref()
        {
            return Err(AppError::Conflict("该请求标识已用于不同申请".into()));
        }
        tx.commit().await?;
        return Ok(row);
    }
    let (symbol, precision) = store::quote_asset(&mut tx, req.quote_asset_id).await?;
    if req.symbol == symbol {
        return Err(AppError::Validation("新币不能与计价资产相同".into()));
    }
    ensure_new_coin_amount_precision(&req.issue_price, precision, "issue_price")?;
    if let Some(policy) = &req.issuance_policy {
        policy.validate(precision)?;
        crate::modules::market::domain::ValidatedMarketSymbol::from_raw(&format!(
            "{}{symbol}",
            req.symbol
        ))
        .map_err(|_| AppError::Validation("新币与计价资产组合后的交易对代码不能超过32位".into()))?;
        if req.listed_at != policy.listing.at {
            return Err(AppError::Validation("计划上市时间与发行合同不一致".into()));
        }
    }
    let id = store::insert(&mut tx, actor, scope.agent_id, &symbol, &req).await?;
    safe_id(id)?;
    let row = store::lock(&mut tx, id).await?;
    tx.commit().await?;
    Ok(row)
}

/// 代理列表只取当前门户账号本人及原代理的记录；客户端不能指定上级、下级或其他账号。
pub(crate) async fn list_own(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    q: CoinApplicationQuery,
) -> AppResult<CoinApplicationsResponse> {
    check_query(&q)?;
    let actor = agent_admin_id_from_subject(subject)?;
    let (pool, scope) = agent_context(mysql, subject).await?;
    store::list(&pool, Some((actor, scope.agent_id)), &q).await
}
/// 代理明细按ID与双重归属共同查询；无记录及其他账号记录统一返回404，不泄漏申请信息。
pub(crate) async fn own_detail(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    id: u64,
) -> AppResult<CoinApplication> {
    safe_id(id)?;
    let actor = agent_admin_id_from_subject(subject)?;
    let (pool, scope) = agent_context(mysql, subject).await?;
    store::detail(&pool, id, Some((actor, scope.agent_id))).await
}
/// 管理员只读审核队列；权限由AdminAuth独立read权限控制，状态和分页在访问数据库前校验。
pub(crate) async fn list_admin(
    mysql: Option<Pool<MySql>>,
    q: CoinApplicationQuery,
) -> AppResult<CoinApplicationsResponse> {
    check_query(&q)?;
    store::list(&agent_mysql_pool(mysql)?, None, &q).await
}
/// 管理员按唯一ID恢复未知审核结果；读取不执行审批、不新建资产。
pub(crate) async fn admin_detail(
    mysql: Option<Pool<MySql>>,
    id: u64,
) -> AppResult<CoinApplication> {
    safe_id(id)?;
    store::detail(&agent_mysql_pool(mysql)?, id, None).await
}

fn replay(
    row: CoinApplication,
    admin: u64,
    req: &CoinApplicationReview,
) -> AppResult<CoinApplication> {
    let status = if req.decision == "approve" {
        "approved"
    } else {
        "rejected"
    };
    if row.status != status
        || row.reviewed_by != Some(admin)
        || row.review_reason.as_deref() != Some(req.reason.as_str())
    {
        return Err(AppError::Conflict(
            "申请已由其他审核操作处理，请刷新记录".into(),
        ));
    }
    Ok(row)
}

/// 批准在一个事务内创建禁用充提的新资产、预热项目和审核凭证；拒绝不创建任何公开资源。
/// 代理链→门户→申请→资产锁顺序与提交一致，同审核重放只返回凭证，冲突或审计失败整体回滚。
pub(crate) async fn review(
    mysql: Option<Pool<MySql>>,
    admin: u64,
    id: u64,
    mut req: CoinApplicationReview,
) -> AppResult<CoinApplication> {
    safe_id(id)?;
    req.reason = reason(&req.reason)?;
    if !matches!(req.decision.as_str(), "approve" | "reject") {
        return Err(AppError::Validation("审核结论无效".into()));
    }
    let pool = agent_mysql_pool(mysql)?;
    let hint = store::detail(&pool, id, None).await?;
    if hint.status != "pending" {
        return replay(hint, admin, &req);
    }
    let scope = if req.decision == "approve" {
        Some(
            load_agent_access_scope_for_admin(&pool, hint.agent_admin_id)
                .await?
                .filter(|s| s.agent_id == hint.agent_id)
                .ok_or(AppError::Conflict("申请代理已停用或改派".into()))?,
        )
    } else {
        None
    };
    let mut tx = pool.begin().await?;
    if let Some(scope) = scope {
        store::lock_actor(&mut tx, hint.agent_admin_id, &scope, None).await?;
    }
    let row = store::lock(&mut tx, id).await?;
    if row.status != "pending" {
        let result = replay(row, admin, &req)?;
        tx.commit().await?;
        return Ok(result);
    }
    let project = if req.decision == "approve" {
        let (_, precision) = store::quote_asset(&mut tx, row.quote_asset_id).await?;
        ensure_new_coin_amount_precision(&row.issue_price, precision, "issue_price")?;
        if let Some(policy) = &row.issuance_policy {
            policy.validate(precision)?;
        }
        let asset = insert_admin_asset_in_tx(
            &mut tx,
            AdminAssetInsert {
                symbol: row.symbol.clone(),
                name: row.name.clone(),
                logo_url: row.logo_url.clone(),
                precision_scale: row.precision_scale,
                asset_type: "coin".into(),
                status: "active".into(),
                deposit_enabled: false,
                withdraw_enabled: false,
                margin_transfer_enabled: false,
                min_deposit_amount: 0.into(),
                deposit_fee: 0.into(),
                withdraw_fee: 0.into(),
                withdraw_fee_tiers: vec![],
            },
        )
        .await?;
        create_wallet_accounts_for_asset_in_tx(&mut tx, asset).await?;
        Some(
            create_admin_new_coin_project_in_tx(
                &mut tx,
                admin,
                CreateNewCoinProjectRequest {
                    asset_id: asset,
                    symbol: row.symbol.clone(),
                    lifecycle_status: "preheat".into(),
                    total_supply: row.total_supply.clone(),
                    issue_price: row.issue_price.clone(),
                    quote_asset_id: row.quote_asset_id,
                    listed_at: Some(row.listed_at),
                    unlock_type: "immediate_on_listing".into(),
                    fixed_unlock_at: None,
                    relative_unlock_seconds: None,
                    unlock_fee_enabled: Some(false),
                    unlock_fee_rate: None,
                    unlock_fee_basis: None,
                    unlock_fee_asset: None,
                    reason: Some(req.reason.clone()),
                },
            )
            .await?
            .id,
        )
    } else {
        None
    };
    if let (Some(project_id), Some(policy)) = (project, &row.issuance_policy) {
        crate::modules::new_coin::infrastructure::issuance::publish_policy(
            &mut tx,
            project_id,
            Some(row.agent_id),
            admin,
            policy,
        )
        .await?;
        crate::modules::spot::infrastructure::ensure_spot_liquidity_user_in_tx(&mut tx).await?;
    }
    store::finish(&mut tx, id, admin, project, &req.reason).await?;
    let after = store::lock(&mut tx, id).await?;
    insert_admin_audit_log_entry_in_tx(
        &mut tx,
        admin,
        AdminAuditLogEntry {
            action: "new_coin_application.review",
            target_type: "new_coin_application",
            target_id: id,
            before_json: Some(json!(row)),
            after_json: Some(json!(after)),
            reason: Some(req.reason),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

/// 代理图片上传复用存储与字节校验；身份及代理链有效才允许上传，归属记录为门户账号。
/// 上传对象不代表发行成功；只有申请审核事务可以将 Logo 发布到资产和交易对。
pub(crate) async fn upload_logo(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    key: Option<&str>,
    input: crate::modules::admin::presentation::UploadFileInput,
) -> AppResult<crate::modules::admin::presentation::UploadImageResponse> {
    let (pool, _) = agent_context(mysql, subject).await?;
    let actor = agent_admin_id_from_subject(subject)?;
    crate::modules::admin::application::upload_image_for_owner(
        &pool,
        crate::modules::admin::repository::UploadObjectOwner::Agent(actor),
        key,
        input,
    )
    .await
}
