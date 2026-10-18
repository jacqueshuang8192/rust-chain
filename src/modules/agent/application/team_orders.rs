//! 团队订单入口：先校验筛选，再读取令牌所属代理子树，全程不触发金融写操作。

use super::agent_context;
use crate::{
    error::{AppError, AppResult},
    modules::agent::{
        infrastructure::team_orders,
        presentation::team_orders::{AgentTeamOrdersQuery, AgentTeamOrdersResponse},
        repository::{AgentListPage, AgentOrderKind, AgentTeamOrderFilter},
        service::{
            normalized_agent_margin_order_status, normalized_agent_seconds_order_status,
            normalized_agent_spot_order_status,
        },
    },
};
use chrono::{DateTime, Utc};
use sqlx::{MySql, Pool};

/// 校验业务口径后按当前身份执行有界分页；筛选他人代理或用户只会得到空集。
/// 不调用交易、计息、行情或钱包服务，禁止查询产生结算或补账副作用。
pub(crate) async fn list_orders(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    kind: &str,
    query: AgentTeamOrdersQuery,
) -> AppResult<AgentTeamOrdersResponse> {
    let filter = normalize_filter(kind, query)?;
    let (pool, scope) = agent_context(mysql, subject).await?;
    team_orders::list_orders(&pool, &scope, &filter).await
}

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.to_owned())
}

fn text(value: Option<String>, maximum: usize) -> AppResult<Option<String>> {
    let value = value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if value
        .as_ref()
        .is_some_and(|value| value.len() > maximum || value.chars().any(char::is_control))
    {
        return Err(invalid("筛选文本过长或包含控制字符"));
    }
    Ok(value)
}

fn timestamp(value: Option<i64>) -> AppResult<Option<DateTime<Utc>>> {
    value
        .map(|value| {
            if !(0..=253_402_300_799_999).contains(&value) {
                return Err(invalid("筛选时间必须为有效的 Unix 毫秒时间戳"));
            }
            DateTime::from_timestamp_millis(value).ok_or_else(|| invalid("筛选时间无效"))
        })
        .transpose()
}

fn normalize_filter(kind: &str, query: AgentTeamOrdersQuery) -> AppResult<AgentTeamOrderFilter> {
    let kind = match kind {
        "seconds" => AgentOrderKind::Seconds,
        "margin" => AgentOrderKind::Margin,
        "earn" => AgentOrderKind::Earn,
        "spot" => AgentOrderKind::Spot,
        "new-coin-subscriptions" => AgentOrderKind::NewCoinSubscription,
        "new-coin-purchases" => AgentOrderKind::NewCoinPurchase,
        _ => return Err(AppError::NotFound),
    };
    let new_coin = matches!(
        kind,
        AgentOrderKind::NewCoinSubscription | AgentOrderKind::NewCoinPurchase
    );
    if (new_coin && query.product_id.is_some()) || (!new_coin && query.project_id.is_some()) {
        return Err(invalid("该订单类型不支持此产品或项目筛选"));
    }
    let team_scope = text(query.team_scope, 16)?.unwrap_or_else(|| "all".to_owned());
    if !["all", "own", "descendants"].contains(&team_scope.as_str()) {
        return Err(invalid("团队范围无效"));
    }
    for id in [
        query.agent_id,
        query.user_id,
        query.order_id,
        query.product_id,
        query.project_id,
    ]
    .into_iter()
    .flatten()
    {
        if !(1..=9_007_199_254_740_991).contains(&id) {
            return Err(invalid("筛选 ID 必须为正安全整数"));
        }
    }
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);
    if !(1..=100).contains(&limit) || offset > 100_000 {
        return Err(invalid("分页大小须为 1-100，偏移不可超过 100000"));
    }
    let status = text(query.status, 32)?;
    let status = match kind {
        AgentOrderKind::Seconds => normalized_agent_seconds_order_status(status)?,
        AgentOrderKind::Margin => normalized_agent_margin_order_status(status)?,
        AgentOrderKind::Spot => normalized_agent_spot_order_status(status)?,
        AgentOrderKind::NewCoinSubscription | AgentOrderKind::NewCoinPurchase => {
            let allowed: &[&str] = if kind == AgentOrderKind::NewCoinSubscription {
                &[
                    "pending",
                    "allocated",
                    "partial_allocated",
                    "refunded",
                    "available",
                ]
            } else {
                &["pending", "locked", "available"]
            };
            if status
                .as_ref()
                .is_some_and(|value| !allowed.contains(&value.as_str()))
            {
                return Err(invalid("新币订单状态无效"));
            }
            status
        }
        AgentOrderKind::Earn => {
            if status
                .as_ref()
                .is_some_and(|value| !["subscribed", "redeemed"].contains(&value.as_str()))
            {
                return Err(invalid("理财订单状态无效"));
            }
            status
        }
    };
    let direction = text(query.direction, 16)?;
    let allowed: &[&str] = match kind {
        AgentOrderKind::Seconds => &["up", "down"],
        AgentOrderKind::Margin => &["long", "short"],
        AgentOrderKind::Spot => &["buy", "sell"],
        AgentOrderKind::Earn
        | AgentOrderKind::NewCoinSubscription
        | AgentOrderKind::NewCoinPurchase => &[],
    };
    if direction
        .as_ref()
        .is_some_and(|value| !allowed.contains(&value.as_str()))
    {
        return Err(invalid("订单方向筛选无效"));
    }
    let result = text(query.result, 16)?;
    if result.as_ref().is_some_and(|value| {
        kind != AgentOrderKind::Seconds || !["win", "loss"].contains(&value.as_str())
    }) {
        return Err(invalid("输赢筛选仅支持秒合约 win 或 loss"));
    }
    let symbol = text(query.symbol, 64)?;
    if (kind == AgentOrderKind::Earn && symbol.is_some())
        || (kind == AgentOrderKind::Spot && query.product_id.is_some())
    {
        return Err(invalid("该订单类型不支持此产品或交易对筛选"));
    }
    let created_from = timestamp(query.created_from)?;
    let created_to = timestamp(query.created_to)?;
    if let (Some(from), Some(to)) = (created_from, created_to)
        && from >= to
    {
        return Err(invalid("结束时间必须晚于开始时间"));
    }
    Ok(AgentTeamOrderFilter {
        kind,
        team_scope,
        agent_id: query.agent_id,
        user_id: query.user_id,
        order_id: query.order_id,
        product_id: query.product_id,
        project_id: query.project_id,
        email: text(query.email, 254)?,
        symbol,
        status,
        direction,
        result,
        created_from,
        created_to,
        page: AgentListPage { limit, offset },
    })
}
