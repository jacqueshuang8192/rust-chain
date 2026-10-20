//! 新发行用例编排：代理权限、事务顺序、报价确认和持久化阶段调度。
use crate::{
    error::{AppError, AppResult},
    modules::{
        admin::{
            infrastructure::load_admin_new_coin_project_in_tx, service::new_coin_project_audit_json,
        },
        agent::{
            application::agent_context,
            infrastructure::{coin_applications::lock_actor, recharges},
            service::agent_admin_id_from_subject,
        },
        market::infrastructure::default_runtime::PairGenerationLock,
        new_coin::{infrastructure::issuance as store, presentation::issuance::*},
    },
};
use redis::aio::ConnectionManager;
use serde_json::{Value, json};
use sqlx::{MySql, Pool};
use std::collections::{BTreeMap, BTreeSet};
use store::{allocation, trading};

fn pool(mysql: Option<Pool<MySql>>) -> AppResult<Pool<MySql>> {
    mysql.ok_or_else(|| AppError::Internal("mysql未配置".into()))
}
fn user(subject: &str) -> AppResult<u64> {
    crate::modules::new_coin::service::user_id_from_subject(subject)
}
fn reason(value: &str) -> AppResult<String> {
    let s = value.trim();
    if s.is_empty() || s.chars().count() > 500 {
        Err(AppError::Validation("操作原因需为1至500字".into()))
    } else {
        Ok(s.into())
    }
}
fn key(value: &str) -> AppResult<String> {
    crate::modules::admin::service::normalize_admin_recharge_idempotency_key(value)
}

/// 发行控制面按令牌代理隔离，与下级购买记录使用不同范围；返回实际阶段及排程失败状态。
pub(crate) async fn agent_projects(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    query: ProjectsQuery,
) -> AppResult<Value> {
    let (pool, scope) = agent_context(mysql, subject).await?;
    let mut rows = store::own_projects(&pool, scope.agent_id, query.before_id).await?;
    let has_more = rows.len() > 100;
    rows.truncate(100);
    let next = has_more.then(|| rows.last().map(|p| p.id)).flatten();
    Ok(json!({"projects": rows, "next_before_id": next}))
}

/// 代理只能推进自己发行的项目；先持行情协调锁再锁代理身份和项目，任何归属变化均使事务失败。
pub(crate) async fn agent_stage(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    version: u64,
    id: u64,
    req: StageRequest,
) -> AppResult<store::Project> {
    let reason = reason(&req.reason)?;
    let actor = agent_admin_id_from_subject(subject)?;
    let (pool, scope) = agent_context(mysql, subject).await?;
    let hint = store::project(&pool, id).await?;
    if hint.issuer_agent_id != Some(scope.agent_id) {
        return Err(AppError::NotFound);
    }
    let mut guard = PairGenerationLock::acquire(&pool, hint.pair()?, 5)
        .await?
        .ok_or_else(|| AppError::Conflict("行情正在切换，请稍后重试".into()))?;
    let mut tx = pool.begin().await?;
    lock_actor(&mut tx, actor, &scope, Some(version)).await?;
    let before = store::lock_project(&mut tx, id).await?;
    if before.issuer_agent_id != Some(scope.agent_id) {
        return Err(AppError::NotFound);
    }
    if before.lifecycle_status == req.target_stage {
        tx.commit().await?;
        return Ok(before);
    }
    if before.lifecycle_status != req.expected_stage {
        return Err(AppError::Conflict("项目阶段已变化，请刷新".into()));
    }
    guard.ensure_owned().await?;
    let after = store::advance(
        &mut tx,
        &before,
        &req.target_stage,
        ("agent", Some(actor)),
        &reason,
        false,
    )
    .await?;
    guard.ensure_owned().await?;
    tx.commit().await?;
    Ok(after)
}

/// 管理员原阶段接口进入新版分支；原配置版本在命名锁和项目行锁后重验，禁止跳过认购阶段。
pub(crate) async fn admin_stage(
    mysql: Pool<MySql>,
    admin: u64,
    id: u64,
    target: &str,
    expected_config: Option<&str>,
    audit_reason: &str,
) -> AppResult<()> {
    let reason = reason(audit_reason)?;
    let hint = store::project(&mysql, id).await?;
    let mut guard = PairGenerationLock::acquire(&mysql, hint.pair()?, 5)
        .await?
        .ok_or_else(|| AppError::Conflict("行情正在切换".into()))?;
    let mut tx = mysql.begin().await?;
    let before = store::lock_project(&mut tx, id).await?;
    let existing = load_admin_new_coin_project_in_tx(&mut tx, id).await?;
    let current_config = new_coin_project_audit_json(&existing).to_string();
    if expected_config.is_some_and(|v| v != current_config) {
        return Err(AppError::Conflict("项目配置已更新，请刷新".into()));
    }
    guard.ensure_owned().await?;
    store::advance(
        &mut tx,
        &before,
        target,
        ("admin", Some(admin)),
        &reason,
        false,
    )
    .await?;
    guard.ensure_owned().await?;
    tx.commit().await?;
    Ok(())
}

