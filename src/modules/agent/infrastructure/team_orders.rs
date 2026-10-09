//! 团队订单只读 SQL：列表与 COUNT 重复同一子树谓词，所有用户输入均绑定。

use crate::{
    error::AppResult,
    modules::agent::{
        presentation::team_orders::{AgentTeamOrder, AgentTeamOrderPage, AgentTeamOrdersResponse},
        repository::{AgentAccessScope, AgentOrderKind, AgentTeamOrderFilter},
    },
};
use sqlx::{MySql, Pool, QueryBuilder};

/// 用相同查询构造器读取四类订单及总数，保持业务 DTO 和金额文本；不锁行也不写账本。
/// 范围取自身份路径；agent_id 只在该范围内选分支，不能让子代理读到父级或兄弟数据。
pub(crate) async fn list_orders(
    pool: &Pool<MySql>,
    scope: &AgentAccessScope,
    filter: &AgentTeamOrderFilter,
) -> AppResult<AgentTeamOrdersResponse> {
    let mut count = query(scope, filter, true);
    let total = count.build_query_scalar::<i64>().fetch_one(pool).await?;
    let mut rows = query(scope, filter, false);
    rows.push(format!(
        " ORDER BY {} DESC, orders.id DESC LIMIT ",
        created_column(filter.kind)
    ));
    rows.push_bind(i64::from(filter.page.limit));
    rows.push(" OFFSET ")
        .push_bind(i64::from(filter.page.offset));
    // 各分支仅决定静态 DTO，查询条件和总数逻辑在上方共用。
    Ok(match filter.kind {
        AgentOrderKind::Seconds => AgentTeamOrdersResponse::Seconds(AgentTeamOrderPage {
            orders: rows
                .build_query_as::<AgentTeamOrder<_>>()
                .fetch_all(pool)
                .await?,
            total,
        }),
        AgentOrderKind::Margin => AgentTeamOrdersResponse::Margin(AgentTeamOrderPage {
            orders: rows
                .build_query_as::<AgentTeamOrder<_>>()
                .fetch_all(pool)
                .await?,
            total,
        }),
        AgentOrderKind::Spot => AgentTeamOrdersResponse::Spot(AgentTeamOrderPage {
            orders: rows
                .build_query_as::<AgentTeamOrder<_>>()
                .fetch_all(pool)
                .await?,
            total,
        }),
        AgentOrderKind::Earn => AgentTeamOrdersResponse::Earn(AgentTeamOrderPage {
            orders: rows
                .build_query_as::<AgentTeamOrder<_>>()
                .fetch_all(pool)
                .await?,
            total,
        }),
    })
}

fn created_column(kind: AgentOrderKind) -> &'static str {
    if kind == AgentOrderKind::Earn {
        "orders.subscribed_at"
    } else {
        "orders.created_at"
    }
}

