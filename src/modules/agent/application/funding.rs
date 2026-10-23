//! 团队充提历史查询，先验证筛选与代理祖先状态，再按子树读取持久化订单。

use super::agent_context;
use crate::{
    error::{AppError, AppResult},
    modules::agent::{
        infrastructure::funding,
        presentation::funding::{AgentFundingPage, AgentFundingQuery},
        repository::{
            AgentListPage,
            funding::{FundingFilter, FundingKind},
        },
    },
};
use chrono::{DateTime, Utc};
use sqlx::{MySql, Pool};

/// 代理可查看自己和下级归属用户的充提状态；无权筛选为空集，不补建钱包也不驱动资金状态机。
/// 参数错误在数据库访问前返回；停用祖先、身份错误沿用代理上下文的认证语义。
pub(crate) async fn list_records(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    kind: &str,
    query: AgentFundingQuery,
) -> AppResult<AgentFundingPage> {
    let filter = normalize(kind, query)?;
    let (pool, scope) = agent_context(mysql, subject).await?;
    funding::list_records(&pool, &scope, &filter).await
}

fn invalid(message: &str) -> AppError {
    AppError::Validation(message.to_owned())
}
fn text(value: Option<String>, maximum: usize) -> AppResult<Option<String>> {
    let value = value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
    if value
        .as_ref()
        .is_some_and(|v| v.len() > maximum || v.chars().any(char::is_control))
    {
        return Err(invalid("筛选文本过长或包含控制字符"));
    }
    Ok(value)
}
fn timestamp(value: Option<i64>) -> AppResult<Option<DateTime<Utc>>> {
    value
        .map(|v| {
            if !(0..=253_402_300_799_999).contains(&v) {
                return Err(invalid("筛选时间必须为有效 Unix 毫秒时间戳"));
            }
            DateTime::from_timestamp_millis(v).ok_or_else(|| invalid("筛选时间无效"))
        })
        .transpose()
}
fn normalize(kind: &str, query: AgentFundingQuery) -> AppResult<FundingFilter> {
    let kind = match kind {
        "deposits" => FundingKind::Deposits,
        "withdrawals" => FundingKind::Withdrawals,
        _ => return Err(AppError::NotFound),
    };
    let team_scope = text(query.team_scope, 16)?.unwrap_or_else(|| "all".to_owned());
    if !["all", "own", "descendants"].contains(&team_scope.as_str()) {
        return Err(invalid("团队范围无效"));
    }
    for id in [query.agent_id, query.user_id, query.record_id]
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
    let source = text(query.source, 16)?;
    if source
        .as_deref()
        .is_some_and(|v| kind != FundingKind::Deposits || !["quick", "chain"].contains(&v))
    {
        return Err(invalid("充值来源无效"));
    }
    let status = text(query.status, 32)?;
    let allowed = if kind == FundingKind::Deposits {
        &[
            "created", "pending", "expired", "review", "credited", "reversed", "failed",
        ][..]
    } else {
        &[
            "pending_review",
            "approved",
            "broadcasting",
            "unknown_broadcast",
            "broadcasted",
            "confirmed",
            "rejected",
            "failed",
            "manual_review",
        ][..]
    };
    if status.as_deref().is_some_and(|v| !allowed.contains(&v)) {
        return Err(invalid("充提记录状态无效"));
    }
    let order_no = text(query.order_no, 64)?;
    if kind == FundingKind::Withdrawals && order_no.is_some() {
        return Err(invalid("提现请使用记录 ID 筛选"));
    }
    let created_from = timestamp(query.created_from)?;
    let created_to = timestamp(query.created_to)?;
    if let (Some(from), Some(to)) = (created_from, created_to)
        && from >= to
    {
        return Err(invalid("结束时间必须晚于开始时间"));
    }
    Ok(FundingFilter {
        kind,
        team_scope,
        agent_id: query.agent_id,
        user_id: query.user_id,
        record_id: query.record_id,
        email: text(query.email, 254)?,
        order_no,
        asset_symbol: text(query.asset_symbol, 32)?.map(|v| v.to_uppercase()),
        network: text(query.network, 64)?.map(|v| v.to_lowercase()),
        source,
        status,
        created_from,
        created_to,
        page: AgentListPage { limit, offset },
    })
}