/// 每一批只处理显式选择的下级订单；所有归属在写事务里锁定，混入外部订单整批拒绝，无部分扣款。
pub(crate) async fn allocate(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    version: u64,
    mut req: AllocationRequest,
) -> AppResult<Value> {
    req.reason = reason(&req.reason)?;
    req.idempotency_key = key(&req.idempotency_key)?;
    if req.subscription_ids.is_empty()
        || req.subscription_ids.len() > 100
        || req.subscription_ids.contains(&0)
    {
        return Err(AppError::Validation("每批需选择1至100笔订单".into()));
    }
    req.subscription_ids.sort_unstable();
    if req.subscription_ids.windows(2).any(|w| w[0] == w[1]) {
        return Err(AppError::Validation("批次不能重复选择订单".into()));
    }
    crate::modules::new_coin::domain::issuance::allocation_quantity(&1.into(), &req.percent, 18)?;
    let actor = agent_admin_id_from_subject(subject)?;
    let (pool, scope) = agent_context(mysql, subject).await?;
    let orders = allocation::hints(&pool, &req.subscription_ids).await?;
    if orders.len() != req.subscription_ids.len() {
        return Err(AppError::NotFound);
    }
    let users: BTreeSet<_> = orders.iter().map(|o| o.user_id).collect();
    let mut hints = BTreeMap::new();
    for id in &users {
        hints.insert(
            *id,
            recharges::recharge_scope_hint(&pool, actor, *id).await?,
        );
    }
    let intent = json!({"ids":req.subscription_ids,"percent":req.percent.normalized().to_string(),"reason":req.reason});
    let mut tx = pool.begin().await?;
    for id in &users {
        if recharges::lock_recharge_user(&mut tx, *id).await? != "active" {
            return Err(AppError::Conflict("批次包含停用用户".into()));
        }
    }
    lock_actor(&mut tx, actor, &scope, Some(version)).await?;
    for (id, hint) in &hints {
        recharges::lock_recharge_scope(&mut tx, actor, *id, Some(version), hint).await?;
    }
    if let Some(result) = allocation::replay(&mut tx, actor, &req.idempotency_key, &intent).await? {
        tx.commit().await?;
        return Ok(result);
    }
    let ids: BTreeSet<_> = orders.iter().map(|o| o.project_id).collect();
    let mut projects = BTreeMap::new();
    for id in ids {
        projects.insert(id, store::lock_project(&mut tx, id).await?);
    }
    let treasury = trading::treasury(&mut tx).await?;
    let mut wallet_users = users.iter().copied().collect::<Vec<_>>();
    wallet_users.push(treasury);
    trading::lock_batch_money(
        &mut tx,
        &projects.values().collect::<Vec<_>>(),
        &wallet_users,
    )
    .await?;
    let mut results = Vec::new();
    for order in &orders {
        let project = projects.get(&order.project_id).ok_or(AppError::NotFound)?;
        let result = allocation::allocate(
            &mut tx,
            project,
            order,
            &req.percent,
            &format!("agent:{actor}:{}:{}", req.idempotency_key, order.id),
            None,
        )
        .await?;
        store::operation(
            &mut tx,
            project.id,
            ("agent", Some(actor)),
            "new_coin.allocate",
            result.clone(),
            &req.reason,
        )
        .await?;
        results.push(result);
    }
    let response = json!({"idempotency_key":req.idempotency_key,"results":results});
    allocation::receipt(&mut tx, actor, &req.idempotency_key, &intent, &response).await?;
    tx.commit().await?;
    Ok(response)
}

