//! 代理充值申请、授权锁及资金落账适配；所有写入复用调用方事务，不隐式提交。

use crate::{
    error::{AppError, AppResult},
    modules::{
        admin::infrastructure::credit_admin_wallet_available_in_tx,
        agent::{
            domain::{MAX_AGENT_LEVEL, is_same_or_descendant_path},
            presentation::recharges::*,
        },
        wallet::{
            infrastructure::insert_wallet_platform_journal_legs_in_tx,
            platform_journal::WalletPlatformJournalLeg,
        },
    },
};
use sqlx::{MySql, MySqlPool, QueryBuilder, Transaction};
use std::collections::BTreeMap;

/// 事务外预读仅决定锁集合；所有身份、归属和状态必须在资金事务里再锁定校验。
#[derive(sqlx::FromRow)]
pub(crate) struct RechargeScopeHint {
    pub agent_id: u64,
    pub agent_path: String,
    pub owner_agent_id: u64,
    pub owner_path: String,
}

#[derive(sqlx::FromRow)]
struct LockedAgent {
    id: u64,
    user_id: u64,
    path: String,
    status: String,
    direct_recharge_enabled: bool,
}

/// 只定位代理账号和目标归属，不信任客户端agent_id；不在子树或用户未归属统一返回未找到。
pub(crate) async fn recharge_scope_hint(
    pool: &MySqlPool,
    actor: u64,
    user: u64,
) -> AppResult<RechargeScopeHint> {
    sqlx::query_as(
        "SELECT a.id agent_id,a.path agent_path,o.id owner_agent_id,o.path owner_path \
         FROM agent_admin_users aa JOIN agents a ON a.id=aa.agent_id \
         JOIN user_referrals r ON r.user_id=? JOIN agents o ON o.id=r.root_agent_id \
         WHERE aa.id=? AND a.user_id<>? AND (o.path=a.path OR o.path LIKE CONCAT(a.path,'/%'))",
    )
    .bind(user)
    .bind(actor)
    .bind(user)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

/// 用户行是人工充值与改派的共同串行点；归属后代批量迁移另由后续referral共享锁阻断。
pub(crate) async fn lock_recharge_user(
    tx: &mut Transaction<'_, MySql>,
    user: u64,
) -> AppResult<String> {
    sqlx::query_scalar("SELECT status FROM users WHERE id=? FOR UPDATE")
        .bind(user)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)
}

