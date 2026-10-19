//! 发币申请持久化；仅保存不可变输入和一次审核结果，写入均由应用层提交事务。
use crate::{
    error::{AppError, AppResult},
    modules::agent::{presentation::coin_applications::*, repository::AgentAccessScope},
};
use sqlx::{MySql, MySqlPool, QueryBuilder, Transaction};
use std::collections::BTreeMap;

/// 根据服务端预读路径逐个锁祖先，再锁门户账号；校验状态、归属和会话版本，避免审批跨过停用/改派。
/// 最多三个主键共享锁，不做反向LIKE锁表；门户排他锁也串行化同账号的幂等提交。
pub(crate) async fn lock_actor(
    tx: &mut Transaction<'_, MySql>,
    actor: u64,
    scope: &AgentAccessScope,
    version: Option<u64>,
) -> AppResult<()> {
    let mut paths = BTreeMap::new();
    let mut prefix = String::new();
    let mut last = None;
    for (depth, part) in scope
        .path
        .strip_prefix('/')
        .ok_or(AppError::Unauthorized)?
        .split('/')
        .enumerate()
    {
        let id = part
            .strip_prefix("agent:")
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|id| *id > 0 && part == format!("agent:{id}"))
            .ok_or(AppError::Unauthorized)?;
        prefix.push('/');
        prefix.push_str(part);
        if depth >= 3 || paths.insert(id, prefix.clone()).is_some() {
            return Err(AppError::Unauthorized);
        }
        last = Some(id);
    }
    if last != Some(scope.agent_id) {
        return Err(AppError::Unauthorized);
    }
    for (id, path) in paths {
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT path,status FROM agents WHERE id=? FOR SHARE")
                .bind(id)
                .fetch_optional(&mut **tx)
                .await?;
        if row != Some((path, "active".into())) {
            return Err(AppError::Conflict("代理层级或状态已变化".into()));
        }
    }
    let account: Option<(u64, String, u64)> = sqlx::query_as(
        "SELECT agent_id,status,auth_session_version FROM agent_admin_users WHERE id=? FOR UPDATE",
    )
    .bind(actor)
    .fetch_optional(&mut **tx)
    .await?;
    if !account.is_some_and(|(id, status, current)| {
        id == scope.agent_id && status == "active" && version.is_none_or(|v| v == current)
    }) {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

/// 账号锁之后读取原始幂等键；非锁定快照在锁后首次建立，避免缺失键范围锁导致并发死锁。
pub(crate) async fn by_key(
    tx: &mut Transaction<'_, MySql>,
    actor: u64,
    key: &str,
) -> AppResult<Option<CoinApplication>> {
    Ok(sqlx::query_as(
        "SELECT * FROM agent_new_coin_applications WHERE agent_admin_id=? AND idempotency_key=?",
    )
    .bind(actor)
    .bind(key.as_bytes())
    .fetch_optional(&mut **tx)
    .await?)
}

/// 同事务锁定结算资产并确认启用和精度；申请只保存资产快照，审批时还需再次验证。
pub(crate) async fn quote_asset(
    tx: &mut Transaction<'_, MySql>,
    id: u64,
) -> AppResult<(String, i32)> {
    let row: Option<(String, i32, String)> =
        sqlx::query_as("SELECT symbol,precision_scale,status FROM assets WHERE id=? FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;
    match row {
        Some((symbol, precision, status))
            if status == "active" && (0..=18).contains(&precision) =>
        {
            Ok((symbol, precision))
        }
        _ => Err(AppError::Validation("计价资产不可用".into())),
    }
}

/// 插入待审核申请，不创建资产或项目；账号/幂等键唯一索引作为最终重复保护。
pub(crate) async fn insert(
    tx: &mut Transaction<'_, MySql>,
    actor: u64,
    agent: u64,
    quote_symbol: &str,
    req: &CoinApplicationRequest,
) -> AppResult<u64> {
    let result = sqlx::query("INSERT INTO agent_new_coin_applications (agent_id,agent_admin_id,symbol,name,precision_scale,total_supply,issue_price,quote_asset_id,quote_symbol,listed_at,reason,idempotency_key,issuance_policy) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(agent).bind(actor).bind(&req.symbol).bind(&req.name).bind(req.precision_scale)
        .bind(&req.total_supply).bind(&req.issue_price).bind(req.quote_asset_id).bind(quote_symbol)
        .bind(req.listed_at).bind(&req.reason).bind(req.idempotency_key.as_bytes())
        .bind(req.issuance_policy.as_ref().map(sqlx::types::Json)).execute(&mut **tx).await?;
    Ok(result.last_insert_id())
}

/// 审核锁定唯一申请行，保证并发批准/驳回只有一条终态；缺失记录统一返回404。
pub(crate) async fn lock(tx: &mut Transaction<'_, MySql>, id: u64) -> AppResult<CoinApplication> {
    sqlx::query_as("SELECT * FROM agent_new_coin_applications WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)
}

/// 明细与计数使用相同的账号及原代理归属谓词；外部ID不能替代令牌主体，不返回其他门户历史。
pub(crate) async fn detail(
    pool: &MySqlPool,
    id: u64,
    owner: Option<(u64, u64)>,
) -> AppResult<CoinApplication> {
    let mut q = QueryBuilder::new("SELECT * FROM agent_new_coin_applications WHERE id=");
    q.push_bind(id);
    if let Some((actor, agent)) = owner {
        q.push(" AND agent_admin_id=")
            .push_bind(actor)
            .push(" AND agent_id=")
            .push_bind(agent);
    }
    q.build_query_as()
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

fn filters<'a>(
    q: &mut QueryBuilder<'a, MySql>,
    owner: Option<(u64, u64)>,
    status: Option<&'a str>,
) {
    q.push(" WHERE 1=1");
    if let Some((actor, agent)) = owner {
        q.push(" AND agent_admin_id=")
            .push_bind(actor)
            .push(" AND agent_id=")
            .push_bind(agent);
    }
    if let Some(status) = status {
        q.push(" AND status=").push_bind(status);
    }
}

/// 只读快照内按相同条件查询行和总数；倒序ID稳定分页，读取不会创建资产或改变申请。
pub(crate) async fn list(
    pool: &MySqlPool,
    owner: Option<(u64, u64)>,
    query: &CoinApplicationQuery,
) -> AppResult<CoinApplicationsResponse> {
    let mut tx = pool.begin().await?;
    let mut rows = QueryBuilder::new("SELECT * FROM agent_new_coin_applications");
    filters(&mut rows, owner, query.status.as_deref());
    rows.push(" ORDER BY id DESC LIMIT ")
        .push_bind(query.limit.unwrap_or(20))
        .push(" OFFSET ")
        .push_bind(query.offset.unwrap_or(0));
    let applications = rows.build_query_as().fetch_all(&mut *tx).await?;
    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM agent_new_coin_applications");
    filters(&mut count, owner, query.status.as_deref());
    let total = count.build_query_scalar().fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(CoinApplicationsResponse {
        applications,
        total,
    })
}

/// 在已持有申请行锁的事务中写审核人、原因和正式项目引用；项目/审计失败时此终态一起回滚。
pub(crate) async fn finish(
    tx: &mut Transaction<'_, MySql>,
    id: u64,
    admin: u64,
    project: Option<u64>,
    reason: &str,
) -> AppResult<()> {
    sqlx::query("UPDATE agent_new_coin_applications SET status=?,project_id=?,reviewed_by=?,review_reason=?,reviewed_at=CURRENT_TIMESTAMP(3) WHERE id=? AND status='pending'")
        .bind(if project.is_some(){"approved"}else{"rejected"}).bind(project).bind(admin).bind(reason).bind(id).execute(&mut **tx).await?;
    Ok(())
}
