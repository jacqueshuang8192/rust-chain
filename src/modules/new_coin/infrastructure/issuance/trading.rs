//! 新币持仓与平台回购：报价和成交凭证分离，用户/平台钱包、持仓、账本在同一事务核销。
use super::*;
use crate::modules::market::infrastructure::cache::latest::{LatestPrice, load_latest_price};
use crate::modules::new_coin::domain::issuance::exit_amounts;
use crate::modules::new_coin::presentation::issuance::{TradeQuote, TradeQuoteRequest};
use bigdecimal::RoundingMode;
use redis::aio::ConnectionManager;
use sha2::{Digest, Sha256};

/// 每一持仓只对应一个不可变购买来源；卖出整个批次，不能与旧自动释放或通用可用余额重复消费。
#[derive(Debug, sqlx::FromRow, Serialize)]
pub(crate) struct Holding {
    pub id: u64,
    pub user_id: u64,
    pub project_id: u64,
    pub source_kind: String,
    pub source_id: u64,
    pub quantity: BigDecimal,
    pub cost: BigDecimal,
    pub policy_json: Json<IssuancePolicy>,
    pub status: String,
    #[serde(with = "crate::time::unix_millis")]
    pub acquired_at: DateTime<Utc>,
    #[serde(with = "crate::time::option_unix_millis")]
    pub unlock_at: Option<DateTime<Utc>>,
    #[serde(with = "crate::time::option_unix_millis")]
    pub sold_at: Option<DateTime<Utc>>,
}

/// 查平台预充值的现货资金账户；审批创建身份，交易不能伪造余额或自动补充资金。
pub(crate) async fn treasury(tx: &mut Transaction<'_, MySql>) -> AppResult<u64> {
    sqlx::query_scalar("SELECT id FROM users WHERE email=?")
        .bind(crate::modules::spot::infrastructure::SYSTEM_SPOT_LIQUIDITY_EMAIL)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| AppError::Conflict("平台结算账户未配置".into()))
}

/// 返回结算币种标签；缺失的资产必须报错而不是默认显示 USDT。
pub(crate) async fn quote_symbol(tx: &mut Transaction<'_, MySql>, asset: u64) -> AppResult<String> {
    sqlx::query_scalar("SELECT symbol FROM assets WHERE id=?")
        .bind(asset)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)
}

/// 资金分录逐资产独立平衡，零腿不入库；唯一业务键阻止重复结算，失败由外层回滚钱包。
pub(crate) async fn journal(
    tx: &mut Transaction<'_, MySql>,
    asset: u64,
    key: &str,
    reference: &str,
    legs: &[(&str, BigDecimal)],
    user: u64,
) -> AppResult<()> {
    if legs
        .iter()
        .fold(BigDecimal::from(0), |sum, (_, amount)| sum + amount)
        != 0
    {
        return Err(AppError::Internal("新币分录不平衡".into()));
    }
    for (account, amount) in legs {
        if amount == &BigDecimal::from(0) {
            continue;
        }
        crate::numeric::ensure_amount_storage(amount, "新币分录")?;
        sqlx::query("INSERT INTO platform_financial_journal(transaction_key,context,account_code,asset_id,amount,ref_type,ref_id,metadata_json) VALUES (?,'new_coin_issuance',?,?,?,'new_coin_trade',?,?)")
            .bind(format!("new_coin:{key}")).bind(account).bind(asset).bind(amount).bind(reference).bind(Json(json!({"user_id":user}))).execute(&mut **tx).await?;
    }
    Ok(())
}

