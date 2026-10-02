//! 默认审核及逐代理免审人工充值；禁止从浏览器决定是否审核，所有资金变更同事务完成。

use super::{agent_context, agent_mysql_pool};
use crate::{
    error::{AppError, AppResult},
    modules::{
        admin::{
            infrastructure::{
                AdminAuditLogEntry, insert_admin_audit_log_entry_in_tx,
                lock_admin_recharge_asset_in_tx,
            },
            service::normalize_admin_recharge_idempotency_key,
        },
        agent::{
            infrastructure::recharges as store, presentation::recharges::*,
            service::agent_admin_id_from_subject,
        },
        wallet::amount_fits_asset_precision,
    },
};
use bigdecimal::BigDecimal;
use serde_json::json;
use sqlx::{MySql, Pool, Transaction};

fn reason(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 500 {
        return Err(AppError::Validation("操作原因不能为空且最多500字".into()));
    }
    Ok(value.to_owned())
}

fn safe_id(id: u64) -> AppResult<()> {
    if id == 0 || id > 9_007_199_254_740_991 {
        return Err(AppError::Validation("编号超过允许范围".into()));
    }
    Ok(())
}

fn check_query(query: &AdminAgentRechargeQuery) -> AppResult<()> {
    if let Some(status) = &query.status
        && !matches!(status.as_str(), "pending" | "credited" | "rejected")
    {
        return Err(AppError::Validation("不支持的充值状态".into()));
    }
    for id in [query.user_id, query.agent_id].into_iter().flatten() {
        safe_id(id)?;
    }
    Ok(())
}

async fn validate_asset(
    tx: &mut Transaction<'_, MySql>,
    asset_id: u64,
    amount: &BigDecimal,
) -> AppResult<String> {
    crate::numeric::ensure_amount_storage(amount, "充值金额")?;
    let asset = lock_admin_recharge_asset_in_tx(tx, asset_id).await?;
    if asset.status != "active" {
        return Err(AppError::Validation("充值资产已停用".into()));
    }
    if !(0..=18).contains(&asset.precision_scale) {
        return Err(AppError::Internal("充值资产精度配置无效".into()));
    }
    if !amount_fits_asset_precision(amount, asset.precision_scale) {
        return Err(AppError::Validation(format!(
            "充值金额超过资产精度{}",
            asset.precision_scale
        )));
    }
    Ok(asset.symbol)
}

fn replay_request(
    record: AgentRechargeRecord,
    user: u64,
    request: &AgentRechargeRequest,
) -> AppResult<AgentRechargeRecord> {
    if record.user_id != user
        || record.asset_id != request.asset_id
        || record.amount != request.amount
        || record.reason != request.reason
        || record.idempotency_key != request.idempotency_key.as_bytes()
    {
        return Err(AppError::Conflict("充值幂等键已用于不同请求".into()));
    }
    Ok(record)
}

fn replay_review(
    record: AgentRechargeRecord,
    admin: u64,
    request: &ReviewAgentRechargeRequest,
) -> AppResult<AgentRechargeRecord> {
    let target = if request.decision == "approve" {
        "credited"
    } else {
        "rejected"
    };
    if record.mode != "review"
        || record.status != target
        || record.reviewed_by != Some(admin)
        || record.review_reason.as_deref() != Some(request.reason.as_str())
    {
        return Err(AppError::Conflict("该充值申请已由其他审核操作处理".into()));
    }
    Ok(record)
}

/// 读取当前代理的免审快照与资产精度目录；只作表单提示，提交时必须按事务内授权重新决定路径。
pub(crate) async fn get_recharge_options(
    mysql: Option<Pool<MySql>>,
    subject: &str,
) -> AppResult<AgentRechargeOptions> {
    let (pool, scope) = agent_context(mysql, subject).await?;
    let policy = store::recharge_policy(&pool, scope.agent_id).await?;
    Ok(AgentRechargeOptions {
        direct_recharge_enabled: policy.direct_recharge_enabled,
        assets: store::recharge_assets(&pool).await?,
    })
}

