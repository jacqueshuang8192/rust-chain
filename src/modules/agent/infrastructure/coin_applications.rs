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

/// 批准事务发布完整不可变规则和下一自动节点；此处只排程，既不派发也不出售持仓。
pub(crate) async fn publish_policy(
    tx: &mut Transaction<'_, MySql>,
    project_id: u64,
    agent_id: u64,
    admin_id: u64,
    policy: &crate::modules::new_coin::domain::issuance::IssuancePolicy,
) -> AppResult<()> {
    let (base, quote, symbol, quote_symbol, quantity_precision, price_precision): (u64,u64,String,String,i32,i32) = sqlx::query_as("SELECT p.asset_id,p.quote_asset_id,p.symbol,q.symbol,a.precision_scale,q.precision_scale FROM new_coin_projects p JOIN assets a ON a.id=p.asset_id JOIN assets q ON q.id=p.quote_asset_id WHERE p.id=?")
        .bind(project_id).fetch_one(&mut **tx).await?;
    let pair_symbol = crate::modules::market::domain::ValidatedMarketSymbol::from_raw(&format!(
        "{symbol}{quote_symbol}"
    ))
    .map_err(|_| AppError::Validation("新币与计价资产组合后的交易对代码不能超过32位".into()))?;
    let pair_id = sqlx::query("INSERT INTO trading_pairs (base_asset,quote_asset,symbol,price_precision,qty_precision,min_order_value,status,market_type) VALUES (?,?,?,?,?,0,'disabled','internal')")
        .bind(base).bind(quote).bind(pair_symbol.as_str()).bind(price_precision).bind(quantity_precision)
        .execute(&mut **tx).await?.last_insert_id();
    // 新交易对尚未提交、没有运行中的生成器；只在审批事务创建停用版本，不需要排空外部写入者。
    crate::modules::admin::infrastructure::save_default_market_in_tx(
        tx,
        &crate::modules::admin::infrastructure::DefaultMarketRecord {
            pair_id,
            enabled: false,
            all_market_paused: false,
            initial_price: Some(policy.purchase_initial_price.clone()),
            config_json: sqlx::types::Json(serde_json::json!(
                crate::modules::market::synthetic_default::DefaultMarketParameters::default()
            )),
            version: 1,
            seed: format!("issuance:{project_id}:{pair_id}"),
            updated_at: chrono::Utc::now(),
        },
        Some(admin_id),
    )
    .await?;
    let due = policy
        .subscription
        .automatic
        .then_some(policy.subscription.at);
    sqlx::query("UPDATE new_coin_projects SET issuance_policy=?,issuer_agent_id=?,next_transition_at=?,next_transition_attempt_at=?,post_listing_pair_id=? WHERE id=? AND issuance_policy IS NULL")
        .bind(sqlx::types::Json(policy)).bind(agent_id).bind(due).bind(due).bind(pair_id).bind(project_id)
        .execute(&mut **tx).await?;
    Ok(())
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
