//! 项目行情草稿侧车存储；调用方持项目锁串行化修改，草稿与线上生成器完全分离。

use super::*;
use crate::modules::admin::presentation::{
    NewCoinMarketDraftResponse, NewCoinMarketPairResponse, SaveNewCoinMarketDraftRequest,
};
use serde_json::json;

#[derive(sqlx::FromRow)]
pub(crate) struct NewCoinMarketDraftRecord {
    pub(crate) pair_id: u64,
    pub(crate) revision: u32,
    pub(crate) base_version: u32,
    pub(crate) initial_price: Option<BigDecimal>,
    pub(crate) config_json: SqlxJson<Value>,
    pub(crate) published_version: Option<u32>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl NewCoinMarketDraftRecord {
    /// 草稿 JSON 必须完整合法；损坏时拒绝显示为可发布的默认配置。
    pub(crate) fn response(self) -> AppResult<NewCoinMarketDraftResponse> {
        Ok(NewCoinMarketDraftResponse {
            pair_id: self.pair_id,
            revision: self.revision,
            base_version: self.base_version,
            initial_price: self.initial_price,
            config: serde_json::from_value(self.config_json.0)
                .map_err(|_| AppError::Internal("行情草稿格式异常".into()))?,
            published_version: self.published_version,
            updated_at: self.updated_at,
        })
    }
}

/// 项目锁由写用例持有；只读用例使用事务快照，不创建草稿或绑定交易对。
pub(crate) async fn load_new_coin_market_draft(
    tx: &mut Transaction<'_, MySql>,
    project_id: u64,
) -> AppResult<Option<NewCoinMarketDraftRecord>> {
    Ok(sqlx::query_as("SELECT pair_id, revision, base_version, initial_price, config_json, published_version, updated_at FROM new_coin_market_drafts WHERE project_id=?")
        .bind(project_id).fetch_optional(&mut **tx).await?)
}

/// 严格按两个资产 ID 提供候选，保留禁用和外部交易对供运营识别；超过上限明确拒绝而不截断目录。
pub(crate) async fn new_coin_market_pairs(
    tx: &mut Transaction<'_, MySql>,
    base: u64,
    quote: Option<u64>,
) -> AppResult<Vec<NewCoinMarketPairResponse>> {
    let pairs: Vec<NewCoinMarketPairResponse> = sqlx::query_as("SELECT id, symbol, status, market_type, price_precision, qty_precision FROM trading_pairs WHERE base_asset=? AND quote_asset=? ORDER BY id LIMIT 101")
        .bind(base).bind(quote).fetch_all(&mut **tx).await?;
    if pairs.len() > 100 {
        return Err(AppError::Validation(
            "项目候选交易对超过上限，请先整理交易对配置".into(),
        ));
    }
    Ok(pairs)
}

/// 在项目锁内覆盖草稿并清除上版发布凭证；不修改当前生成器、交易开关或购买关系。
pub(crate) async fn save_new_coin_market_draft(
    tx: &mut Transaction<'_, MySql>,
    project_id: u64,
    admin_id: u64,
    revision: u32,
    request: &SaveNewCoinMarketDraftRequest,
) -> AppResult<()> {
    sqlx::query("INSERT INTO new_coin_market_drafts (project_id,pair_id,revision,base_version,initial_price,config_json,updated_by) VALUES (?,?,?,?,?,?,?) ON DUPLICATE KEY UPDATE pair_id=VALUES(pair_id),revision=VALUES(revision),base_version=VALUES(base_version),initial_price=VALUES(initial_price),config_json=VALUES(config_json),published_version=NULL,updated_by=VALUES(updated_by),updated_at=UTC_TIMESTAMP(3)")
        .bind(project_id).bind(request.pair_id).bind(revision).bind(request.expected_market_version)
        .bind(&request.initial_price).bind(SqlxJson(json!(request.config))).bind(admin_id)
        .execute(&mut **tx).await?;
    Ok(())
}

/// 发布凭证和实际生成器版本共用调用方事务；同一草稿重放仅返回凭证，不能再次套用旧参数。
pub(crate) async fn mark_new_coin_market_published(
    tx: &mut Transaction<'_, MySql>,
    project_id: u64,
    version: u32,
) -> AppResult<()> {
    sqlx::query("UPDATE new_coin_market_drafts SET published_version=? WHERE project_id=?")
        .bind(version)
        .bind(project_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
