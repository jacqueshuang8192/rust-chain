//! 子树钱包账变只读查询：账户表名仅由已验证白名单选择，用户输入全部参数绑定。

use super::push_scoped_financial_user_predicate;
use crate::{
    error::AppResult,
    modules::agent::{
        presentation::wallet_records::{
            AgentWalletRecord, AgentWalletRecordsQuery, AgentWalletRecordsResponse,
        },
        repository::AgentAccessScope,
        service::agent_financial_list_page,
    },
};
use sqlx::{MySql, Pool, QueryBuilder};

/// 分页读取现货或杠杆原始流水及对应余额快照；两个 SELECT 使用相同子树、用户及筛选谓词。
/// 不读取或暴露平台会计对手腿，不锁钱包，不追加审计或资金记录；数据库失败保持错误语义。
pub(crate) async fn list_records(
    pool: &Pool<MySql>,
    scope: &AgentAccessScope,
    user_id: u64,
    filter: &AgentWalletRecordsQuery,
) -> AppResult<AgentWalletRecordsResponse> {
    let page = agent_financial_list_page(filter.limit, filter.offset);
    let total = query(scope, user_id, filter, true)
        .build_query_scalar::<i64>()
        .fetch_one(pool)
        .await?;
    let mut rows = query(scope, user_id, filter, false);
    rows.push(" ORDER BY ledger.created_at DESC, ledger.id DESC LIMIT ")
        .push_bind(i64::from(page.limit))
        .push(" OFFSET ")
        .push_bind(i64::from(page.offset));
    let ledger = rows
        .build_query_as::<AgentWalletRecord>()
        .fetch_all(pool)
        .await?;
    Ok(AgentWalletRecordsResponse { ledger, total })
}

fn query(
    scope: &AgentAccessScope,
    user_id: u64,
    filter: &AgentWalletRecordsQuery,
    count: bool,
) -> QueryBuilder<'static, MySql> {
    let (table, account) = if filter.account_type.as_deref() == Some("margin") {
        ("margin_wallet_ledger", "margin")
    } else {
        ("wallet_ledger", "spot")
    };
    let mut builder = QueryBuilder::new(if count {
        "SELECT COUNT(*)".to_owned()
    } else {
        format!(
            "SELECT ledger.id, '{account}' AS account_type, ledger.user_id, users.email AS user_email, \
            ledger.asset_id, assets.symbol AS asset_symbol, ledger.change_type, ledger.amount, ledger.balance_type, \
            ledger.balance_after, ledger.available_after, ledger.frozen_after, ledger.locked_after, \
            ledger.ref_type, ledger.ref_id, ledger.created_at"
        )
    });
    builder.push(format!(
        " FROM {table} ledger \
        INNER JOIN users ON users.id = ledger.user_id \
        INNER JOIN assets ON assets.id = ledger.asset_id \
        INNER JOIN user_referrals referrals ON referrals.user_id = ledger.user_id \
        INNER JOIN agents owner_agents ON owner_agents.id = referrals.root_agent_id"
    ));
    push_scoped_financial_user_predicate(&mut builder, "ledger.user_id", scope, user_id);
    if let Some(asset_id) = filter.asset_id {
        builder.push(" AND ledger.asset_id = ").push_bind(asset_id);
    }
    for (column, value) in [
        ("change_type", &filter.change_type),
        ("balance_type", &filter.balance_type),
        ("ref_type", &filter.ref_type),
        ("ref_id", &filter.ref_id),
    ] {
        if let Some(value) = value.as_ref().filter(|value| !value.trim().is_empty()) {
            builder
                .push(format!(" AND ledger.{column} = "))
                .push_bind(value.trim().to_owned());
        }
    }
    builder
}
