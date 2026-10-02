//! 新币行情草稿用例：项目身份、交易对与生成器版本分别校验，不把发行或购买开关当行情开关。

use super::admin_mysql_pool;
use super::default_market::{
    save_default_market_configuration_in_tx, validate_default_market_reference,
};
use crate::{
    error::{AppError, AppResult},
    modules::{
        admin::{
            infrastructure::*,
            presentation::*,
            service::{
                next_default_market_version, required_admin_audit_reason,
                validate_default_market_initial_price,
            },
        },
        market::infrastructure::default_runtime::PairGenerationLock,
    },
};
use serde_json::json;
use sqlx::{MySql, Pool};

/// 读取项目专属草稿和精确资产候选目录；缺草稿返回空，绝不创建生成器或推测购买交易对。
pub(crate) async fn get_new_coin_market_setup(
    pool: Option<Pool<MySql>>,
    project_id: u64,
) -> AppResult<NewCoinMarketSetupResponse> {
    let pool = admin_mysql_pool(pool)?;
    let mut tx = pool.begin().await?;
    let project = load_admin_new_coin_project_in_tx(&mut tx, project_id).await?;
    let pairs = new_coin_market_pairs(&mut tx, project.asset_id, project.quote_asset_id).await?;
    let draft = load_new_coin_market_draft(&mut tx, project_id)
        .await?
        .map(NewCoinMarketDraftRecord::response)
        .transpose()?;
    tx.commit().await?;
    Ok(NewCoinMarketSetupResponse {
        project_id,
        pairs,
        draft,
    })
}

