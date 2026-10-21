//! 新版发行持久化公共边界；项目先锁，实际节点与操作日志原子提交，旧合同不在这里重解释。
use super::*;
use crate::modules::new_coin::domain::issuance::{HoldingLock, IssuancePolicy};
use chrono::DateTime;
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::types::Json;

pub(crate) mod allocation;
pub(crate) mod trading;

/// 新项目的锁内快照，NULL 合同表示旧版；交易对身份随批准固定，不允许其他现货路径替代。
#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct Project {
    pub id: u64,
    pub asset_id: u64,
    pub issue_price: BigDecimal,
    pub quote_asset_id: Option<u64>,
    pub symbol: String,
    pub lifecycle_status: String,
    pub status: String,
    pub issuer_agent_id: Option<u64>,
    pub post_listing_pair_id: Option<u64>,
    pub issuance_policy: Option<Json<IssuancePolicy>>,
    #[serde(with = "crate::time::option_unix_millis")]
    pub actual_purchase_at: Option<DateTime<Utc>>,
    #[serde(with = "crate::time::option_unix_millis")]
    pub actual_listed_at: Option<DateTime<Utc>>,
    #[serde(with = "crate::time::option_unix_millis")]
    pub next_transition_at: Option<DateTime<Utc>>,
    pub transition_error: Option<String>,
}

/// 估值只读取本人已有非零钱包对应的新版项目；不把待派发订单的预期数量加进钱包资产。
pub(crate) async fn wallet_projects(pool: &Pool<MySql>, user: u64) -> AppResult<Vec<Project>> {
    Ok(sqlx::query_as("SELECT p.* FROM new_coin_projects p JOIN wallet_accounts w ON w.asset_id=p.asset_id WHERE w.user_id=? AND (w.available<>0 OR w.frozen<>0 OR w.locked<>0) AND p.issuance_policy IS NOT NULL ORDER BY p.id")
        .bind(user).fetch_all(pool).await?)
}
impl Project {
    /// 交易和调度仅接受显式第二版合同；不为历史项目填充新默认值。
    pub fn policy(&self) -> AppResult<&IssuancePolicy> {
        self.issuance_policy
            .as_ref()
            .map(|p| &p.0)
            .filter(|p| p.version == 2)
            .ok_or_else(|| AppError::Validation("该项目不使用新版发行流程".into()))
    }
    /// 审核确定的交易对不可缺失；缺少关联时停止交易，不能按币种猜测。
    pub fn pair(&self) -> AppResult<u64> {
        self.post_listing_pair_id
            .ok_or_else(|| AppError::Conflict("项目未配置交易对".into()))
    }
    /// 计价资产是资金唯一单位；NULL 只允许旧版读取，不能用于新版结算。
    pub fn quote(&self) -> AppResult<u64> {
        self.quote_asset_id
            .ok_or_else(|| AppError::Conflict("项目缺少计价资产".into()))
    }
}