/// 校验预读路径的规范ID及三级深度，按ID升序逐行锁主键并核对每个祖先的完整路径与状态。
/// 不以反向LIKE扫描加锁，避免锁住无关代理；路径缺失或变化时失败，不扩展预读锁集合。
/// 随后锁门户账号及用户归属，授权/停用/改派均不能穿过入账事务，重试必须重新取hint。
pub(crate) async fn lock_recharge_scope(
    tx: &mut Transaction<'_, MySql>,
    actor: u64,
    user: u64,
    session_version: Option<u64>,
    hint: &RechargeScopeHint,
) -> AppResult<bool> {
    if !is_same_or_descendant_path(&hint.agent_path, &hint.owner_path) {
        return Err(AppError::NotFound);
    }
    let mut ancestor_paths = BTreeMap::new();
    for (path, leaf_id) in [
        (&hint.agent_path, hint.agent_id),
        (&hint.owner_path, hint.owner_agent_id),
    ] {
        let segments = path.strip_prefix('/').ok_or(AppError::NotFound)?;
        let mut prefix = String::new();
        let mut last_id = None;
        for (depth, segment) in segments.split('/').enumerate() {
            let id = segment
                .strip_prefix("agent:")
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|id| *id > 0 && segment == format!("agent:{id}"))
                .ok_or(AppError::NotFound)?;
            if depth >= MAX_AGENT_LEVEL as usize {
                return Err(AppError::NotFound);
            }
            prefix.push('/');
            prefix.push_str(segment);
            if ancestor_paths
                .insert(id, prefix.clone())
                .is_some_and(|previous| previous != prefix)
            {
                return Err(AppError::NotFound);
            }
            last_id = Some(id);
        }
        if last_id != Some(leaf_id) {
            return Err(AppError::NotFound);
        }
    }
    // At most three primary-key reads avoid optimizer-dependent range or table locks.
    let mut agents = Vec::with_capacity(ancestor_paths.len());
    for (id, expected_path) in ancestor_paths {
        let agent: LockedAgent = sqlx::query_as(
            "SELECT id,user_id,path,status,direct_recharge_enabled FROM agents WHERE id=? FOR SHARE",
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)?;
        if agent.path != expected_path {
            return Err(AppError::Conflict("代理层级已变化，请重新核对".into()));
        }
        agents.push(agent);
    }
    let own = agents
        .iter()
        .find(|a| a.id == hint.agent_id)
        .ok_or(AppError::Unauthorized)?;
    let owner = agents
        .iter()
        .find(|a| a.id == hint.owner_agent_id)
        .ok_or(AppError::NotFound)?;
    if own.path != hint.agent_path
        || owner.path != hint.owner_path
        || !is_same_or_descendant_path(&own.path, &owner.path)
        || own.user_id == user
        || !own.path.starts_with("/agent:")
    {
        return Err(AppError::NotFound);
    }
    if agents.iter().any(|a| a.status != "active") {
        return Err(AppError::Conflict("代理或上级已停用，不能充值".into()));
    }
    let account: Option<(u64, String, u64)> = sqlx::query_as(
        "SELECT agent_id,status,auth_session_version FROM agent_admin_users WHERE id=? FOR UPDATE",
    )
    .bind(actor)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((agent_id, status, version)) = account else {
        return Err(AppError::Unauthorized);
    };
    if agent_id != own.id || status != "active" || session_version.is_some_and(|v| v != version) {
        return Err(AppError::Unauthorized);
    }
    let actual_owner: Option<u64> =
        sqlx::query_scalar("SELECT root_agent_id FROM user_referrals WHERE user_id=? FOR SHARE")
            .bind(user)
            .fetch_optional(&mut **tx)
            .await?;
    if actual_owner != Some(owner.id) {
        return Err(AppError::Conflict("用户归属已变化，请重新核对".into()));
    }
    Ok(own.direct_recharge_enabled)
}

/// 门户账号锁串行化同账号提交；此前只执行锁定读，此处首次一致性读取可见等待期间提交的收据。
/// 不锁缺失收据的范围，避免不同用户/账号在空唯一索引上同时插入时形成gap锁死锁。
pub(crate) async fn recharge_by_key(
    tx: &mut Transaction<'_, MySql>,
    actor: u64,
    key: &str,
) -> AppResult<Option<AgentRechargeRecord>> {
    sqlx::query_as(
        "SELECT * FROM agent_wallet_recharges WHERE agent_admin_id=? AND idempotency_key=?",
    )
    .bind(actor)
    .bind(key.as_bytes())
    .fetch_optional(&mut **tx)
    .await
    .map_err(Into::into)
}

