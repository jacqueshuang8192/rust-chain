//! 人工恢复请求与双进度调度；跳转、历史范围和审计在同一网络锁内持久化。
use super::*;
use crate::modules::wallet::presentation::timed_deposits::RecoverScanner;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Request {
    mode: String,
    admin_id: u64,
    reason: String,
}
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Work {
    Live,
    Recover,
    Backfill,
}

pub(super) struct ScanFailure {
    // 错误载荷间接存储，避免 AppError 与重试标记合并后触发大 Result 错误检查。
    pub error: Box<AppError>,
    pub retryable: bool,
}
impl From<AppError> for ScanFailure {
    fn from(error: AppError) -> Self {
        Self {
            error: Box::new(error),
            retryable: false,
        }
    }
}

/// 网络写权限入口只排队，不调用节点或改变余额；版本锁隔离重放与并发编辑，显式跳过才允许丢弃补扫范围。
pub(crate) async fn request(
    pool: &Pool<MySql>,
    admin_id: u64,
    network: &str,
    request: &RecoverScanner,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    super::super::lock_network(&mut tx, network).await?;
    let (revision, configured, cursor, pending, backfill): (u64, bool, Option<Json<Value>>, bool, Option<Json<Value>>) = sqlx::query_as("SELECT revision,endpoint_encrypted IS NOT NULL,cursor_json,recovery_request_json IS NOT NULL,backfill_cursor_json FROM wallet_deposit_scanners WHERE network=?")
        .bind(network).fetch_one(&mut *tx).await?;
    if revision != request.expected_revision || pending || cursor.is_none() || !configured {
        return Err(AppError::Conflict(
            "监听配置已变化、恢复请求已排队或尚未建立游标，请刷新后重试".into(),
        ));
    }
    let pending = Request {
        mode: request.mode.clone(),
        admin_id,
        reason: request.reason.clone(),
    };
    sqlx::query("UPDATE wallet_deposit_scanners SET recovery_request_json=?,revision=revision+1,last_scan_at=NULL,lease_token=NULL,lease_until=NULL WHERE network=?")
        .bind(Json(&pending)).bind(network).execute(&mut *tx).await?;
    super::super::audit(&mut tx, admin_id, "deposit_scanner.recovery_requested", 0,
        Some(json!({"network":network,"cursor":cursor,"backfill_cursor":backfill,"revision":revision})),
        json!({"network":network,"mode":request.mode,"revision":revision+1,"acknowledge_skip":request.acknowledge_skip}), &request.reason).await?;
    tx.commit().await?;
    Ok(())
}

/// 目标与旧锚点均从同一已验证渠道读取；只跳到确认高度，链倒退或重组不会被人工跳转掩盖。
pub(super) async fn confirmed_head(
    node: &Node,
    config: &ScannerConfig,
    context: &Context,
    safe: u64,
) -> AppResult<Cursor> {
    let old = &config
        .cursor_json
        .as_ref()
        .ok_or_else(|| failure("recovery requires a persisted checkpoint"))?
        .0;
    verify_anchor(node, old, context, safe).await?;
    if config
        .recovery_request_json
        .as_ref()
        .is_some_and(|r| r.mode == "backfill")
        && let Some(Json(target)) = &config.backfill_target_json
    {
        verify_anchor(node, target, context, safe).await?;
    }
    let height = *node
        .heights(safe.saturating_sub(32), safe)
        .await?
        .last()
        .ok_or_else(|| failure("no confirmed block available for recovery"))?;
    let block = node
        .block(
            height,
            &Context {
                network: context.network.clone(),
                tip: context.tip,
                assets: vec![],
                addresses: HashSet::new(),
            },
        )
        .await?;
    Ok(Cursor {
        height: safe,
        hash_height: height,
        hash: block.hash,
    })
}

/// 补扫结算前检查固定目标哈希，不能把已经重组的历史交易作为有效到账。
pub(super) async fn verify_anchor(
    node: &Node,
    anchor: &Cursor,
    context: &Context,
    safe: u64,
) -> AppResult<()> {
    if safe < anchor.height {
        return Err(failure("chain head moved behind the recovery anchor"));
    }
    let block = node
        .block(
            anchor.hash_height,
            &Context {
                network: context.network.clone(),
                tip: context.tip,
                assets: vec![],
                addresses: HashSet::new(),
            },
        )
        .await?;
    if block.hash != anchor.hash {
        return Err(failure(
            "confirmed chain reorganization at recovery anchor; reconcile before recovery",
        ));
    }
    Ok(())
}

