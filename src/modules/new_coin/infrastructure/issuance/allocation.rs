//! 选中申购批次的最终派发；复用原始冻结义务结算，新币入专属持仓，绝不生成旧版自动解锁凭证。
use super::*;
use crate::modules::admin::infrastructure::settle_manual_new_coin_subscription_in_tx;
use crate::modules::new_coin::domain::issuance::allocation_quantity;

/// 批次预读只用于构建锁集合；执行时必须锁内重验订单、用户与代理归属。
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct OrderHint {
    pub id: u64,
    pub user_id: u64,
    pub project_id: u64,
    pub requested_quantity: BigDecimal,
    pub created_at: DateTime<Utc>,
}

/// 只查明确选中的有界主键，不按发行人或整个团队隐式扩展批次。
pub(crate) async fn hints(pool: &Pool<MySql>, ids: &[u64]) -> AppResult<Vec<OrderHint>> {
    let mut q = sqlx::QueryBuilder::new(
        "SELECT id,user_id,project_id,requested_quantity,created_at FROM new_coin_subscriptions WHERE id IN (",
    );
    let mut args = q.separated(",");
    for id in ids {
        args.push_bind(id);
    }
    args.push_unseparated(") ORDER BY id");
    let rows = q.build_query_as().fetch_all(pool).await?;
    Ok(rows)
}

/// 门户锁后读取不可变批次收据；同键不同百分比、理由或选择集合一律冲突。
pub(crate) async fn replay(
    tx: &mut Transaction<'_, MySql>,
    actor: u64,
    key: &str,
    intent: &Value,
) -> AppResult<Option<Value>> {
    let row:Option<(Json<Value>,Json<Value>)>=sqlx::query_as("SELECT intent_json,response_json FROM new_coin_allocation_batches WHERE agent_admin_id=? AND idempotency_key=?")
        .bind(actor).bind(key.as_bytes()).fetch_optional(&mut **tx).await?;
    match row {
        Some((stored, result)) if stored.0 == *intent => Ok(Some(result.0)),
        Some(_) => Err(AppError::Conflict("该批次标识已用于不同派发请求".into())),
        None => Ok(None),
    }
}

/// 同事务提交一批最终结果；唯一键和门户锁阻止网络重试产生重复退款或派发。
pub(crate) async fn receipt(
    tx: &mut Transaction<'_, MySql>,
    actor: u64,
    key: &str,
    intent: &Value,
    response: &Value,
) -> AppResult<()> {
    sqlx::query("INSERT INTO new_coin_allocation_batches(agent_admin_id,idempotency_key,intent_json,response_json) VALUES (?,?,?,?)")
        .bind(actor).bind(key.as_bytes()).bind(Json(intent)).bind(Json(response)).execute(&mut **tx).await?;
    Ok(())
}

/// 发放数量只增加 locked 桶与独立批次；所有规则和成本按购买当时固化，到期也必须通过原子卖出退出。
pub(crate) async fn holding(
    tx: &mut Transaction<'_, MySql>,
    project: &Project,
    user: u64,
    source: (&str, u64),
    quantity: &BigDecimal,
    cost: &BigDecimal,
    acquired_at: DateTime<Utc>,
) -> AppResult<u64> {
    let policy = project.policy()?;
    let unlock_at = policy.unlock_at(acquired_at)?;
    let wallet = lock_or_create_wallet_row(tx, user, project.asset_id).await?;
    let locked = &wallet.locked + quantity;
    crate::numeric::ensure_amount_storage(&locked, "新币持仓余额")?;
    let id=sqlx::query("INSERT INTO new_coin_holdings(project_id,user_id,source_kind,source_id,quantity,cost,policy_json,acquired_at,unlock_at) VALUES (?,?,?,?,?,?,?,?,?)")
        .bind(project.id).bind(user).bind(source.0).bind(source.1).bind(quantity).bind(cost).bind(Json(policy)).bind(acquired_at).bind(unlock_at)
        .execute(&mut **tx).await?.last_insert_id();
    sqlx::query("UPDATE wallet_accounts SET locked=? WHERE user_id=? AND asset_id=?")
        .bind(&locked)
        .bind(user)
        .bind(project.asset_id)
        .execute(&mut **tx)
        .await?;
    insert_new_coin_wallet_ledger(
        tx,
        user,
        project.asset_id,
        quantity.clone(),
        "locked",
        &locked,
        &wallet.available,
        &wallet.frozen,
        &locked,
        "new_coin_holding_credit",
        "new_coin_holding",
        &id.to_string(),
    )
    .await?;
    trading::journal(
        tx,
        project.asset_id,
        &format!("holding:{id}"),
        &id.to_string(),
        &[
            ("user_token_liability", -quantity.clone()),
            ("platform_token_issuance", quantity.clone()),
        ],
        user,
    )
    .await?;
    Ok(id)
}