/// 代理申请人工加币；用户→祖先→门户→归属→申请→资产→钱包锁保护充值范围和授权即时性。
/// 默认仅写待审核申请；免审时写申请、余额、流水、平衡分录及审计一次提交，同键重放永不再加币。
pub(crate) async fn submit_recharge(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    session_version: u64,
    user: u64,
    mut request: AgentRechargeRequest,
) -> AppResult<AgentRechargeRecord> {
    safe_id(user)?;
    safe_id(request.asset_id)?;
    if request.amount <= 0 {
        return Err(AppError::Validation("充值金额必须大于0".into()));
    }
    crate::numeric::ensure_amount_storage(&request.amount, "充值金额")?;
    request.reason = reason(&request.reason)?;
    request.idempotency_key = normalize_admin_recharge_idempotency_key(&request.idempotency_key)?;
    let actor = agent_admin_id_from_subject(subject)?;
    let (pool, _) = agent_context(mysql, subject).await?;
    let hint = store::recharge_scope_hint(&pool, actor, user).await?;
    let mut tx = pool.begin().await?;
    let user_status = store::lock_recharge_user(&mut tx, user).await?;
    let direct =
        store::lock_recharge_scope(&mut tx, actor, user, Some(session_version), &hint).await?;
    if let Some(record) = store::recharge_by_key(&mut tx, actor, &request.idempotency_key).await? {
        let record = replay_request(record, user, &request)?;
        tx.commit().await?;
        return Ok(record);
    }
    if user_status != "active" {
        return Err(AppError::Conflict("用户已停用，不能充值".into()));
    }
    let symbol = validate_asset(&mut tx, request.asset_id, &request.amount).await?;
    let id = store::insert_recharge(
        &mut tx,
        hint.agent_id,
        actor,
        user,
        &symbol,
        &request,
        direct,
    )
    .await?;
    safe_id(id)?;
    let record = store::lock_recharge(&mut tx, id).await?;
    if direct {
        store::credit_recharge(&mut tx, &record).await?;
    }
    store::audit_recharge(
        &mut tx,
        id,
        "agent",
        actor,
        if direct { "direct_credit" } else { "submit" },
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(record)
}

/// 管理员按申请不可变用户和金额审核；批准重验当前子树与原代理状态，拒绝可清理已改派/停用申请。
/// 同人同结论同原因的终态重放只返回凭证；其余终态冲突，审核失败不得消费申请或产生资金。
pub(crate) async fn review_recharge(
    mysql: Option<Pool<MySql>>,
    admin: u64,
    id: u64,
    mut request: ReviewAgentRechargeRequest,
) -> AppResult<AgentRechargeRecord> {
    safe_id(id)?;
    if !matches!(request.decision.as_str(), "approve" | "reject") {
        return Err(AppError::Validation("审核结论必须是approve或reject".into()));
    }
    request.reason = reason(&request.reason)?;
    let pool = agent_mysql_pool(mysql)?;
    let hint_record = store::recharge_by_id(&pool, id).await?;
    if hint_record.status != "pending" {
        return replay_review(hint_record, admin, &request);
    }
    let approve = request.decision == "approve";
    let hint = if approve {
        Some(
            store::recharge_scope_hint(&pool, hint_record.agent_admin_id, hint_record.user_id)
                .await?,
        )
    } else {
        None
    };
    let mut tx = pool.begin().await?;
    let status = store::lock_recharge_user(&mut tx, hint_record.user_id).await?;
    if let Some(hint) = &hint {
        if hint.agent_id != hint_record.agent_id {
            return Err(AppError::Conflict(
                "申请账号所属代理已变化，不能批准充值".into(),
            ));
        }
        if status != "active" {
            return Err(AppError::Conflict("用户已停用，不能批准充值".into()));
        }
        store::lock_recharge_scope(
            &mut tx,
            hint_record.agent_admin_id,
            hint_record.user_id,
            None,
            hint,
        )
        .await?;
    }
    let before = store::lock_recharge(&mut tx, id).await?;
    if before.status != "pending" {
        let response = replay_review(before, admin, &request)?;
        tx.commit().await?;
        return Ok(response);
    }
    if approve {
        validate_asset(&mut tx, before.asset_id, &before.amount).await?;
        store::credit_recharge(&mut tx, &before).await?;
    }
    store::review_recharge(&mut tx, id, admin, approve, &request.reason).await?;
    let after = store::lock_recharge(&mut tx, id).await?;
    store::audit_recharge(
        &mut tx,
        id,
        "admin",
        admin,
        &request.decision,
        &request.reason,
    )
    .await?;
    insert_admin_audit_log_entry_in_tx(
        &mut tx,
        admin,
        AdminAuditLogEntry {
            action: "agent_recharge.review",
            target_type: "agent_recharge",
            target_id: id,
            before_json: Some(json!(before)),
            after_json: Some(json!(after)),
            reason: Some(request.reason),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}

/// 代理申请历史只限当前门户本人和当前子树，分页总数与记录同筛选，不读取下级代理的发起历史。
pub(crate) async fn list_own_recharges(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    query: AgentRechargeQuery,
) -> AppResult<AgentRechargesResponse> {
    let query = AdminAgentRechargeQuery {
        status: query.status,
        user_id: query.user_id,
        agent_id: None,
        limit: query.limit,
        offset: query.offset,
    };
    check_query(&query)?;
    let actor = agent_admin_id_from_subject(subject)?;
    let (pool, scope) = agent_context(mysql, subject).await?;
    store::list_recharges(&pool, &query, Some((actor, &scope.path))).await
}

/// 管理员队列提供筛选一致的全平台申请历史；权限由AdminAuth精确映射，只读不代替审核。
pub(crate) async fn list_admin_recharges(
    mysql: Option<Pool<MySql>>,
    query: AdminAgentRechargeQuery,
) -> AppResult<AgentRechargesResponse> {
    check_query(&query)?;
    store::list_recharges(&agent_mysql_pool(mysql)?, &query, None).await
}

/// 按不可变申请ID读取权威结果，供管理员确认未知响应或他人审核的终态；只读不再次审批或入账。
pub(crate) async fn get_admin_recharge(
    mysql: Option<Pool<MySql>>,
    id: u64,
) -> AppResult<AgentRechargeRecord> {
    safe_id(id)?;
    store::recharge_by_id(&agent_mysql_pool(mysql)?, id).await
}

/// 按代理读取授权标志和版本，不从父级继承，也不推断待审核申请应自动执行。
pub(crate) async fn get_recharge_policy(
    mysql: Option<Pool<MySql>>,
    agent: u64,
) -> AppResult<AgentRechargePolicy> {
    safe_id(agent)?;
    store::recharge_policy(&agent_mysql_pool(mysql)?, agent).await
}

/// 管理员显式授予或撤销免审，版本检查和原因审计与标记同事务；不触碰原申请及资金。
/// 授权权限独立于agents.write及审核权限；排他代理锁与新充值的共享代理锁协调。
pub(crate) async fn update_recharge_policy(
    mysql: Option<Pool<MySql>>,
    admin: u64,
    agent: u64,
    request: UpdateAgentRechargePolicy,
) -> AppResult<AgentRechargePolicy> {
    safe_id(agent)?;
    let reason = reason(&request.reason)?;
    let pool = agent_mysql_pool(mysql)?;
    let mut tx = pool.begin().await?;
    let before = store::lock_recharge_policy(&mut tx, agent).await?;
    if before.version != request.expected_version {
        return Err(AppError::Conflict("充值授权版本已变化，请重新加载".into()));
    }
    let version = before
        .version
        .checked_add(1)
        .filter(|v| *v <= 9_007_199_254_740_991)
        .ok_or_else(|| AppError::Conflict("充值授权版本超出范围".into()))?;
    let after = AgentRechargePolicy {
        agent_id: agent,
        direct_recharge_enabled: request.direct_recharge_enabled,
        version,
    };
    store::store_recharge_policy(&mut tx, &after).await?;
    insert_admin_audit_log_entry_in_tx(
        &mut tx,
        admin,
        AdminAuditLogEntry {
            action: "agent_recharge.authorize",
            target_type: "agent",
            target_id: agent,
            before_json: Some(json!(before)),
            after_json: Some(json!(after)),
            reason: Some(reason),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(after)
}
