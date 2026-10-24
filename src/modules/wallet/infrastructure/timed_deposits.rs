//! 限时订单、监听配置与链上证据持久化，资金变更复用原钱包入账事务。

pub(crate) mod orders;
pub(crate) mod providers;
pub(crate) mod scanners;
pub(crate) mod settings;
pub(crate) mod settlement;

#[cfg(test)]
#[path = "../../../../tests/unit_src/src_modules_wallet_timed_deposit_database_tests.rs"]
mod tests;

use crate::{
    error::{AppError, AppResult},
    modules::admin::infrastructure::{AdminAuditLogEntry, insert_admin_audit_log_entry_in_tx},
};
use serde_json::Value;
use sqlx::{MySql, Transaction};

/// 限时充值的配置与人工核对审计和业务共用事务，记录原因但绝不记录监听凭据。
pub(super) async fn audit(
    tx: &mut Transaction<'_, MySql>,
    admin_id: u64,
    action: &'static str,
    target_id: u64,
    before: Option<Value>,
    after: Value,
    reason: &str,
) -> AppResult<()> {
    insert_admin_audit_log_entry_in_tx(
        tx,
        admin_id,
        AdminAuditLogEntry {
            action,
            target_type: "timed_deposit",
            target_id,
            before_json: before,
            after_json: Some(after),
            reason: Some(reason.to_owned()),
        },
    )
    .await
}

/// 所有分配、匹配与配置写入先锁同一网络行；数据库时间作为三十分钟租约的唯一时钟。
pub(super) async fn lock_network(
    tx: &mut Transaction<'_, MySql>,
    network: &str,
) -> AppResult<chrono::DateTime<chrono::Utc>> {
    sqlx::query("SELECT network FROM wallet_deposit_scanners WHERE network=? FOR UPDATE")
        .bind(network)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(sqlx::query_scalar("SELECT CURRENT_TIMESTAMP(6)")
        .fetch_one(&mut **tx)
        .await?)
}
