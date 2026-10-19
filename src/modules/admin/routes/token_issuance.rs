//! 新版发行管理传输入口；权限由统一RBAC处理，事务与只读聚合均由应用层执行。
use super::*;
use crate::modules::admin::{application::token_issuance as app, presentation::token_issuance::*};
use serde_json::Value;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/new-coins/issuance", post(create))
        .route("/new-coins/holdings", get(holdings))
        .route("/new-coins/buybacks", get(buybacks))
        .route("/new-coins/operations", get(operations))
}
async fn create(
    AdminAuth(claims): AdminAuth,
    State(state): State<AppState>,
    Json(req): Json<CreateTokenIssuanceRequest>,
) -> AppResult<Json<NewCoinProjectResponse>> {
    Ok(Json(
        app::create(
            state.mysql.clone(),
            admin_id_from_subject(&claims.sub)?,
            req,
        )
        .await?,
    ))
}
async fn holdings(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Query(q): Query<AdminTokenRecordsQuery>,
) -> AppResult<Json<Value>> {
    Ok(Json(app::holdings(state.mysql.clone(), q).await?))
}
async fn buybacks(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Query(q): Query<AdminTokenRecordsQuery>,
) -> AppResult<Json<Value>> {
    Ok(Json(app::buybacks(state.mysql.clone(), q).await?))
}
async fn operations(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Query(q): Query<AdminTokenOperationsQuery>,
) -> AppResult<Json<Value>> {
    Ok(Json(app::operations(state.mysql.clone(), q).await?))
}
