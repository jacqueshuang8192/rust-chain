//! 项目行情入口仅转发用例；统一 AdminAuth 同时校验项目和交易对权限。

use super::*;
use crate::modules::admin::{
    application::{
        get_new_coin_market_setup, publish_new_coin_market_setup, save_new_coin_market_setup,
    },
    presentation::{
        NewCoinMarketDraftResponse, NewCoinMarketPublishResponse, NewCoinMarketSetupResponse,
        PublishNewCoinMarketRequest, SaveNewCoinMarketDraftRequest,
    },
};

/// 草稿保存与明确发布分开，不提供隐式启用交易对或批量修改价格的命令。
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/new-coins/:id/market-setup", get(read).patch(save))
        .route("/new-coins/:id/market-setup/publish", post(publish))
}

async fn read(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> AppResult<Json<NewCoinMarketSetupResponse>> {
    Ok(Json(
        get_new_coin_market_setup(state.mysql.clone(), id).await?,
    ))
}

async fn save(
    AdminAuth(claims): AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(request): Json<SaveNewCoinMarketDraftRequest>,
) -> AppResult<Json<NewCoinMarketDraftResponse>> {
    Ok(Json(
        save_new_coin_market_setup(
            state.mysql.clone(),
            admin_id_from_subject(&claims.sub)?,
            id,
            request,
        )
        .await?,
    ))
}

async fn publish(
    AdminAuth(claims): AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(request): Json<PublishNewCoinMarketRequest>,
) -> AppResult<Json<NewCoinMarketPublishResponse>> {
    Ok(Json(
        publish_new_coin_market_setup(
            state.mysql.clone(),
            admin_id_from_subject(&claims.sub)?,
            id,
            request,
        )
        .await?,
    ))
}