/// 在项目后锁两种资产，再以用户/资产稳定排序锁全部钱包；不依赖余额足够掩盖反向锁序。
pub(crate) async fn lock_money(
    tx: &mut Transaction<'_, MySql>,
    p: &Project,
    users: &[u64],
) -> AppResult<(i32, i32)> {
    let quote = p.quote()?;
    let mut assets = vec![p.asset_id, quote];
    assets.sort_unstable();
    let precisions = lock_active_asset_precisions_in_order(tx, &assets).await?;
    let mut users = users.to_vec();
    users.sort_unstable();
    users.dedup();
    for user in users {
        for asset in &assets {
            lock_or_create_wallet_row(tx, user, *asset).await?;
        }
    }
    Ok((
        locked_asset_precision(&precisions, p.asset_id)?,
        locked_asset_precision(&precisions, quote)?,
    ))
}

/// 跨项目批次先取得全体资产与钱包的全局顺序锁，避免两个项目共享计价资产时反向取锁。
pub(crate) async fn lock_batch_money(
    tx: &mut Transaction<'_, MySql>,
    projects: &[&Project],
    users: &[u64],
) -> AppResult<()> {
    let mut assets = std::collections::BTreeSet::new();
    for p in projects {
        assets.insert(p.asset_id);
        assets.insert(p.quote()?);
    }
    let assets = assets.into_iter().collect::<Vec<_>>();
    lock_active_asset_precisions_in_order(tx, &assets).await?;
    let users = users
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    for user in users {
        for asset in &assets {
            lock_or_create_wallet_row(tx, user, *asset).await?;
        }
    }
    Ok(())
}

/// 行情交易对与发行资产必须完全匹配，平台必须有有效的生成器；上市只开放现货，不替换交易对。
pub(crate) async fn pair_symbol(tx: &mut Transaction<'_, MySql>, p: &Project) -> AppResult<String> {
    let row:Option<(String,u64,u64,String,String)>=sqlx::query_as("SELECT symbol,base_asset,quote_asset,status,market_type FROM trading_pairs WHERE id=? FOR SHARE")
        .bind(p.pair()?).fetch_optional(&mut **tx).await?;
    match row {
        Some((symbol, base, quote, status, kind))
            if base == p.asset_id
                && quote == p.quote()?
                && status == "active"
                && matches!(kind.as_str(), "internal" | "strategy") =>
        {
            Ok(symbol)
        }
        _ => Err(AppError::Conflict("项目行情交易对不可用".into())),
    }
}

/// 只接收平台引擎最新正价格；陈旧、未来、未知或外部来源均拒绝，绝不使用发行价兜底。
pub(crate) async fn price(
    redis: Option<&ConnectionManager>,
    symbol: &str,
    now: DateTime<Utc>,
) -> AppResult<LatestPrice> {
    let redis = redis.ok_or_else(|| AppError::Conflict("行情暂不可用".into()))?;
    let value = load_latest_price(redis, symbol)
        .await?
        .ok_or_else(|| AppError::Conflict("等待首个有效行情".into()))?;
    let age = now
        .signed_duration_since(value.observed_at)
        .num_milliseconds();
    if !(0..=60_000).contains(&age) || !matches!(value.source.as_str(), "default" | "strategy") {
        return Err(AppError::Conflict("行情已过期或来源无效".into()));
    }
    Ok(value)
}