/// 保存完整草稿并审计，按项目锁串行化并分别检查草稿/生成器版本；不获取行情发布权、不影响线上参数。
/// 已发布草稿再次编辑生成新修订；请求超时后读取修订确认结果，禁止静默附加最新版本覆盖他人修改。
pub(crate) async fn save_new_coin_market_setup(
    pool: Option<Pool<MySql>>,
    admin_id: u64,
    project_id: u64,
    request: SaveNewCoinMarketDraftRequest,
) -> AppResult<NewCoinMarketDraftResponse> {
    let reason = required_admin_audit_reason(request.reason.clone())?;
    let pool = admin_mysql_pool(pool)?;
    let mut tx = pool.begin().await?;
    let project = lock_admin_new_coin_project_in_tx(&mut tx, project_id).await?;
    let pair = lock_admin_trading_pair_in_tx(&mut tx, request.pair_id).await?;
    validate_project_pair(&project, &pair)?;
    let old = load_new_coin_market_draft(&mut tx, project_id).await?;
    let revision = next_revision(
        old.as_ref().map_or(0, |row| row.revision),
        request.expected_revision,
    )?;
    let live = load_default_market_in_tx(&mut tx, pair.id).await?;
    next_default_market_version(
        live.as_ref().map_or(0, |row| row.version),
        request.expected_market_version,
    )?;
    request.config.validate(
        u32::try_from(pair.price_precision)
            .map_err(|_| AppError::Validation("价格精度无效".into()))?,
        u32::try_from(pair.qty_precision)
            .map_err(|_| AppError::Validation("数量精度无效".into()))?,
    )?;
    validate_default_market_initial_price(request.initial_price.as_ref(), pair.price_precision)?;
    validate_default_market_reference(&mut tx, pair.id, &request.config, true).await?;
    let before = old.map(NewCoinMarketDraftRecord::response).transpose()?;
    save_new_coin_market_draft(&mut tx, project_id, admin_id, revision, &request).await?;
    let response = load_new_coin_market_draft(&mut tx, project_id)
        .await?
        .ok_or(AppError::NotFound)?
        .response()?;
    insert_admin_audit_log_entry_in_tx(
        &mut tx,
        admin_id,
        AdminAuditLogEntry {
            action: "new_coin.market_draft.save",
            target_type: "new_coin_project",
            target_id: project_id,
            before_json: before.map(|row| json!(row)),
            after_json: Some(json!(response)),
            reason: Some(reason),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(response)
}

/// 命名锁先于项目/交易对行锁，发布与凭证同事务；同一草稿重放仅返回原发布版本，不重新启用已停用行情。
/// 不启用禁用交易对、不解除全部暂停、不改变项目生命周期；过期生成器版本拒绝并要求重新核对草稿。
pub(crate) async fn publish_new_coin_market_setup(
    pool: Option<Pool<MySql>>,
    admin_id: u64,
    project_id: u64,
    request: PublishNewCoinMarketRequest,
) -> AppResult<NewCoinMarketPublishResponse> {
    let reason = required_admin_audit_reason(request.reason)?;
    let pool = admin_mysql_pool(pool)?;
    let mut guard = PairGenerationLock::acquire(&pool, request.pair_id, 5)
        .await?
        .ok_or_else(|| AppError::Conflict("行情生成正在切换，请稍后重试".into()))?;
    let mut tx = pool.begin().await?;
    let project = lock_admin_new_coin_project_in_tx(&mut tx, project_id).await?;
    let draft = load_new_coin_market_draft(&mut tx, project_id)
        .await?
        .ok_or_else(|| AppError::Validation("请先保存行情草稿".into()))?
        .response()?;
    if draft.revision != request.expected_revision || draft.pair_id != request.pair_id {
        return Err(AppError::Conflict(
            "行情草稿已变化，请重新加载并预览".into(),
        ));
    }
    if let Some(published_version) = draft.published_version {
        tx.commit().await?;
        return Ok(NewCoinMarketPublishResponse {
            revision: draft.revision,
            published_version,
            replayed: true,
        });
    }
    let pair = lock_admin_trading_pair_in_tx(&mut tx, draft.pair_id).await?;
    validate_project_pair(&project, &pair)?;
    let before = json!(draft);
    guard.ensure_owned().await?;
    let result = save_default_market_configuration_in_tx(
        &mut tx,
        admin_id,
        draft.pair_id,
        SaveDefaultMarketRequest {
            expected_version: draft.base_version,
            enabled: true,
            initial_price: draft.initial_price,
            config: draft.config,
            reason: None,
        },
        reason.clone(),
    )
    .await?;
    mark_new_coin_market_published(&mut tx, project_id, result.version).await?;
    let response = NewCoinMarketPublishResponse {
        revision: draft.revision,
        published_version: result.version,
        replayed: false,
    };
    insert_admin_audit_log_entry_in_tx(
        &mut tx,
        admin_id,
        AdminAuditLogEntry {
            action: "new_coin.market_draft.publish",
            target_type: "new_coin_project",
            target_id: project_id,
            before_json: Some(before),
            after_json: Some(json!(response)),
            reason: Some(reason),
        },
    )
    .await?;
    guard.ensure_owned().await?;
    tx.commit().await?;
    Ok(response)
}

fn next_revision(current: u32, expected: u32) -> AppResult<u32> {
    if current != expected {
        return Err(AppError::Conflict(
            "行情草稿已更新，请重新加载后重试".into(),
        ));
    }
    current
        .checked_add(1)
        .ok_or_else(|| AppError::Conflict("行情草稿修订已达到上限".into()))
}

fn validate_project_pair(
    project: &NewCoinProjectResponse,
    pair: &AdminTradingPairResponse,
) -> AppResult<()> {
    if project.status != "active" {
        return Err(AppError::Validation(
            "项目已停用，不能配置或发布行情".into(),
        ));
    }
    if pair.base_asset_id != project.asset_id || Some(pair.quote_asset_id) != project.quote_asset_id
    {
        return Err(AppError::Validation(
            "行情交易对必须与项目发行资产和计价资产一致".into(),
        ));
    }
    if !matches!(pair.market_type.as_str(), "strategy" | "internal") {
        return Err(AppError::Validation(
            "外部真实行情不能配置为平台生成行情".into(),
        ));
    }
    Ok(())
}