/// 只读取得不可变申请身份，供审核先锁用户再锁申请，避免与提交路径形成反向锁序。
pub(crate) async fn recharge_by_id(pool: &MySqlPool, id: u64) -> AppResult<AgentRechargeRecord> {
    sqlx::query_as("SELECT * FROM agent_wallet_recharges WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

/// 在既有用户锁后锁定申请，以最新终态决定批准、拒绝或原审核重放。
pub(crate) async fn lock_recharge(
    tx: &mut Transaction<'_, MySql>,
    id: u64,
) -> AppResult<AgentRechargeRecord> {
    sqlx::query_as("SELECT * FROM agent_wallet_recharges WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)
}

/// 创建不可变金额及审核路径，唯一键防止同账号重复发起；直充与后续落账共用事务。
pub(crate) async fn insert_recharge(
    tx: &mut Transaction<'_, MySql>,
    agent_id: u64,
    actor: u64,
    user: u64,
    asset_symbol: &str,
    request: &AgentRechargeRequest,
    direct: bool,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "INSERT INTO agent_wallet_recharges \
         (agent_id,agent_admin_id,user_id,asset_id,asset_symbol,amount,reason,idempotency_key,mode,status,credited_at) \
         VALUES (?,?,?,?,?,?,?,?,?,?,IF(?,UTC_TIMESTAMP(3),NULL))",
    )
    .bind(agent_id).bind(actor).bind(user).bind(request.asset_id).bind(asset_symbol)
    .bind(&request.amount).bind(&request.reason).bind(request.idempotency_key.as_bytes())
    .bind(if direct { "direct" } else { "review" })
    .bind(if direct { "credited" } else { "pending" }).bind(direct)
    .execute(&mut **tx).await.map(|r| r.last_insert_id())
}

/// 仅pending申请可迁移，审核人和理由与终态同时持久化，不允许覆写原金额或第二次审核。
pub(crate) async fn review_recharge(
    tx: &mut Transaction<'_, MySql>,
    id: u64,
    admin: u64,
    approve: bool,
    reason: &str,
) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE agent_wallet_recharges SET status=?,reviewed_by=?,review_reason=?,reviewed_at=UTC_TIMESTAMP(3), \
         credited_at=IF(?,UTC_TIMESTAMP(3),NULL) WHERE id=? AND status='pending' AND mode='review'",
    ).bind(if approve { "credited" } else { "rejected" }).bind(admin).bind(reason)
        .bind(approve).bind(id).execute(&mut **tx).await?;
    if result.rows_affected() != 1 {
        return Err(AppError::Conflict("充值申请已处理".into()));
    }
    Ok(())
}

/// 增加现货可用余额、原始流水及同资产平衡分录；上层已锁定申请并校验精度与幂等。
/// 清算科目只记录人工加币来源，不伪装链上到账或扣代理余额，任一腿失败整笔回滚。
pub(crate) async fn credit_recharge(
    tx: &mut Transaction<'_, MySql>,
    record: &AgentRechargeRecord,
) -> AppResult<()> {
    credit_admin_wallet_available_in_tx(
        tx,
        record.user_id,
        record.asset_id,
        &record.amount,
        "agent_recharge",
        "agent_recharge",
        &record.id.to_string(),
    )
    .await?;
    insert_wallet_platform_journal_legs_in_tx(
        tx,
        "wallet",
        &format!("agent_recharge:{}", record.id),
        record.asset_id,
        "agent_recharge",
        record.id,
        &[
            WalletPlatformJournalLeg {
                account_code: "agent_recharge_clearing",
                amount: record.amount.clone(),
            },
            WalletPlatformJournalLeg {
                account_code: "user_wallet_liability",
                amount: -record.amount.clone(),
            },
        ],
    )
    .await
}

/// 必需申请审计区分代理账号与管理员身份；与申请、钱包和终态共事务，不允许资金成功审计失败。
pub(crate) async fn audit_recharge(
    tx: &mut Transaction<'_, MySql>,
    id: u64,
    actor_type: &str,
    actor: u64,
    action: &str,
    reason: &str,
) -> AppResult<()> {
    sqlx::query("INSERT INTO agent_recharge_audits (request_id,actor_type,actor_id,action,reason) VALUES (?,?,?,?,?)")
        .bind(id).bind(actor_type).bind(actor).bind(action).bind(reason).execute(&mut **tx).await?;
    Ok(())
}

/// 授权以agents单行保存，缺省为false；读取不初始化钱包或修改策略。
pub(crate) async fn recharge_policy(
    pool: &MySqlPool,
    agent: u64,
) -> AppResult<AgentRechargePolicy> {
    sqlx::query_as("SELECT id agent_id,direct_recharge_enabled,recharge_policy_version version FROM agents WHERE id=?")
        .bind(agent).fetch_optional(pool).await?.ok_or(AppError::NotFound)
}