/// 项目锁串行化阶段、申购、派发和成交；各调用方随后按资产、用户钱包稳定排序取锁。
pub(crate) async fn lock_project(tx: &mut Transaction<'_, MySql>, id: u64) -> AppResult<Project> {
    sqlx::query_as("SELECT * FROM new_coin_projects WHERE id=? FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)
}

/// 只读定位项目；后续写事务必须再次锁定校验，不能依赖此快照授权或定价。
pub(crate) async fn project(pool: &Pool<MySql>, id: u64) -> AppResult<Project> {
    sqlx::query_as("SELECT * FROM new_coin_projects WHERE id=?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

/// 符号入口定位完整项目，包括历史项目；不会用相似符号或其他资产替代。
pub(crate) async fn project_by_symbol(pool: &Pool<MySql>, symbol: &str) -> AppResult<Project> {
    sqlx::query_as("SELECT * FROM new_coin_projects WHERE symbol=?")
        .bind(symbol)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

/// 代理只看自己发行的项目控制面；买家记录使用独立的子树范围，与发行人无关。
pub(crate) async fn own_projects(
    pool: &Pool<MySql>,
    agent_id: u64,
    before: Option<u64>,
) -> AppResult<Vec<Project>> {
    Ok(sqlx::query_as(
        "SELECT * FROM new_coin_projects WHERE issuer_agent_id=? AND (? IS NULL OR id < ?) ORDER BY id DESC LIMIT 101",
    )
    .bind(agent_id)
    .bind(before)
    .bind(before)
    .fetch_all(pool)
    .await?)
}

/// 所有阶段/派发/回购操作保存真实角色身份；不得把代理或调度器写成批准该项目的管理员。
pub(crate) async fn operation(
    tx: &mut Transaction<'_, MySql>,
    project: u64,
    actor: (&str, Option<u64>),
    action: &str,
    payload: Value,
    reason: &str,
) -> AppResult<()> {
    sqlx::query("INSERT INTO new_coin_operations(project_id,actor_type,actor_id,action,payload_json,reason) VALUES (?,?,?,?,?,?)")
        .bind(project).bind(actor.0).bind(actor.1).bind(action).bind(Json(&payload)).bind(reason).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO new_coin_lifecycle_events(project_id,event_type,payload_json,created_by) VALUES (?,?,?,?)")
        .bind(project).bind(action).bind(Json(json!({"actor_type":actor.0,"actor_id":actor.1,"data":payload,"reason":reason})))
        .bind(if actor.0 == "admin" {actor.1} else {None}).execute(&mut **tx).await?;
    Ok(())
}

/// 以数据库 UTC 时钟作为阶段和报价共同的有效性基准，避免不同 API/worker 主机时钟漂移。
pub(crate) async fn now(tx: &mut Transaction<'_, MySql>) -> AppResult<DateTime<Utc>> {
    Ok(sqlx::query_scalar("SELECT UTC_TIMESTAMP(6)")
        .fetch_one(&mut **tx)
        .await?)
}

/// 调用方已持有生成器命名锁和项目行锁；阶段只前进一步，启用原始种子后永不重置行情。
pub(crate) async fn advance(
    tx: &mut Transaction<'_, MySql>,
    before: &Project,
    target: &str,
    actor: (&str, Option<u64>),
    reason: &str,
    scheduled: bool,
) -> AppResult<Project> {
    let policy = before.policy()?;
    let (expected, node) = policy
        .next(&before.lifecycle_status)
        .ok_or_else(|| AppError::Conflict("项目已经上市".into()))?;
    let now = now(tx).await?;
    if target != expected
        || before.status != "active"
        || (scheduled && (!node.automatic || node.at > now))
    {
        return Err(AppError::Conflict("项目阶段或自动节点已变化".into()));
    }
    if matches!(target, "purchase" | "listed") {
        crate::modules::admin::infrastructure::ensure_manual_new_coin_subscriptions_settled_in_tx(
            tx, before.id,
        )
        .await?;
    }
    if target == "purchase" {
        let pair = before.pair()?;
        sqlx::query("UPDATE trading_pairs SET status='active' WHERE id=? AND base_asset=? AND quote_asset=? AND market_type IN ('internal','strategy')")
            .bind(pair).bind(before.asset_id).bind(before.quote()?).execute(&mut **tx).await?;
        let mut saved = crate::modules::admin::infrastructure::load_default_market_in_tx(tx, pair)
            .await?
            .ok_or_else(|| AppError::Conflict("项目行情配置缺失".into()))?;
        if saved.all_market_paused {
            return Err(AppError::Conflict("项目行情处于暂停状态".into()));
        }
        if !saved.enabled {
            saved.enabled = true;
            saved.version = saved
                .version
                .checked_add(1)
                .ok_or_else(|| AppError::Conflict("行情版本超限".into()))?;
            crate::modules::admin::infrastructure::save_default_market_in_tx(
                tx,
                &saved,
                if actor.0 == "admin" { actor.1 } else { None },
            )
            .await?;
        }
    }
    if target == "listed" {
        trading::handoff_inventory(tx, before).await?;
        sqlx::query("UPDATE trading_pairs p JOIN assets a ON a.id=p.base_asset SET p.logo_url=COALESCE(a.logo_url,p.logo_url) WHERE p.id=? AND p.base_asset=? AND p.quote_asset=?")
            .bind(before.pair()?).bind(before.asset_id).bind(before.quote()?).execute(&mut **tx).await?;
    }
    let due = policy
        .next(target)
        .and_then(|(_, n)| n.automatic.then_some(n.at));
    sqlx::query("UPDATE new_coin_projects SET lifecycle_status=?,actual_purchase_at=COALESCE(actual_purchase_at,?),actual_listed_at=COALESCE(actual_listed_at,?),next_transition_at=?,next_transition_attempt_at=?,transition_error=NULL WHERE id=?")
        .bind(target).bind((target=="purchase").then_some(now)).bind((target=="listed").then_some(now))
        .bind(due).bind(due).bind(before.id).execute(&mut **tx).await?;
    let after = lock_project(tx, before.id).await?;
    operation(
        tx,
        before.id,
        actor,
        "new_coin.stage",
        json!({"before":before,"after":after}),
        reason,
    )
    .await?;
    Ok(after)
}

/// 持久化公平调度候选；失败项至少退避一分钟，其他到期项目不受阻塞，重启后继续同一队列。
pub(crate) async fn due_projects(pool: &Pool<MySql>) -> AppResult<Vec<Project>> {
    Ok(sqlx::query_as("SELECT * FROM new_coin_projects WHERE status='active' AND next_transition_at<=UTC_TIMESTAMP(6) AND next_transition_attempt_at<=UTC_TIMESTAMP(6) ORDER BY next_transition_attempt_at,id LIMIT 100")
        .fetch_all(pool).await?)
}

/// 只标记仍处于原阶段的失败调度，不覆盖其他实例已经完成的节点或下一节点排程。
pub(crate) async fn defer_transition(pool: &Pool<MySql>, id: u64, stage: &str) -> AppResult<()> {
    sqlx::query("UPDATE new_coin_projects SET next_transition_attempt_at=DATE_ADD(UTC_TIMESTAMP(6),INTERVAL 1 MINUTE),transition_error='transition_blocked' WHERE id=? AND lifecycle_status=?")
        .bind(id).bind(stage).execute(pool).await?;
    Ok(())
}

/// 根据持仓原始合同与真实项目事件计算可卖状态；预计时间不会替代真实阶段，成熟不会自动卖出。
pub(crate) fn eligible(
    policy: &IssuancePolicy,
    unlock_at: Option<DateTime<Utc>>,
    p: &Project,
    now: DateTime<Utc>,
) -> bool {
    let at = match policy.lock {
        HoldingLock::OnListing => p.actual_listed_at,
        HoldingLock::OnPurchase => p.actual_purchase_at,
        _ => unlock_at,
    };
    at.is_some_and(|at| at <= now)
}

/// 新版资金贷记适配统一流水元数据；调用方持钱包行锁，失败时和业务收据一起回滚。
pub(crate) async fn credit_balance(
    tx: &mut Transaction<'_, MySql>,
    user: u64,
    asset: u64,
    amount: &BigDecimal,
    meta: NewCoinLedgerMetadata<'_>,
) -> AppResult<()> {
    credit_wallet_available(
        tx,
        user,
        asset,
        amount,
        meta.change_type,
        meta.ref_type,
        meta.ref_id,
    )
    .await
}

/// 新建项目事务内固化合同、交易对及停用生成器；管理员发行无代理归属，审核发行记录原代理。
pub(crate) async fn publish_policy(
    tx: &mut Transaction<'_, MySql>,
    project_id: u64,
    agent_id: Option<u64>,
    admin_id: u64,
    policy: &crate::modules::new_coin::domain::issuance::IssuancePolicy,
) -> AppResult<()> {
    let (base, quote, symbol, quote_symbol, quantity_precision, price_precision, logo): (u64,u64,String,String,i32,i32,Option<String>) = sqlx::query_as("SELECT p.asset_id,p.quote_asset_id,p.symbol,q.symbol,a.precision_scale,q.precision_scale,a.logo_url FROM new_coin_projects p JOIN assets a ON a.id=p.asset_id JOIN assets q ON q.id=p.quote_asset_id WHERE p.id=?")
        .bind(project_id).fetch_one(&mut **tx).await?;
    // 交易对代码与后台手工创建保持同一口径：项目代码与计价资产用连字符连接。
    let pair_symbol =
        crate::modules::new_coin::service::post_listing_pair_symbol(&symbol, &quote_symbol)?;
    let pair_id = sqlx::query("INSERT INTO trading_pairs (base_asset,quote_asset,symbol,price_precision,qty_precision,min_order_value,status,market_type,logo_url) VALUES (?,?,?,?,?,0,'disabled','internal',?)")
        .bind(base).bind(quote).bind(&pair_symbol).bind(price_precision).bind(quantity_precision).bind(logo)
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
