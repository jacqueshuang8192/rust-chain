//! 代理用户账变查询：先验证身份和子树成员，再读取同范围账本；不触发资金操作。

use super::{agent_context, ensure_agent_user_in_scope};
use crate::{
    error::{AppError, AppResult},
    modules::agent::{
        infrastructure,
        presentation::wallet_records::{AgentWalletRecordsQuery, AgentWalletRecordsResponse},
    },
};
use sqlx::{MySql, Pool};

/// 读取子树用户不可变资金记录，查询前校验账户/余额类型并验证祖先启用状态。
/// 行与计数查询重复身份路径谓词；无权或不存在统一为未找到，不补建钱包、不重算历史余额。
pub(crate) async fn list_records(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    user_id: u64,
    query: AgentWalletRecordsQuery,
) -> AppResult<AgentWalletRecordsResponse> {
    if !matches!(
        query.account_type.as_deref().unwrap_or("spot"),
        "spot" | "margin"
    ) {
        return Err(AppError::Validation(
            "account_type must be spot or margin".to_owned(),
        ));
    }
    if query
        .balance_type
        .as_deref()
        .is_some_and(|value| !matches!(value, "available" | "frozen" | "locked"))
    {
        return Err(AppError::Validation(
            "unsupported wallet balance_type".to_owned(),
        ));
    }
    for value in [&query.change_type, &query.ref_type, &query.ref_id]
        .into_iter()
        .flatten()
    {
        if value.len() > 128 {
            return Err(AppError::Validation(
                "wallet record filter is too long".to_owned(),
            ));
        }
    }
    let (pool, scope) = agent_context(mysql, subject).await?;
    ensure_agent_user_in_scope(&pool, &scope, user_id).await?;
    infrastructure::wallet_records::list_records(&pool, &scope, user_id, &query).await
}