/// 授权变更排他锁与资金路径祖先共享锁协调；等待期间旧直充先完成，提交后新申请必见新授权。
pub(crate) async fn lock_recharge_policy(
    tx: &mut Transaction<'_, MySql>,
    agent: u64,
) -> AppResult<AgentRechargePolicy> {
    sqlx::query_as("SELECT id agent_id,direct_recharge_enabled,recharge_policy_version version FROM agents WHERE id=? FOR UPDATE")
        .bind(agent).fetch_optional(&mut **tx).await?.ok_or(AppError::NotFound)
}

/// 仅更新免审标记和版本，不扫描待审核单、不赋予下级权限，审计由调用方同事务补齐。
pub(crate) async fn store_recharge_policy(
    tx: &mut Transaction<'_, MySql>,
    policy: &AgentRechargePolicy,
) -> AppResult<()> {
    sqlx::query("UPDATE agents SET direct_recharge_enabled=?,recharge_policy_version=? WHERE id=?")
        .bind(policy.direct_recharge_enabled)
        .bind(policy.version)
        .bind(policy.agent_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// 只列启用资产及权威精度；异常精度失败关闭，不静默回退到18位。
pub(crate) async fn recharge_assets(pool: &MySqlPool) -> AppResult<Vec<RechargeAsset>> {
    let assets: Vec<RechargeAsset> = sqlx::query_as("SELECT id asset_id,symbol,precision_scale FROM assets WHERE status='active' ORDER BY id LIMIT 1001")
        .fetch_all(pool).await?;
    if assets.len() > 1000
        || assets
            .iter()
            .any(|a| !(0..=18).contains(&a.precision_scale))
    {
        return Err(AppError::Internal("充值资产目录过大或精度配置无效".into()));
    }
    Ok(assets)
}

/// 分页筛选同一只读事务内共享谓词；代理只看本人提交且仍在当前子树内的记录，不能借历史跨树读用户。
pub(crate) async fn list_recharges(
    pool: &MySqlPool,
    query: &AdminAgentRechargeQuery,
    actor_scope: Option<(u64, &str)>,
) -> AppResult<AgentRechargesResponse> {
    let mut rows = QueryBuilder::<MySql>::new("SELECT r.* FROM agent_wallet_recharges r WHERE 1=1");
    let mut count =
        QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM agent_wallet_recharges r WHERE 1=1");
    for builder in [&mut rows, &mut count] {
        if let Some((actor, path)) = actor_scope {
            builder.push(" AND r.agent_admin_id=").push_bind(actor);
            builder.push(" AND EXISTS (SELECT 1 FROM user_referrals ur JOIN agents a ON a.id=ur.root_agent_id \
                WHERE ur.user_id=r.user_id AND (a.path=").push_bind(path.to_owned())
                .push(" OR a.path LIKE CONCAT(").push_bind(path.to_owned()).push(",'/%')))");
        }
        if let Some(status) = &query.status {
            builder.push(" AND r.status=").push_bind(status.clone());
        }
        if let Some(user) = query.user_id {
            builder.push(" AND r.user_id=").push_bind(user);
        }
        if let Some(agent) = query.agent_id {
            builder.push(" AND r.agent_id=").push_bind(agent);
        }
    }
    rows.push(" ORDER BY r.id DESC LIMIT ")
        .push_bind(query.limit.unwrap_or(20).clamp(1, 100))
        .push(" OFFSET ")
        .push_bind(query.offset.unwrap_or(0).min(100_000));
    let mut tx = pool.begin().await?;
    let recharges = rows.build_query_as().fetch_all(&mut *tx).await?;
    let total = count.build_query_scalar().fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(AgentRechargesResponse { recharges, total })
}