/// 报价及确认均复核当前生成器未暂停；失去租约的旧写入者不能借用仍在缓存里的价格成交。
pub(crate) async fn engine_ready(
    tx: &mut Transaction<'_, MySql>,
    p: &Project,
    source: &str,
) -> AppResult<String> {
    #[derive(sqlx::FromRow)]
    struct Engine {
        enabled: bool,
        all_market_paused: bool,
        active_source: String,
        lease_expires_at: Option<DateTime<Utc>>,
        generation: u64,
        version: u32,
        default_version: Option<u32>,
        strategy_id: Option<u64>,
        strategy_version: Option<i32>,
    }
    let live: Option<Engine> = sqlx::query_as("SELECT g.enabled,g.all_market_paused,g.version,r.active_source,r.lease_expires_at,r.generation,r.default_version,r.strategy_id,r.strategy_version FROM market_default_generators g JOIN market_pair_generation_runs r ON r.pair_id=g.pair_id WHERE g.pair_id=? FOR SHARE")
        .bind(p.pair()?).fetch_optional(&mut **tx).await?;
    let now = now(tx).await?;
    let live = live
        .filter(|r| {
            !r.all_market_paused
                && r.active_source == source
                && r.lease_expires_at.is_some_and(|v| v > now)
        })
        .ok_or_else(|| AppError::Conflict("平台行情引擎尚未就绪或已暂停".into()))?;
    if source == "default" && (!live.enabled || live.default_version != Some(live.version)) {
        return Err(AppError::Conflict("行情生成器版本尚未就绪".into()));
    }
    if source == "strategy" {
        let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM market_strategies s JOIN strategy_runs r ON r.strategy_id=s.id WHERE s.id=? AND s.pair_id=? AND s.status='active' AND s.start_time<=UTC_TIMESTAMP(6) AND s.end_time>UTC_TIMESTAMP(6) AND r.active_version=? AND r.run_status IN ('live','running')")
            .bind(live.strategy_id).bind(p.pair()?).bind(live.strategy_version).fetch_one(&mut **tx).await?;
        if active != 1 {
            return Err(AppError::Conflict("行情策略已变化".into()));
        }
    }
    let identity = format!(
        "{}:{}:{}:{}",
        live.generation,
        live.version,
        live.strategy_id.unwrap_or(0),
        live.strategy_version.unwrap_or(0)
    );
    Ok(hex::encode(Sha256::digest(identity.as_bytes()))[..32].to_owned())
}

/// 按认证用户获取持仓并锁定；即使知道主键也不能读写其他用户的持仓。
pub(crate) async fn lock_holding(
    tx: &mut Transaction<'_, MySql>,
    user: u64,
    id: u64,
) -> AppResult<Holding> {
    sqlx::query_as("SELECT * FROM new_coin_holdings WHERE id=? AND user_id=? FOR UPDATE")
        .bind(id)
        .bind(user)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)
}

/// 最近持仓按认证用户隔离；旧版锁仓仍由原接口读取，不推测历史批次成本。
pub(crate) async fn holdings(
    pool: &Pool<MySql>,
    user: u64,
    project: Option<u64>,
    before: Option<u64>,
) -> AppResult<Vec<Holding>> {
    let mut query = sqlx::QueryBuilder::new("SELECT * FROM new_coin_holdings WHERE user_id=");
    query.push_bind(user);
    if let Some(project) = project {
        query.push(" AND project_id=").push_bind(project);
    }
    if let Some(before) = before {
        query.push(" AND id<").push_bind(before);
    }
    query.push(" ORDER BY id DESC LIMIT 101");
    Ok(query.build_query_as().fetch_all(pool).await?)
}