/// 同一事务核销原申购冻结款、退款差额和剩余供给；原购买时间用于周期锁，不使用派发按钮点击时间。
pub(crate) async fn allocate(
    tx: &mut Transaction<'_, MySql>,
    p: &Project,
    order: &OrderHint,
    percent: &BigDecimal,
    key: &str,
    exact_quantity: Option<&BigDecimal>,
) -> AppResult<Value> {
    if p.lifecycle_status != "distribution" || p.status != "active" {
        return Err(AppError::Conflict("项目尚未进入派发阶段".into()));
    }
    p.policy()?;
    let quote = p.quote()?;
    let mut assets = vec![p.asset_id, quote];
    assets.sort_unstable();
    let precisions = lock_active_asset_precisions_in_order(tx, &assets).await?;
    let base_precision = locked_asset_precision(&precisions, p.asset_id)?;
    let quote_precision = locked_asset_precision(&precisions, quote)?;
    lock_new_coin_order_wallets_in_order(tx, order.user_id, p.asset_id, quote).await?;
    let allocated = match exact_quantity {
        Some(value) => {
            ensure_new_coin_amount_precision(value, base_precision, "派发数量")?;
            value.clone()
        }
        None => allocation_quantity(&order.requested_quantity, percent, base_precision)?,
    };
    let cost = settle_manual_new_coin_subscription_in_tx(
        tx,
        order.id,
        p.id,
        order.user_id,
        quote,
        &allocated,
        quote_precision,
    )
    .await?
    .ok_or_else(|| AppError::Conflict("历史已付款申购不能重新派发".into()))?;
    let holding_id = if allocated > 0 {
        let id = holding(
            tx,
            p,
            order.user_id,
            ("subscription", order.id),
            &allocated,
            &cost,
            order.created_at,
        )
        .await?;
        finalize_new_coin_supply_in_tx(tx, p.id, &allocated).await?;
        let treasury = trading::treasury(tx).await?;
        credit_balance(
            tx,
            treasury,
            quote,
            &cost,
            NewCoinLedgerMetadata {
                change_type: "new_coin_issuance_proceeds",
                ref_type: "new_coin_subscription",
                ref_id: &order.id.to_string(),
            },
        )
        .await?;
        trading::journal(
            tx,
            quote,
            &format!("allocation:{}", order.id),
            &order.id.to_string(),
            &[
                ("user_quote_liability", cost.clone()),
                ("platform_buyback_inventory", -cost.clone()),
            ],
            order.user_id,
        )
        .await?;
        Some(id)
    } else {
        None
    };
    let status = if allocated == 0 { "refunded" } else { "held" };
    sqlx::query("INSERT INTO new_coin_distributions(project_id,user_id,subscription_id,asset_id,quantity,status,idempotency_key) VALUES (?,?,?,?,?,?,?)")
        .bind(p.id).bind(order.user_id).bind(order.id).bind(p.asset_id).bind(&allocated).bind(status).bind(key).execute(&mut **tx).await?;
    let refund: BigDecimal =
        sqlx::query_scalar("SELECT refunded_quote_amount FROM new_coin_subscriptions WHERE id=?")
            .bind(order.id)
            .fetch_one(&mut **tx)
            .await?;
    Ok(
        json!({"subscription_id":order.id,"project_id":p.id,"user_id":order.user_id,"percent":percent,"quantity":allocated,"settled_quote_amount":cost,"refunded_quote_amount":refund,"holding_id":holding_id,"status":status}),
    )
}

/// 管理员已锁项目后读取指定来源；用户和项目仍须由用例匹配，不允许按客户端金额编造订单。
pub(crate) async fn hint_in_tx(tx: &mut Transaction<'_, MySql>, id: u64) -> AppResult<OrderHint> {
    sqlx::query_as("SELECT id,user_id,project_id,requested_quantity,created_at FROM new_coin_subscriptions WHERE id=?")
        .bind(id).fetch_optional(&mut **tx).await?.ok_or(AppError::NotFound)
}
