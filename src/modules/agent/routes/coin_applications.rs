//! 新币申请传输入口；认证主体由提取器取得，所有事务和归属检查由应用层负责。
use crate::{
    error::AppResult,
    modules::{
        agent::{application::coin_applications as app, presentation::coin_applications::*},
        auth::AgentAuth,
    },
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/coin-applications", get(list).post(submit))
        .route("/coin-applications/options", get(options))
        .route("/coin-applications/:id", get(detail))
}
async fn list(
    AgentAuth(c): AgentAuth,
    State(s): State<AppState>,
    Query(q): Query<CoinApplicationQuery>,
) -> AppResult<Json<CoinApplicationsResponse>> {
    Ok(Json(app::list_own(s.mysql, &c.sub, q).await?))
}
async fn detail(
    AgentAuth(c): AgentAuth,
    State(s): State<AppState>,
    Path(id): Path<u64>,
) -> AppResult<Json<CoinApplication>> {
    Ok(Json(app::own_detail(s.mysql, &c.sub, id).await?))
}
async fn submit(
    AgentAuth(c): AgentAuth,
    State(s): State<AppState>,
    Json(q): Json<CoinApplicationRequest>,
) -> AppResult<Json<CoinApplication>> {
    Ok(Json(
        app::submit(
            s.mysql,
            &c.sub,
            crate::modules::auth::claims_auth_session_version(&c)?,
            q,
        )
        .await?,
    ))
}
async fn options(
    AgentAuth(c): AgentAuth,
    State(s): State<AppState>,
) -> AppResult<Json<Vec<crate::modules::agent::presentation::recharges::RechargeAsset>>> {
    Ok(Json(app::options(s.mysql, &c.sub).await?))
}