/// 用户行串行化跨项目报价幂等键；仅 active 用户可建立新的资金意图。
pub(crate) async fn lock_user(tx: &mut Transaction<'_, MySql>, user: u64) -> AppResult<()> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM users WHERE id=? FOR UPDATE")
            .bind(user)
            .fetch_optional(&mut **tx)
            .await?;
    if status.as_deref() != Some("active") {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

/// 重放返回原始报价，包括过期报价；刷新必须创建新意图，不能悄悄替换用户已确认的价格。
pub(crate) async fn quote_replay(
    tx: &mut Transaction<'_, MySql>,
    user: u64,
    key: &str,
    intent: &Value,
) -> AppResult<Option<TradeQuote>> {
    let row: Option<TradeQuote> =
        sqlx::query_as("SELECT * FROM new_coin_trade_quotes WHERE user_id=? AND idempotency_key=?")
            .bind(user)
            .bind(key.as_bytes())
            .fetch_optional(&mut **tx)
            .await?;
    if row.as_ref().is_some_and(|r| r.intent_json.0 != *intent) {
        return Err(AppError::Conflict("该报价标识已用于其他交易".into()));
    }
    Ok(row)
}

/// 持久化服务端有时限报价与来源证据；仅报价不冻结资金、不卖币、不收取费用。
pub(crate) async fn save_quote(
    tx: &mut Transaction<'_, MySql>,
    p: &Project,
    user: u64,
    req: &TradeQuoteRequest,
    quote: &LatestPrice,
    intent: Value,
    (base_precision, quote_precision): (i32, i32),
) -> AppResult<TradeQuote> {
    let now = now(tx).await?;
    let (quantity, cost, fee_rule) = if req.side == "buy" {
        if p.lifecycle_status != "purchase" || req.holding_id.is_some() {
            return Err(AppError::Conflict("认购已结束，请前往现货交易".into()));
        }
        (
            req.quantity
                .clone()
                .filter(|q| q > &BigDecimal::from(0))
                .ok_or_else(|| AppError::Validation("请输入正的认购数量".into()))?,
            BigDecimal::from(0),
            None,
        )
    } else {
        if req.quantity.is_some() {
            return Err(AppError::Validation("卖出按整个持仓批次结算".into()));
        }
        let h = lock_holding(
            tx,
            user,
            req.holding_id
                .ok_or_else(|| AppError::Validation("请选择持仓".into()))?,
        )
        .await?;
        if h.project_id != p.id
            || h.status != "held"
            || !eligible(&h.policy_json, h.unlock_at, p, now)
        {
            return Err(AppError::Conflict("持仓尚未解禁或已卖出".into()));
        }
        (h.quantity, h.cost, Some(h.policy_json.fee.clone()))
    };
    ensure_new_coin_amount_precision(&quantity, base_precision, "认购数量")?;
    let gross =
        (&quantity * &quote.price).with_scale_round(i64::from(quote_precision), RoundingMode::Down);
    crate::numeric::ensure_amount_storage(&gross, "成交金额")?;
    if gross <= 0 {
        return Err(AppError::Validation("成交金额低于最小单位".into()));
    }
    let (fee, net) = if let Some(fee_rule) = fee_rule {
        exit_amounts(&fee_rule, &gross, &cost, quote_precision)?
    } else {
        (BigDecimal::from(0), gross.clone())
    };
    let id = uuid::Uuid::now_v7().to_string();
    let expires_at = now + chrono::Duration::seconds(15);
    sqlx::query("INSERT INTO new_coin_trade_quotes(id,user_id,project_id,holding_id,side,quantity,price,gross,cost,fee,net,source,source_version,observed_at,expires_at,idempotency_key,intent_json) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(&id).bind(user).bind(p.id).bind(req.holding_id).bind(&req.side).bind(&quantity).bind(&quote.price).bind(&gross).bind(&cost).bind(&fee).bind(&net)
        .bind(&quote.source).bind(&quote.source_version).bind(quote.observed_at).bind(expires_at).bind(req.idempotency_key.as_bytes()).bind(Json(intent)).execute(&mut **tx).await?;
    lock_quote(tx, user, &id).await
}

/// 使用用户和报价主键共同锁定一次性确认凭证；终态重放不受后续行情或时钟变化影响。
pub(crate) async fn lock_quote(
    tx: &mut Transaction<'_, MySql>,
    user: u64,
    id: &str,
) -> AppResult<TradeQuote> {
    sqlx::query_as("SELECT * FROM new_coin_trade_quotes WHERE id=? AND user_id=? FOR UPDATE")
        .bind(id)
        .bind(user)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)
}

/// 事务外预读只决定项目锁和钱包集合；同事务的锁定读取才决定是否成交。
pub(crate) async fn quote_hint(pool: &Pool<MySql>, user: u64, id: &str) -> AppResult<TradeQuote> {
    sqlx::query_as("SELECT * FROM new_coin_trade_quotes WHERE id=? AND user_id=?")
        .bind(id)
        .bind(user)
        .fetch_optional(pool)
        .await?
        .ok_or(AppError::NotFound)
}

/// 已锁项目、资产、双方钱包及报价后执行一次成交；回购只能花费平台已备资金，原子扣除手续费。
pub(crate) async fn execute(
    tx: &mut Transaction<'_, MySql>,
    p: &Project,
    q: &TradeQuote,
    treasury: u64,
) -> AppResult<Value> {
    let now = now(tx).await?;
    if q.expires_at < now
        || now.signed_duration_since(q.observed_at).num_seconds() > 60
        || q.observed_at > now
    {
        return Err(AppError::Conflict("报价已过期，请重新获取".into()));
    }
    if p.status != "active" || !matches!(p.lifecycle_status.as_str(), "purchase" | "listed") {
        return Err(AppError::Conflict("项目暂不支持交易".into()));
    }
    let quote = p.quote()?;
    let ledger = NewCoinLedgerMetadata {
        change_type: "new_coin_trade",
        ref_type: "new_coin_quote",
        ref_id: &q.id,
    };
    let holding_id = if q.side == "buy" {
        if p.lifecycle_status != "purchase" {
            return Err(AppError::Conflict("认购已结束，请前往现货交易".into()));
        }
        reserve_new_coin_supply_in_tx(tx, p.id, &q.quantity).await?;
        debit_wallet_available(tx, q.user_id, quote, &q.gross, ledger).await?;
        credit_balance(
            tx,
            treasury,
            quote,
            &q.gross,
            NewCoinLedgerMetadata {
                change_type: "new_coin_issuance_proceeds",
                ref_type: "new_coin_quote",
                ref_id: &q.id,
            },
        )
        .await?;
        let order=sqlx::query("INSERT INTO new_coin_purchase_orders(project_id,user_id,pair_id,base_asset,quote_asset,price,quantity,quote_amount,status,idempotency_key) VALUES (?,?,?,?,?,?,?,?,'held',?)")
            .bind(p.id).bind(q.user_id).bind(p.pair()?).bind(p.asset_id).bind(quote).bind(&q.price).bind(&q.quantity).bind(&q.gross).bind(format!("quote:{}",q.id)).execute(&mut **tx).await?.last_insert_id();
        let id = allocation::holding(
            tx,
            p,
            q.user_id,
            ("purchase", order),
            &q.quantity,
            &q.gross,
            now,
        )
        .await?;
        finalize_new_coin_supply_in_tx(tx, p.id, &q.quantity).await?;
        journal(
            tx,
            quote,
            &q.id,
            &q.id,
            &[
                ("user_quote_liability", q.gross.clone()),
                ("platform_buyback_inventory", -q.gross.clone()),
            ],
            q.user_id,
        )
        .await?;
        id
    } else {
        let h = lock_holding(tx, q.user_id, q.holding_id.ok_or(AppError::NotFound)?).await?;
        if h.project_id != p.id
            || h.status != "held"
            || h.quantity != q.quantity
            || h.cost != q.cost
            || !eligible(&h.policy_json, h.unlock_at, p, now)
        {
            return Err(AppError::Conflict("持仓已变化或尚未解禁".into()));
        }
        let wallet = lock_wallet_row(tx, q.user_id, p.asset_id).await?;
        if wallet.locked < q.quantity {
            return Err(AppError::Conflict("锁仓余额不足".into()));
        }
        // 平台只支付扣费后净额，费用记收入；绝不为满足回购凭空补充钱包。
        if q.net > 0 {
            debit_wallet_available(
                tx,
                treasury,
                quote,
                &q.net,
                NewCoinLedgerMetadata {
                    change_type: "new_coin_buyback",
                    ref_type: "new_coin_quote",
                    ref_id: &q.id,
                },
            )
            .await?;
            credit_balance(tx, q.user_id, quote, &q.net, ledger).await?;
        }
        let locked = &wallet.locked - &q.quantity;
        sqlx::query("UPDATE wallet_accounts SET locked=? WHERE user_id=? AND asset_id=?")
            .bind(&locked)
            .bind(q.user_id)
            .bind(p.asset_id)
            .execute(&mut **tx)
            .await?;
        insert_new_coin_wallet_ledger(
            tx,
            q.user_id,
            p.asset_id,
            -q.quantity.clone(),
            "locked",
            &locked,
            &wallet.available,
            &wallet.frozen,
            &locked,
            "new_coin_buyback",
            "new_coin_quote",
            &q.id,
        )
        .await?;
        credit_balance(
            tx,
            treasury,
            p.asset_id,
            &q.quantity,
            NewCoinLedgerMetadata {
                change_type: "new_coin_buyback_inventory",
                ref_type: "new_coin_quote",
                ref_id: &q.id,
            },
        )
        .await?;
        sqlx::query(
            "UPDATE new_coin_holdings SET status='sold',sold_at=? WHERE id=? AND status='held'",
        )
        .bind(now)
        .bind(h.id)
        .execute(&mut **tx)
        .await?;
        journal(
            tx,
            p.asset_id,
            &q.id,
            &q.id,
            &[
                ("user_token_liability", q.quantity.clone()),
                ("platform_token_inventory", -q.quantity.clone()),
            ],
            q.user_id,
        )
        .await?;
        journal(
            tx,
            quote,
            &q.id,
            &q.id,
            &[
                ("user_quote_liability", -q.net.clone()),
                ("platform_buyback_expense", q.gross.clone()),
                ("platform_exit_fee_revenue", -q.fee.clone()),
            ],
            q.user_id,
        )
        .await?;
        h.id
    };
    let receipt = json!({"quote_id":q.id,"project_id":p.id,"side":q.side,"quantity":q.quantity,"price":q.price,"gross":q.gross,"fee":q.fee,"net":q.net,"holding_id":holding_id,"executed_at":now.timestamp_millis()});
    sqlx::query("UPDATE new_coin_trade_quotes SET status='executed',executed_at=?,receipt_json=? WHERE id=? AND status='quoted'").bind(now).bind(Json(&receipt)).bind(&q.id).execute(&mut **tx).await?;
    operation(
        tx,
        p.id,
        ("user", Some(q.user_id)),
        "new_coin.trade",
        receipt.clone(),
        "用户确认报价成交",
    )
    .await?;
    Ok(receipt)
}

/// 上市时把尚未售出的供给交给同一交易对的平台库存；已售持仓继续原规则，不重播价格或 K 线。
pub(crate) async fn handoff_inventory(
    tx: &mut Transaction<'_, MySql>,
    p: &Project,
) -> AppResult<()> {
    let available: BigDecimal =
        sqlx::query_scalar("SELECT remaining_supply FROM new_coin_projects WHERE id=?")
            .bind(p.id)
            .fetch_one(&mut **tx)
            .await?;
    if available <= 0 {
        return Ok(());
    }
    let treasury = treasury(tx).await?;
    lock_money(tx, p, &[treasury]).await?;
    let reference = format!("listing:{}", p.id);
    credit_balance(
        tx,
        treasury,
        p.asset_id,
        &available,
        NewCoinLedgerMetadata {
            change_type: "new_coin_spot_inventory",
            ref_type: "new_coin_project",
            ref_id: &p.id.to_string(),
        },
    )
    .await?;
    sqlx::query("UPDATE new_coin_projects SET allocated_supply=allocated_supply+remaining_supply,remaining_supply=0 WHERE id=?").bind(p.id).execute(&mut **tx).await?;
    journal(
        tx,
        p.asset_id,
        &reference,
        &p.id.to_string(),
        &[
            ("platform_spot_inventory", -available.clone()),
            ("platform_token_issuance", available),
        ],
        treasury,
    )
    .await
}

/// 只读估值使用计价资产精度；不能把未量化的乘法结果传给严格金额客户端。
pub(crate) async fn quote_precision(tx: &mut Transaction<'_, MySql>, asset: u64) -> AppResult<i32> {
    sqlx::query_scalar("SELECT precision_scale FROM assets WHERE id=?")
        .bind(asset)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)
}
