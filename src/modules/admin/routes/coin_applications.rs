//! 新币申请传输入口；认证主体由提取器取得，所有事务和归属检查由应用层负责。
use crate::{
    error::AppResult,
    modules::{
        agent::{application::coin_applications as app, presentation::coin_applications::*},
        auth::AdminAuth,
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
        .route("/agent-coin-applications", get(list))
        .route("/agent-coin-applications/:id", get(detail))
        .route(
            "/agent-coin-applications/:id/review",
            axum::routing::post(review),
        )
}
async fn list(
    _: AdminAuth,
    State(s): State<AppState>,
    Query(q): Query<CoinApplicationQuery>,
) -> AppResult<Json<CoinApplicationsResponse>> {
    Ok(Json(app::list_admin(s.mysql, q).await?))
}
async fn detail(
    _: AdminAuth,
    State(s): State<AppState>,
    Path(id): Path<u64>,
) -> AppResult<Json<CoinApplication>> {
    Ok(Json(app::admin_detail(s.mysql, id).await?))
}
async fn review(
    AdminAuth(c): AdminAuth,
    State(s): State<AppState>,
    Path(id): Path<u64>,
    Json(q): Json<CoinApplicationReview>,
) -> AppResult<Json<CoinApplication>> {
    Ok(Json(
        app::review(
            s.mysql,
            crate::modules::admin::service::admin_id_from_subject(&c.sub)?,
            id,
            q,
        )
        .await?,
    ))
}
