//! 新版发行管理的凭证和快照查询；列表与计数共用谓词及事务，不触发任何资金结算。
use crate::{
    error::{AppError, AppResult},
    modules::admin::presentation::token_issuance::*,
};
use serde_json::Value;
use sqlx::{MySql, Pool, QueryBuilder, Transaction, types::Json};

/// 管理员行锁串行化本人创建键；先核对活动身份，再读取不可变意图，变更输入不能复用原键。
pub(crate) async fn creation_replay(
    tx: &mut Transaction<'_, MySql>,
    admin: u64,
    key: &str,
    intent: &Value,
) -> AppResult<Option<u64>> {
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM admin_users WHERE id=? FOR UPDATE")
            .bind(admin)
            .fetch_optional(&mut **tx)
            .await?;
    if status.as_deref() != Some("active") {
        return Err(AppError::Forbidden);
    }
    let receipt: Option<(Json<Value>, u64)> = sqlx::query_as("SELECT intent_json,project_id FROM admin_token_issuance_receipts WHERE admin_id=? AND idempotency_key=?")
        .bind(admin).bind(key.as_bytes()).fetch_optional(&mut **tx).await?;
    match receipt {
        Some((original, id)) if original.0 == *intent => Ok(Some(id)),
        Some(_) => Err(AppError::Conflict(
            "发行请求标识已绑定其他配置，请核对原项目".into(),
        )),
        None => Ok(None),
    }
}

/// 发行凭证与项目、钱包及审计同事务落库；失败时外层回滚全部创建结果。
pub(crate) async fn creation_receipt(
    tx: &mut Transaction<'_, MySql>,
    admin: u64,
    key: &str,
    intent: &Value,
    project: u64,
) -> AppResult<()> {
    sqlx::query("INSERT INTO admin_token_issuance_receipts(admin_id,idempotency_key,intent_json,project_id) VALUES (?,?,?,?)")
        .bind(admin).bind(key.as_bytes()).bind(Json(intent)).bind(project).execute(&mut **tx).await?;
    Ok(())
}

fn record_query(
    table: &str,
    fields: &str,
    filter: &AdminTokenRecordsQuery,
) -> QueryBuilder<'static, MySql> {
    let mut q = QueryBuilder::new(format!(
        "SELECT {fields} FROM {table} r JOIN new_coin_projects p ON p.id=r.project_id JOIN assets quote ON quote.id=p.quote_asset_id JOIN users u ON u.id=r.user_id LEFT JOIN user_referrals ref ON ref.user_id=u.id LEFT JOIN agents owner ON owner.id=ref.root_agent_id WHERE 1=1"
    ));
    for (column, value) in [
        ("r.project_id", filter.project_id),
        ("r.user_id", filter.user_id),
        ("owner.id", filter.agent_id),
        ("p.issuer_agent_id", filter.issuer_agent_id),
    ] {
        if let Some(value) = value {
            q.push(format!(" AND {column}=")).push_bind(value);
        }
    }
    if let Some(value) = filter.email.as_ref().filter(|s| !s.trim().is_empty()) {
        q.push(" AND u.email LIKE ")
            .push_bind(format!("%{}%", value.trim()));
    }
    if let Some(value) = filter.status.as_ref().filter(|s| !s.trim().is_empty()) {
        q.push(" AND r.status=").push_bind(value.trim().to_owned());
    }
    q
}

const IDENTITIES: &str = "r.id,r.project_id,p.symbol,quote.symbol AS quote_symbol,r.user_id,u.email,owner.id AS owner_agent_id,owner.agent_code AS owner_agent_code,p.issuer_agent_id";