/// 实时优先；仅实时进度在30秒内且补扫到期时安排一轮历史任务，复用网络租约和所有渠道节流。
pub(super) async fn run_network(
    pool: &Pool<MySql>,
    key: Option<&str>,
    config: ScannerConfig,
) -> AppResult<u64> {
    let token = uuid::Uuid::now_v7().to_string();
    let claim = sqlx::query("UPDATE wallet_deposit_scanners SET lease_token=?,lease_until=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 60 SECOND) WHERE network=? AND revision=? AND (lease_until IS NULL OR lease_until<CURRENT_TIMESTAMP(6))")
        .bind(&token).bind(&config.network).bind(config.revision).execute(pool).await?;
    if claim.rows_affected() != 1 {
        return Ok(0);
    }
    // 认领后重读游标，避免另一个worker在列表读取与认领之间推进时使用旧快照回退进度。
    let config = sqlx::query_as::<_, ScannerConfig>(&format!("{CONFIG_SELECT} WHERE network=?"))
        .bind(&config.network)
        .fetch_one(pool)
        .await?;
    let work = if config.recovery_request_json.is_some() {
        Work::Recover
    } else if config.backfill_cursor_json.is_some()
        && config.backfill_due
        && (config.live_recent || config.backfill_blocked)
    {
        Work::Backfill
    } else {
        Work::Live
    };
    let result = tokio::time::timeout(
        Duration::from_secs(45),
        provider_runtime::scan_available(pool, key, &config, &token, work),
    )
    .await
    .unwrap_or_else(|_| {
        Err(ScanFailure {
            error: Box::new(failure("scanner round timed out")),
            retryable: true,
        })
    });
    match result {
        Ok((cursor, count, caught_up)) => {
            complete(pool, &config, &token, work, cursor, caught_up).await?;
            Ok(count)
        }
        Err(error) => {
            let message = match error.error.as_ref() {
                AppError::Internal(s) | AppError::Validation(s) | AppError::Conflict(s) => {
                    s.as_str()
                }
                _ => "deposit scanner persistence failed",
            };
            if work == Work::Backfill {
                sqlx::query("UPDATE wallet_deposit_scanners SET backfill_last_error=?,backfill_blocked=IF(?,backfill_blocked,TRUE),last_error=IF(?,last_error,?),backfill_next_at=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 15 SECOND),lease_token=NULL,lease_until=NULL WHERE network=? AND revision=? AND lease_token=?")
                    .bind(message.chars().take(255).collect::<String>()).bind(error.retryable).bind(error.retryable).bind(message.chars().take(255).collect::<String>()).bind(&config.network).bind(config.revision).bind(&token).execute(pool).await?;
            } else {
                sqlx::query("UPDATE wallet_deposit_scanners SET last_error=?,lease_token=NULL,lease_until=NULL WHERE network=? AND revision=? AND lease_token=?")
                    .bind(message.chars().take(255).collect::<String>()).bind(&config.network).bind(config.revision).bind(&token).execute(pool).await?;
            }
            tracing::warn!(network=%config.network,error=%message,"链上充值扫描失败，保留实时与历史进度等待重试");
            Ok(0)
        }
    }
}

async fn complete(
    pool: &Pool<MySql>,
    config: &ScannerConfig,
    token: &str,
    work: Work,
    cursor: Cursor,
    caught_up: bool,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    super::super::lock_network(&mut tx, &config.network).await?;
    let valid: bool = sqlx::query_scalar("SELECT revision=? AND COALESCE(lease_token=?,FALSE) AND COALESCE(lease_until>CURRENT_TIMESTAMP(6),FALSE) FROM wallet_deposit_scanners WHERE network=?")
        .bind(config.revision).bind(token).bind(&config.network).fetch_one(&mut *tx).await?;
    if !valid {
        return Ok(());
    }
    match work {
        Work::Recover => {
            let request = &config
                .recovery_request_json
                .as_ref()
                .ok_or_else(|| failure("recovery request missing"))?
                .0;
            let old = config
                .cursor_json
                .as_ref()
                .ok_or_else(|| failure("recovery checkpoint missing"))?;
            let retained = config.backfill_cursor_json.as_ref().unwrap_or(old);
            let backfill = if request.mode == "backfill" && retained.height < cursor.height {
                Some(retained)
            } else {
                None
            };
            let blocked = backfill.is_some() && config.backfill_blocked;
            sqlx::query("UPDATE wallet_deposit_scanners SET cursor_json=?,backfill_cursor_json=?,backfill_target_json=?,backfill_last_error=IF(?,backfill_last_error,NULL),backfill_blocked=?,backfill_next_at=CURRENT_TIMESTAMP(6),recovery_request_json=NULL,revision=revision+1,last_scan_at=NULL,last_error=IF(?,last_error,NULL),lease_token=NULL,lease_until=NULL WHERE network=?")
                .bind(Json(&cursor)).bind(backfill).bind(backfill.map(|_| Json(&cursor))).bind(blocked).bind(blocked).bind(blocked).bind(&config.network).execute(&mut *tx).await?;
            super::super::audit(&mut tx, request.admin_id, "deposit_scanner.recovery_applied", 0,
                Some(json!({"network":config.network,"cursor":old,"backfill_cursor":config.backfill_cursor_json,"backfill_target":config.backfill_target_json})),
                json!({"network":config.network,"mode":request.mode,"cursor":cursor,"backfill_cursor":backfill,"revision":config.revision+1}), &request.reason).await?;
        }
        Work::Backfill => {
            sqlx::query("UPDATE wallet_deposit_scanners SET backfill_cursor_json=?,backfill_target_json=IF(?,NULL,backfill_target_json),backfill_last_error=NULL,last_scan_at=IF(backfill_blocked,NULL,last_scan_at),last_error=IF(backfill_blocked,NULL,last_error),backfill_blocked=FALSE,backfill_next_at=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL 15 SECOND),lease_token=NULL,lease_until=NULL WHERE network=?")
                .bind(if caught_up { None } else { Some(Json(cursor)) }).bind(caught_up).bind(&config.network).execute(&mut *tx).await?;
        }
        Work::Live => {
            sqlx::query("UPDATE wallet_deposit_scanners SET cursor_json=?,last_scan_at=IF(?,CURRENT_TIMESTAMP(6),last_scan_at),last_error=IF(COALESCE(backfill_blocked,FALSE),COALESCE(backfill_last_error,'历史补扫异常，需核对'),NULL),lease_token=NULL,lease_until=NULL WHERE network=?")
                .bind(Json(cursor)).bind(caught_up).bind(&config.network).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
