//! 管理后台法律文件发布、历史复核和用户确认凭据入口；所有路由经管理员权限提取器。

use crate::{
    error::AppResult,
    modules::{
        admin::service::admin_id_from_subject,
        auth::{
            AdminAuth,
            application::{legal as app, mysql_pool},
            presentation::legal::{AdminLegalQuery, PublishRegistrationLegalRequest},
        },
    },
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};

/// 后台精确范围发布入口及历史/确认凭据读取，正文发布须写明操作原因。
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/legal-documents", get(list).post(publish))
        .route("/legal-documents/publications/:id", get(publication))
        .route("/legal-documents/receipts/:user_id", get(receipt))
}

async fn list(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Query(query): Query<AdminLegalQuery>,
) -> AppResult<Json<crate::modules::auth::presentation::legal::AdminLegalResponse>> {
    Ok(Json(
        app::admin_registration_legal(&mysql_pool(&state)?, &query.scope_code, &query.locale)
            .await?,
    ))
}

async fn publish(
    AdminAuth(claims): AdminAuth,
    State(state): State<AppState>,
    Json(request): Json<PublishRegistrationLegalRequest>,
) -> AppResult<Json<crate::modules::auth::presentation::legal::RegistrationLegalBundle>> {
    Ok(Json(
        app::publish_registration_legal(
            &mysql_pool(&state)?,
            admin_id_from_subject(&claims.sub)?,
            request,
        )
        .await?,
    ))
}

async fn publication(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> AppResult<Json<crate::modules::auth::presentation::legal::RegistrationLegalBundle>> {
    Ok(Json(
        app::admin_legal_publication_by_id(&mysql_pool(&state)?, id).await?,
    ))
}

async fn receipt(
    _auth: AdminAuth,
    State(state): State<AppState>,
    Path(user_id): Path<u64>,
) -> AppResult<Json<crate::modules::auth::presentation::legal::UserRegistrationLegalReceipt>> {
    Ok(Json(
        app::admin_legal_receipt(&mysql_pool(&state)?, user_id).await?,
    ))
}
