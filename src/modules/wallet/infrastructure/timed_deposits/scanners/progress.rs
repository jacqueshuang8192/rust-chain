//! Base 两类资产拥有独立进度；网络配置仍是统一版本屏障，SQL 标识符只由内部枚举分支生成。
use super::*;
use crate::modules::wallet::presentation::timed_deposits::{ScannerStreamView, ScannerView};
use sqlx::Transaction;

/// 只有 Base 拆分原生与代币；其他网络继续复用原来的单行进度。
pub(super) fn source(stream: &str) -> &'static str {
    if stream.is_empty() {
        "wallet_deposit_scanners p"
    } else {
        "wallet_deposit_scanners s JOIN wallet_deposit_scanner_streams p ON p.network=s.network"
    }
}

/// 所有工作共享网络版本；进度和租约字段只来自当前分流。
pub(super) fn revision(stream: &str) -> &'static str {
    if stream.is_empty() {
        "p.revision"
    } else {
        "s.revision"
    }
}

/// 固定分流条件不接受外部 SQL；非法名称拒绝匹配任何进度行。
pub(super) fn scope(stream: &str) -> &'static str {
    match stream {
        "" => "p.network=?",
        "native" => "p.network=? AND p.stream='native'",
        "token" => "p.network=? AND p.stream='token'",
        _ => "p.network=? AND FALSE",
    }
}

/// 根据独立游标装配运行快照，不从旧 Base 汇总行推测分流健康。
pub(super) fn select(stream: &str) -> String {
    let source = source(stream);
    let revision = revision(stream);
    let config = if stream.is_empty() { "p" } else { "s" };
    let stream_col = if stream.is_empty() {
        "CAST('' AS CHAR CHARACTER SET utf8mb4)"
    } else {
        "p.stream"
    };
    format!(
        "SELECT p.network,{stream_col} AS stream,{config}.confirmations,{revision} AS revision,p.cursor_json,p.recovery_request_json,p.backfill_cursor_json,p.backfill_target_json,COALESCE(p.backfill_next_at<=CURRENT_TIMESTAMP(6),TRUE) AS backfill_due,COALESCE(p.last_scan_at>DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 30 SECOND) AND p.last_error IS NULL,FALSE) AS live_recent,COALESCE(p.backfill_blocked,FALSE) AS backfill_blocked FROM {source}"
    )
}

/// 配置变更清除两路准入与租约但保留所有区块进度；调用方已持网络锁。
pub(crate) async fn invalidate(tx: &mut Transaction<'_, MySql>, network: &str) -> AppResult<()> {
    if network == "base" {
        sqlx::query("UPDATE wallet_deposit_scanner_streams SET last_scan_at=NULL,lease_token=NULL,lease_until=NULL WHERE network=?")
            .bind(network).execute(&mut **tx).await?;
    }
    Ok(())
}

/// 后台分别显示两路只读状态；Base 顶层仅作保守汇总，不参与资产准入。
pub(crate) async fn load_views(pool: &Pool<MySql>, scanners: &mut [ScannerView]) -> AppResult<()> {
    let streams=sqlx::query_as::<_,ScannerStreamView>("SELECT stream,last_scan_at,last_error,COALESCE(last_scan_at>DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 120 SECOND),FALSE) AS scan_fresh,CAST(JSON_UNQUOTE(JSON_EXTRACT(cursor_json,'$.height')) AS UNSIGNED) AS cursor_height,CAST(JSON_UNQUOTE(JSON_EXTRACT(recovery_request_json,'$.mode')) AS CHAR CHARACTER SET utf8mb4) COLLATE utf8mb4_unicode_ci AS recovery_mode,CAST(JSON_UNQUOTE(JSON_EXTRACT(backfill_cursor_json,'$.height')) AS UNSIGNED) AS backfill_cursor_height,CAST(JSON_UNQUOTE(JSON_EXTRACT(backfill_target_json,'$.height')) AS UNSIGNED) AS backfill_target_height,backfill_last_error,backfill_blocked FROM wallet_deposit_scanner_streams WHERE network='base' ORDER BY stream")
        .fetch_all(pool).await?;
    if let Some(scanner) = scanners.iter_mut().find(|s| s.network == "base") {
        scanner.scan_fresh = streams.len() == 2
            && streams
                .iter()
                .all(|s| s.scan_fresh && s.last_error.is_none());
        scanner.last_scan_at = streams.iter().map(|s| s.last_scan_at).min().flatten();
        scanner.cursor_height = streams.iter().map(|s| s.cursor_height).min().flatten();
        scanner.last_error = streams.iter().find_map(|s| {
            s.last_error.as_ref().map(|e| {
                format!(
                    "{}: {e}",
                    if s.stream == "native" {
                        "ETH 原生币"
                    } else {
                        "ERC20 代币"
                    }
                )
            })
        });
        scanner.recovery_mode = streams.iter().find_map(|s| s.recovery_mode.clone());
        scanner.backfill_cursor_height = None;
        scanner.backfill_target_height = None;
        scanner.backfill_last_error = None;
        scanner.backfill_blocked = streams.iter().any(|s| s.backfill_blocked);
        scanner.streams = streams;
    }
    Ok(())
}
