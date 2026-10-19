//! 管理员直接发行与新版账务查询；复用审核发布合同，所有新建副作用必须与幂等凭证一起提交。
use super::{admin_mysql_pool, create_admin_new_coin_project_in_tx};
use crate::{
    error::{AppError, AppResult},
    modules::admin::{
        infrastructure::{self as store, token_issuance as records},
        presentation::{CreateNewCoinProjectRequest, NewCoinProjectResponse, token_issuance::*},
        service::{
            new_coin_project_audit_json, normalize_admin_recharge_idempotency_key,
            required_admin_audit_reason,
        },
    },
};
use serde_json::{Value, json};
use sqlx::{MySql, Pool};

/// 身份锁先于顺序资产锁；重试返回原项目，不重建钱包/交易对；任意审计失败原子回滚。
pub(crate) async fn create(
    mysql: Option<Pool<MySql>>,
    admin: u64,
    mut req: CreateTokenIssuanceRequest,
) -> AppResult<NewCoinProjectResponse> {
    req.reason = required_admin_audit_reason(Some(req.reason))?;
    req.logo_url =
        crate::modules::admin::service::validate_optional_image_url(req.logo_url, "logo_url")?;
    req.idempotency_key = normalize_admin_recharge_idempotency_key(&req.idempotency_key)?;
    if req.asset_id == 0 || req.quote_asset_id == 0 || req.asset_id == req.quote_asset_id {
        return Err(AppError::Validation(
            "发行资产与计价资产必须有效且不同".into(),
        ));
    }
    req.total_supply = req.total_supply.normalized();
    req.issue_price = req.issue_price.normalized();
    req.issuance_policy.purchase_initial_price =
        req.issuance_policy.purchase_initial_price.normalized();
    req.issuance_policy.listing_reference_price =
        req.issuance_policy.listing_reference_price.normalized();
    use crate::modules::new_coin::domain::issuance::ExitFee;
    match &mut req.issuance_policy.fee {
        ExitFee::Profit { rate } | ExitFee::Gross { rate } => *rate = rate.normalized(),
        ExitFee::Fixed { amount } => *amount = amount.normalized(),
        ExitFee::None => {}
    }
    let intent = json!(req);
    let pool = admin_mysql_pool(mysql)?;
    let mut tx = pool.begin().await?;
    if let Some(id) =
        records::creation_replay(&mut tx, admin, &req.idempotency_key, &intent).await?
    {
        let row = store::load_admin_new_coin_project_in_tx(&mut tx, id).await?;
        tx.commit().await?;
        return Ok(row);
    }
    let mut ids = [req.asset_id, req.quote_asset_id];
    ids.sort_unstable();
    for id in ids {
        let asset = store::lock_admin_asset_in_tx(&mut tx, id).await?;
        if asset.status != "active" {
            return Err(AppError::Validation("发行与计价资产必须启用".into()));
        }
    }
    let base = store::load_admin_asset_in_tx(&mut tx, req.asset_id).await?;
    if let Some(logo) = &req.logo_url {
        records::set_asset_logo(&mut tx, req.asset_id, logo).await?;
        store::insert_admin_audit_log_entry_in_tx(
            &mut tx,
            admin,
            store::AdminAuditLogEntry {
                action: "asset.logo.update",
                target_type: "asset",
                target_id: req.asset_id,
                before_json: Some(json!({"logo_url":base.logo_url})),
                after_json: Some(json!({"logo_url":logo})),
                reason: Some(req.reason.clone()),
            },
        )
        .await?;
    }
    let quote = store::load_admin_asset_in_tx(&mut tx, req.quote_asset_id).await?;
    req.issuance_policy.validate(quote.precision_scale)?;
    let project = create_admin_new_coin_project_in_tx(
        &mut tx,
        admin,
        CreateNewCoinProjectRequest {
            asset_id: req.asset_id,
            symbol: base.symbol,
            lifecycle_status: "preheat".into(),
            total_supply: req.total_supply,
            issue_price: req.issue_price,
            quote_asset_id: req.quote_asset_id,
            listed_at: Some(req.issuance_policy.listing.at),
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
    .await
    .map_err(creation_error)?;
    crate::modules::new_coin::infrastructure::issuance::publish_policy(
        &mut tx,
        project.id,
        None,
        admin,
        &req.issuance_policy,
    )
    .await
    .map_err(creation_error)?;
    crate::modules::spot::infrastructure::ensure_spot_liquidity_user_in_tx(&mut tx).await?;
    store::create_wallet_accounts_for_asset_in_tx(&mut tx, req.asset_id).await?;
    let row = store::load_admin_new_coin_project_in_tx(&mut tx, project.id).await?;
    let snapshot = new_coin_project_audit_json(&row);
    crate::modules::new_coin::infrastructure::issuance::operation(
        &mut tx,
        row.id,
        ("admin", Some(admin)),
        "new_coin.issuance.create",
        snapshot.clone(),
        &req.reason,
    )
    .await?;
    store::insert_admin_audit_log_entry_in_tx(
        &mut tx,
        admin,
        store::AdminAuditLogEntry {
            action: "new_coin.issuance.create",
            target_type: "new_coin_project",
            target_id: row.id,
            before_json: None,
            after_json: Some(snapshot),
            reason: Some(req.reason),
        },
    )
    .await?;
    records::creation_receipt(&mut tx, admin, &req.idempotency_key, &intent, row.id).await?;
    tx.commit().await?;
    Ok(row)
}

fn creation_error(error: AppError) -> AppError {
    if matches!(&error, AppError::Database(err) if err.as_database_error().is_some_and(|e| e.is_unique_violation()))
    {
        AppError::Api {
            status: axum::http::StatusCode::CONFLICT,
            code: "TOKEN_ISSUANCE_EXISTS",
            message: "发行项目或专属交易对已存在，请核对已有项目".into(),
        }
    } else {
        error
    }
}

fn page(limit: Option<u32>, offset: Option<u32>) -> AppResult<(u32, u32)> {
    let (limit, offset) = (limit.unwrap_or(20), offset.unwrap_or(0));
    if !(1..=100).contains(&limit) || offset > 100_000 {
        return Err(AppError::Validation("分页范围无效".into()));
    }
    Ok((limit, offset))
}
fn validate_records(q: &AdminTokenRecordsQuery, states: &[&str]) -> AppResult<()> {
    if [q.project_id, q.user_id, q.agent_id, q.issuer_agent_id].contains(&Some(0))
        || q.email.as_ref().is_some_and(|v| v.len() > 254)
        || q.status
            .as_ref()
            .is_some_and(|v| !v.trim().is_empty() && !states.contains(&v.trim()))
    {
        return Err(AppError::Validation("记录筛选条件无效".into()));
    }
    Ok(())
}
/// 只读新版持仓，状态校验与分页上界在访问数据库前执行；权限由管理员入口统一检查。
pub(crate) async fn holdings(
    mysql: Option<Pool<MySql>>,
    q: AdminTokenRecordsQuery,
) -> AppResult<Value> {
    validate_records(&q, &["held", "sold"])?;
    let (limit, offset) = page(q.limit, q.offset)?;
    let (rows, total) = records::holdings(&admin_mysql_pool(mysql)?, &q, limit, offset).await?;
    Ok(json!({"holdings":rows,"total":total}))
}
/// 回购列表仅含已执行卖出，不用尚未确认的报价推算收益、手续费或到账。
pub(crate) async fn buybacks(
    mysql: Option<Pool<MySql>>,
    q: AdminTokenRecordsQuery,
) -> AppResult<Value> {
    validate_records(&q, &["executed"])?;
    let (limit, offset) = page(q.limit, q.offset)?;
    let (rows, total) = records::buybacks(&admin_mysql_pool(mysql)?, &q, limit, offset).await?;
    Ok(json!({"buybacks":rows,"total":total}))
}
/// 全角色操作只供审计追溯，精确匹配项目、身份与动作，绝不将浏览行为转换为补偿任务。
pub(crate) async fn operations(
    mysql: Option<Pool<MySql>>,
    q: AdminTokenOperationsQuery,
) -> AppResult<Value> {
    if q.project_id == Some(0)
        || q.actor_id == Some(0)
        || q.actor_type.as_ref().is_some_and(|v| {
            !v.trim().is_empty() && !["admin", "agent", "scheduler", "user"].contains(&v.trim())
        })
        || q.action.as_ref().is_some_and(|v| v.len() > 64)
    {
        return Err(AppError::Validation("操作记录筛选条件无效".into()));
    }
    let (limit, offset) = page(q.limit, q.offset)?;
    let (rows, total) = records::operations(&admin_mysql_pool(mysql)?, &q, limit, offset).await?;
    Ok(json!({"operations":rows,"total":total}))
}