fn query(
    scope: &AgentAccessScope,
    filter: &AgentTeamOrderFilter,
    count: bool,
) -> QueryBuilder<'static, MySql> {
    let (columns, from) = match filter.kind {
        AgentOrderKind::Seconds => (
            "orders.product_id, orders.pair_id, pairs.symbol, orders.stake_asset, \
             assets.symbol AS stake_asset_symbol, orders.direction, orders.stake_amount, \
             orders.duration_seconds, orders.payout_rate, orders.entry_price, \
             orders.settlement_price, orders.status, orders.result, orders.expires_at, \
             orders.created_at, orders.settled_at",
            "seconds_contract_orders orders JOIN trading_pairs pairs ON pairs.id = orders.pair_id \
             JOIN assets ON assets.id = orders.stake_asset",
        ),
        AgentOrderKind::Margin => (
            "orders.product_id, orders.pair_id, pairs.symbol, orders.margin_asset, \
             assets.symbol AS margin_asset_symbol, orders.wallet_scope, orders.margin_mode, \
             orders.direction, orders.order_type, orders.margin_amount, orders.leverage, \
             orders.notional_amount, orders.borrowed_amount, orders.interest_amount, \
             orders.entry_price, orders.limit_price, orders.exit_price, orders.realized_pnl, \
             orders.opened_at, orders.created_at, orders.closed_at, orders.status",
            "margin_positions orders JOIN trading_pairs pairs ON pairs.id = orders.pair_id \
             JOIN assets ON assets.id = orders.margin_asset",
        ),
        AgentOrderKind::Spot => (
            "orders.pair_id, pairs.symbol, orders.side, orders.order_type, orders.price, \
             orders.trigger_price, orders.quantity, orders.filled_quantity, orders.status, \
             orders.created_at, orders.updated_at",
            "spot_orders orders JOIN trading_pairs pairs ON pairs.id = orders.pair_id",
        ),
        AgentOrderKind::Earn => (
            "orders.product_id, products.name AS product_name, orders.asset_id, \
             assets.symbol AS asset_symbol, orders.amount, orders.apr_rate, orders.term_days, \
             orders.status, orders.subscribed_at AS created_at, orders.matures_at, orders.redeemed_at",
            "earn_subscriptions orders JOIN earn_products products ON products.id = orders.product_id \
             JOIN assets ON assets.id = orders.asset_id",
        ),
    };
    let mut builder = QueryBuilder::new(if count {
        "SELECT COUNT(*)"
    } else {
        "SELECT users.email, owner_agents.id AS owner_agent_id, owner_agents.agent_code AS owner_agent_code, \
         orders.id, orders.user_id, "
    });
    if !count {
        builder.push(columns);
    }
    builder
        .push(" FROM ")
        .push(from)
        .push(
            " JOIN users ON users.id = orders.user_id \
         JOIN user_referrals referrals ON referrals.user_id = orders.user_id \
         JOIN agents owner_agents ON owner_agents.id = referrals.root_agent_id \
         WHERE (owner_agents.path = ",
        )
        .push_bind(scope.path.clone())
        .push(" OR owner_agents.path LIKE CONCAT(")
        .push_bind(scope.path.clone())
        .push(", '/%'))");
    match filter.team_scope.as_str() {
        "own" => {
            builder
                .push(" AND owner_agents.id = ")
                .push_bind(scope.agent_id);
        }
        "descendants" => {
            builder
                .push(" AND owner_agents.id <> ")
                .push_bind(scope.agent_id);
        }
        _ => {}
    }
    if let Some(agent_id) = filter.agent_id {
        builder.push(" AND EXISTS (SELECT 1 FROM agents selected_agent WHERE selected_agent.id = ")
            .push_bind(agent_id)
            .push(" AND (selected_agent.path = ").push_bind(scope.path.clone())
            .push(" OR selected_agent.path LIKE CONCAT(").push_bind(scope.path.clone()).push(", '/%'))")
            .push(" AND (owner_agents.path = selected_agent.path OR owner_agents.path LIKE CONCAT(selected_agent.path, '/%')))");
    }
    for (column, value) in [
        ("orders.user_id", filter.user_id),
        ("orders.id", filter.order_id),
        ("orders.product_id", filter.product_id),
    ] {
        if let Some(value) = value {
            builder
                .push(" AND ")
                .push(column)
                .push(" = ")
                .push_bind(value);
        }
    }
    if let Some(email) = &filter.email {
        builder
            .push(" AND LOCATE(")
            .push_bind(email.clone())
            .push(", users.email) > 0");
    }
    if let Some(symbol) = &filter.symbol {
        builder
            .push(
                " AND REPLACE(REPLACE(REPLACE(UPPER(pairs.symbol), '-', ''), '/', ''), '_', '') = ",
            )
            .push_bind(symbol.replace(['-', '/', '_'], "").to_uppercase());
    }
    if filter.kind == AgentOrderKind::Margin && filter.status.as_deref() == Some("pending") {
        builder.push(" AND orders.status = 'opened' AND orders.entry_price IS NULL");
    } else if let Some(status) = &filter.status {
        builder
            .push(" AND orders.status = ")
            .push_bind(status.clone());
        if filter.kind == AgentOrderKind::Margin && status == "opened" {
            builder.push(" AND orders.entry_price IS NOT NULL");
        }
    }
    if let Some(direction) = &filter.direction {
        builder
            .push(if filter.kind == AgentOrderKind::Spot {
                " AND orders.side = "
            } else {
                " AND orders.direction = "
            })
            .push_bind(direction.clone());
    }
    if let Some(result) = &filter.result {
        builder
            .push(" AND orders.result = ")
            .push_bind(result.clone());
    }
    if let Some(from) = filter.created_from {
        builder
            .push(" AND ")
            .push(created_column(filter.kind))
            .push(" >= ")
            .push_bind(from.naive_utc());
    }
    if let Some(to) = filter.created_to {
        builder
            .push(" AND ")
            .push(created_column(filter.kind))
            .push(" < ")
            .push_bind(to.naive_utc());
    }
    builder
}
