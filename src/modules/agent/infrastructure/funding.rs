//! 代理充提只读投影：快速/链上充值合并分页，提现不暴露审批凭据；行和计数共享身份谓词。

use crate::{
    error::AppResult,
    modules::agent::{
        presentation::funding::{AgentFundingPage, AgentFundingRecord},
        repository::{
            AgentAccessScope,
            funding::{FundingFilter, FundingKind},
        },
    },
};
use chrono::{DateTime, Utc};
use sqlx::{MySql, Pool, QueryBuilder};

/// 以同一到期判定时刻构造列表和总数；每个 UNION 分支均重复令牌子树与用户筛选。
/// 不锁钱包、不刷新渠道、不变更过期订单；链上冲正和人工核对状态来自原始到账事件。
pub(crate) async fn list_records(
    pool: &Pool<MySql>,
    scope: &AgentAccessScope,
    filter: &FundingFilter,
) -> AppResult<AgentFundingPage> {
    let as_of = Utc::now();
    let total = query(scope, filter, as_of, true)
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await?;
    let mut rows = query(scope, filter, as_of, false);
    rows.push(" ORDER BY funding.created_at DESC, funding.source DESC, funding.id DESC LIMIT ")
        .push_bind(i64::from(filter.page.limit))
        .push(" OFFSET ")
        .push_bind(i64::from(filter.page.offset));
    let items = rows
        .build_query_as::<AgentFundingRecord>()
        .fetch_all(pool)
        .await?;
    Ok(AgentFundingPage { items, total })
}

fn query(
    scope: &AgentAccessScope,
    filter: &FundingFilter,
    as_of: DateTime<Utc>,
    count: bool,
) -> QueryBuilder<'static, MySql> {
    let mut q = QueryBuilder::new(if count {
        "SELECT COUNT(*) FROM ("
    } else {
        "SELECT funding.* FROM ("
    });
    let sources: &[&str] = match (filter.kind, filter.source.as_deref()) {
        (FundingKind::Withdrawals, _) => &["withdrawal"],
        (_, Some("quick")) => &["quick"],
        (_, Some("chain")) => &["chain"],
        _ => &["quick", "chain"],
    };
    for (index, source) in sources.iter().enumerate() {
        if index != 0 {
            q.push(" UNION ALL ");
        }
        q.push("SELECT orders.id, orders.user_id, users.email, owner_agents.id AS owner_agent_id, owner_agents.agent_code AS owner_agent_code, orders.asset_symbol, orders.network, orders.created_at, ");
        match *source {
            "chain" => {
                q.push("'chain' AS source, orders.order_no, orders.address, orders.amount, orders.paid_amount, \
                    CASE WHEN orders.status='credited' AND (event.status IS NULL OR event.status NOT IN ('reversed','manual_review')) THEN orders.paid_amount-orders.fee_amount ELSE NULL END AS credited_amount, \
                    orders.fee_amount AS fee, NULL AS total_reserved, NULL AS fiat_amount, NULL AS fiat_currency, \
                    CASE WHEN event.status='reversed' THEN 'reversed' WHEN event.status='manual_review' THEN 'review' \
                    WHEN orders.status='pending' AND orders.expires_at<=")
                    .push_bind(as_of).push(" THEN 'expired' ELSE orders.status END AS status, \
                    event.tx_hash, orders.expires_at, orders.paid_at AS completed_at \
                    FROM wallet_deposit_orders orders LEFT JOIN wallet_deposit_events event ON event.id=orders.deposit_event_id");
            }
            "quick" => {
                q.push("'quick' AS source, orders.order_id AS order_no, orders.receive_address AS address, orders.actual_amount AS amount, \
                    CASE WHEN orders.status='paid' THEN orders.actual_amount ELSE NULL END AS paid_amount, \
                    CASE WHEN orders.status='paid' THEN orders.actual_amount ELSE NULL END AS credited_amount, \
                    CAST(0 AS DECIMAL(38,18)) AS fee, NULL AS total_reserved, orders.fiat_amount, UPPER(orders.currency) AS fiat_currency, \
                    CASE WHEN orders.status='paid' THEN 'credited' ELSE orders.status END AS status, orders.block_transaction_id AS tx_hash, \
                    FROM_UNIXTIME(orders.expiration_time) AS expires_at, orders.paid_at AS completed_at FROM quick_recharge_orders orders");
            }
            _ => {
                q.push("'withdrawal' AS source, NULL AS order_no, orders.address, orders.amount, NULL AS paid_amount, NULL AS credited_amount, \
                    orders.fee, orders.total_reserved, NULL AS fiat_amount, NULL AS fiat_currency, orders.status, orders.tx_hash, \
                    NULL AS expires_at, COALESCE(orders.confirmed_at,orders.released_at) AS completed_at FROM wallet_withdrawal_requests orders");
            }
        }
        push_scope_and_filters(&mut q, scope, filter);
    }
    q.push(") funding WHERE 1=1");
    for (column, value) in [("status", &filter.status), ("order_no", &filter.order_no)] {
        if let Some(value) = value {
            q.push(format!(" AND funding.{column} = "))
                .push_bind(value.clone());
        }
    }
    q
}

fn push_scope_and_filters(
    q: &mut QueryBuilder<'static, MySql>,
    scope: &AgentAccessScope,
    filter: &FundingFilter,
) {
    q.push(" JOIN users ON users.id=orders.user_id JOIN user_referrals referrals ON referrals.user_id=orders.user_id \
        JOIN agents owner_agents ON owner_agents.id=referrals.root_agent_id WHERE (owner_agents.path=")
        .push_bind(scope.path.clone()).push(" OR owner_agents.path LIKE CONCAT(").push_bind(scope.path.clone()).push(", '/%'))");
    match filter.team_scope.as_str() {
        "own" => {
            q.push(" AND owner_agents.id=").push_bind(scope.agent_id);
        }
        "descendants" => {
            q.push(" AND owner_agents.id<>").push_bind(scope.agent_id);
        }
        _ => {}
    }
    if let Some(agent) = filter.agent_id {
        q.push(" AND EXISTS(SELECT 1 FROM agents selected_agent WHERE selected_agent.id=").push_bind(agent)
            .push(" AND (selected_agent.path=").push_bind(scope.path.clone())
            .push(" OR selected_agent.path LIKE CONCAT(").push_bind(scope.path.clone()).push(", '/%'))")
            .push(" AND (owner_agents.path=selected_agent.path OR owner_agents.path LIKE CONCAT(selected_agent.path, '/%')))");
    }
    for (column, value) in [("user_id", filter.user_id), ("id", filter.record_id)] {
        if let Some(value) = value {
            q.push(format!(" AND orders.{column}=")).push_bind(value);
        }
    }
    if let Some(email) = &filter.email {
        q.push(" AND LOCATE(")
            .push_bind(email.clone())
            .push(",users.email)>0");
    }
    for (column, value) in [
        ("UPPER(orders.asset_symbol)", &filter.asset_symbol),
        ("LOWER(orders.network)", &filter.network),
    ] {
        if let Some(value) = value {
            q.push(" AND ")
                .push(column)
                .push("=")
                .push_bind(value.clone());
        }
    }
    if let Some(from) = filter.created_from {
        q.push(" AND orders.created_at>=").push_bind(from);
    }
    if let Some(to) = filter.created_to {
        q.push(" AND orders.created_at<").push_bind(to);
    }
}