/// 报价以服务端实时行情定价并绑定用户意图；重试返回原报价，过期后需要用户主动重新报价。
pub(crate) async fn quote(
    mysql: Option<Pool<MySql>>,
    redis: Option<&ConnectionManager>,
    subject: &str,
    symbol: &str,
    mut req: TradeQuoteRequest,
) -> AppResult<TradeQuote> {
    req.idempotency_key = key(&req.idempotency_key)?;
    if !matches!(req.side.as_str(), "buy" | "sell") {
        return Err(AppError::Validation("交易方向无效".into()));
    }
    let user = user(subject)?;
    let pool = pool(mysql)?;
    let hint = store::project_by_symbol(&pool, symbol).await?;
    let intent = json!({"project_id":hint.id,"side":req.side,"quantity":req.quantity.as_ref().map(|n|n.normalized().to_string()),"holding_id":req.holding_id});
    let mut tx = pool.begin().await?;
    trading::lock_user(&mut tx, user).await?;
    if let Some(q) = trading::quote_replay(&mut tx, user, &req.idempotency_key, &intent).await? {
        tx.commit().await?;
        return Ok(q);
    }
    let p = store::lock_project(&mut tx, hint.id).await?;
    p.policy()?;
    if p.status != "active" || !matches!(p.lifecycle_status.as_str(), "purchase" | "listed") {
        return Err(AppError::Conflict("项目尚未开放行情交易".into()));
    }
    let pair = trading::pair_symbol(&mut tx, &p).await?;
    let (base_precision, quote_precision) = trading::lock_money(&mut tx, &p, &[]).await?;
    let now = store::now(&mut tx).await?;
    let mut price = trading::price(redis, &pair, now).await?;
    let engine = trading::engine_ready(&mut tx, &p, &price.source).await?;
    price.source_version = format!("{};{engine}", price.source_version);
    let q = trading::save_quote(
        &mut tx,
        &p,
        user,
        &req,
        &price,
        intent,
        (base_precision, quote_precision),
    )
    .await?;
    tx.commit().await?;
    Ok(q)
}

/// 认购以提交时有效行情一次成交；内部报价与钱包、持仓、审计同事务提交，不留下待确认报价。
/// 用户锁串行化幂等键；终态先于阶段和行情检查重放，未知结果重试不会按新价格再次购买。
pub(crate) async fn market_purchase(
    mysql: Option<Pool<MySql>>,
    redis: Option<&ConnectionManager>,
    subject: &str,
    symbol: &str,
    req: MarketPurchaseRequest,
) -> AppResult<Value> {
    let user = user(subject)?;
    let pool = pool(mysql)?;
    let key = key(&req.idempotency_key)?;
    let hint = store::project_by_symbol(&pool, symbol).await?;
    let intent = json!({"operation":"market_purchase","project_id":hint.id,"quantity":req.quantity.normalized().to_string()});
    let mut tx = pool.begin().await?;
    trading::lock_user(&mut tx, user).await?;
    if let Some(q) = trading::quote_replay(&mut tx, user, &key, &intent).await? {
        let receipt = q
            .receipt_json
            .filter(|_| q.status == "executed")
            .ok_or_else(|| AppError::Conflict("成交凭证尚未完成".into()))?;
        tx.commit().await?;
        return Ok(json!({"idempotency_key":key,"receipt":receipt.0}));
    }
    let p = store::lock_project(&mut tx, hint.id).await?;
    p.policy()?;
    if p.status != "active" || p.lifecycle_status != "purchase" {
        return Err(AppError::Conflict("项目不在认购阶段".into()));
    }
    let pair = trading::pair_symbol(&mut tx, &p).await?;
    let treasury = trading::treasury(&mut tx).await?;
    let precision = trading::lock_money(&mut tx, &p, &[user, treasury]).await?;
    let now = store::now(&mut tx).await?;
    let mut price = trading::price(redis, &pair, now).await?;
    let engine = trading::engine_ready(&mut tx, &p, &price.source).await?;
    price.source_version = format!("{};{engine}", price.source_version);
    let q = trading::save_quote(
        &mut tx,
        &p,
        user,
        &TradeQuoteRequest {
            side: "buy".into(),
            quantity: Some(req.quantity),
            holding_id: None,
            idempotency_key: key.clone(),
        },
        &price,
        intent,
        precision,
    )
    .await?;
    let receipt = trading::execute(&mut tx, &p, &q, treasury).await?;
    tx.commit().await?;
    Ok(json!({"idempotency_key":key,"receipt":receipt}))
}

/// 一次确认原子完成买入或解锁卖出；终态重放返回原收据，行情故障、余额不足或日志故障全部回滚。
pub(crate) async fn confirm(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    id: &str,
) -> AppResult<Value> {
    let user = user(subject)?;
    let pool = pool(mysql)?;
    let hint = trading::quote_hint(&pool, user, id).await?;
    let mut tx = pool.begin().await?;
    trading::lock_user(&mut tx, user).await?;
    let p = store::lock_project(&mut tx, hint.project_id).await?;
    let q = trading::lock_quote(&mut tx, user, id).await?;
    if let Some(receipt) = &q.receipt_json
        && q.status == "executed"
    {
        let result = receipt.0.clone();
        tx.commit().await?;
        return Ok(result);
    }
    let treasury = trading::treasury(&mut tx).await?;
    trading::pair_symbol(&mut tx, &p).await?;
    let engine = trading::engine_ready(&mut tx, &p, &q.source).await?;
    if q.source_version.rsplit_once(';').map(|(_, v)| v) != Some(engine.as_str()) {
        return Err(AppError::Conflict("行情引擎已切换，请重新报价".into()));
    }
    trading::lock_money(&mut tx, &p, &[user, treasury]).await?;
    let receipt = trading::execute(&mut tx, &p, &q, treasury).await?;
    tx.commit().await?;
    Ok(receipt)
}