/// 按相同事务快照读取持仓及总数；阶段锁只使用实际事件，保留原合同和原购买时点。
pub(crate) async fn holdings(
    pool: &Pool<MySql>,
    filter: &AdminTokenRecordsQuery,
    limit: u32,
    offset: u32,
) -> AppResult<(Vec<AdminTokenHolding>, i64)> {
    let fields = format!(
        "{IDENTITIES},r.source_kind,r.source_id,r.quantity,r.cost,r.policy_json,r.status,r.acquired_at,r.sold_at,CASE JSON_UNQUOTE(JSON_EXTRACT(r.policy_json,'$.lock.kind')) WHEN 'on_purchase' THEN p.actual_purchase_at WHEN 'on_listing' THEN p.actual_listed_at ELSE r.unlock_at END AS unlock_at"
    );
    let mut rows = record_query("new_coin_holdings", &fields, filter);
    let mut count = record_query("new_coin_holdings", "COUNT(*)", filter);
    rows.push(" ORDER BY r.id DESC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let mut tx = pool.begin().await?;
    let total = count.build_query_scalar().fetch_one(&mut *tx).await?;
    let rows = rows.build_query_as().fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok((rows, total))
}

/// 只列出已提交的卖出成交，报价草稿和买入不能被计作回购；行、计数和币种单位一致。
pub(crate) async fn buybacks(
    pool: &Pool<MySql>,
    filter: &AdminTokenRecordsQuery,
    limit: u32,
    offset: u32,
) -> AppResult<(Vec<AdminTokenBuyback>, i64)> {
    let fields = format!(
        "{IDENTITIES},r.holding_id,r.quantity,r.price,r.cost,r.gross,r.fee,r.net,r.source,r.source_version,r.status,r.observed_at,r.executed_at"
    );
    let mut rows = record_query("new_coin_trade_quotes", &fields, filter);
    let mut count = record_query("new_coin_trade_quotes", "COUNT(*)", filter);
    for q in [&mut rows, &mut count] {
        q.push(" AND r.side='sell' AND r.status='executed'");
    }
    rows.push(" ORDER BY r.executed_at DESC,r.id DESC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let mut tx = pool.begin().await?;
    let total = count.build_query_scalar().fetch_one(&mut *tx).await?;
    let rows = rows.build_query_as().fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok((rows, total))
}

/// 审计查询保留管理员、代理、调度器和用户的实际操作身份；所有筛选同时作用于列表和总数。
pub(crate) async fn operations(
    pool: &Pool<MySql>,
    filter: &AdminTokenOperationsQuery,
    limit: u32,
    offset: u32,
) -> AppResult<(Vec<AdminTokenOperation>, i64)> {
    let mut rows = QueryBuilder::new(
        "SELECT o.id,o.project_id,p.symbol,o.actor_type,o.actor_id,o.action,o.reason,o.payload_json,o.created_at FROM new_coin_operations o JOIN new_coin_projects p ON p.id=o.project_id WHERE 1=1",
    );
    let mut count = QueryBuilder::new(
        "SELECT COUNT(*) FROM new_coin_operations o JOIN new_coin_projects p ON p.id=o.project_id WHERE 1=1",
    );
    for q in [&mut rows, &mut count] {
        for (column, value) in [
            ("project_id", filter.project_id),
            ("actor_id", filter.actor_id),
        ] {
            if let Some(value) = value {
                q.push(format!(" AND o.{column}=")).push_bind(value);
            }
        }
        for (column, value) in [
            ("actor_type", &filter.actor_type),
            ("action", &filter.action),
        ] {
            if let Some(value) = value.as_ref().filter(|s| !s.trim().is_empty()) {
                q.push(format!(" AND o.{column}="))
                    .push_bind(value.trim().to_owned());
            }
        }
    }
    rows.push(" ORDER BY o.id DESC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let mut tx = pool.begin().await?;
    let total = count.build_query_scalar().fetch_one(&mut *tx).await?;
    let rows = rows.build_query_as().fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok((rows, total))
}

/// 调用方先锁资产；新币 Logo 与创建项目、发布交易对及审计在同一事务内提交或回滚。
pub(crate) async fn set_asset_logo(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    asset: u64,
    logo: &str,
) -> crate::error::AppResult<()> {
    sqlx::query("UPDATE assets SET logo_url=? WHERE id=?")
        .bind(logo)
        .bind(asset)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