/// 持仓页面返回原成本、锁规则和实时估值；无有效行情时显式置空，不能用发行价伪造收益。
pub(crate) async fn holdings(
    mysql: Option<Pool<MySql>>,
    redis: Option<&ConnectionManager>,
    subject: &str,
    query: crate::modules::new_coin::presentation::issuance::HoldingsQuery,
) -> AppResult<Value> {
    let user = user(subject)?;
    let pool = pool(mysql)?;
    if query.project_id == Some(0) || query.before_id == Some(0) {
        return Err(AppError::Validation("持仓查询编号无效".into()));
    }
    let mut rows = trading::holdings(&pool, user, query.project_id, query.before_id).await?;
    let has_more = rows.len() > 100;
    rows.truncate(100);
    let next_before_id = if has_more {
        rows.last().map(|h| h.id)
    } else {
        None
    };
    let mut result = Vec::new();
    let now = chrono::Utc::now();
    for h in rows {
        let p = store::project(&pool, h.project_id).await?;
        let mut tx = pool.begin().await?;
        let pair = trading::pair_symbol(&mut tx, &p).await.ok();
        let price = if let Some(symbol) = pair.as_deref() {
            match trading::price(redis, symbol, now).await {
                Ok(price)
                    if trading::engine_ready(&mut tx, &p, &price.source)
                        .await
                        .is_ok() =>
                {
                    Some(price)
                }
                _ => None,
            }
        } else {
            None
        };
        let quote_symbol: String =
            crate::modules::new_coin::infrastructure::issuance::trading::quote_symbol(
                &mut tx,
                p.quote()?,
            )
            .await?;
        let quote_precision = trading::quote_precision(&mut tx, p.quote()?).await?;
        tx.commit().await?;
        let market_value = price
            .as_ref()
            .map(|v| {
                (&h.quantity * &v.price)
                    .with_scale_round(i64::from(quote_precision), bigdecimal::RoundingMode::Down)
            })
            .filter(|v| crate::numeric::ensure_amount_storage(v, "持仓估值").is_ok());
        let profit = market_value.as_ref().map(|v| v - &h.cost);
        let eligible = h.status == "held" && store::eligible(&h.policy_json, h.unlock_at, &p, now);
        let unlock_at = match h.policy_json.lock {
            crate::modules::new_coin::domain::issuance::HoldingLock::OnListing => {
                p.actual_listed_at
            }
            crate::modules::new_coin::domain::issuance::HoldingLock::OnPurchase => {
                p.actual_purchase_at
            }
            _ => h.unlock_at,
        };
        result.push(json!({"holding":h,"symbol":p.symbol,"quote_symbol":quote_symbol,"pair_id":p.post_listing_pair_id,"pair_symbol":pair,"lifecycle_status":p.lifecycle_status,"eligible":eligible,"unlock_at":unlock_at.map(|v|v.timestamp_millis()),"market_price":price.as_ref().map(|v|&v.price),"market_value":market_value,"profit":profit}));
    }
    Ok(
        json!({"holdings":result,"checked_at":now.timestamp_millis(),"next_before_id":next_before_id}),
    )
}

/// 自动节点使用相同锁与真实事件事务；持久化退避保证进程重启和多个实例下不重复、不跳级。
pub async fn advance_due(mysql: &Pool<MySql>) -> AppResult<u32> {
    let rows = store::due_projects(mysql).await?;
    let mut advanced = 0;
    for hint in rows {
        let result = async {
            let mut guard = PairGenerationLock::acquire(mysql, hint.pair()?, 0)
                .await?
                .ok_or_else(|| AppError::Conflict("行情正在切换".into()))?;
            let mut tx = mysql.begin().await?;
            let p = store::lock_project(&mut tx, hint.id).await?;
            if p.lifecycle_status != hint.lifecycle_status {
                return Ok(false);
            }
            let target = p
                .policy()?
                .next(&p.lifecycle_status)
                .map(|(s, _)| s)
                .ok_or(AppError::NotFound)?;
            guard.ensure_owned().await?;
            store::advance(
                &mut tx,
                &p,
                target,
                ("scheduler", None),
                "自动阶段节点",
                true,
            )
            .await?;
            guard.ensure_owned().await?;
            tx.commit().await?;
            Ok::<_, AppError>(true)
        }
        .await;
        match result {
            Ok(true) => advanced += 1,
            Ok(false) => {}
            Err(_) => store::defer_transition(mysql, hint.id, &hint.lifecycle_status).await?,
        }
    }
    Ok(advanced)
}
