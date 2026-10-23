//! 限时充值传输边界：用户归属来自令牌，后台写入复用统一权限中间件。

use crate::{
    error::{AppError, AppResult},
    modules::{
        auth::{AdminAuth, UserAuth},
        user::service::user_id_from_subject,
        wallet::{
            application::{admin_id_from_subject, mysql_pool, timed_deposits as app},
            presentation::timed_deposits::*,
        },
    },
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
};

/// 用户入口显式创建限时订单，金额提交前不会领取永久地址。
pub(super) fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/wallet/deposit-options", get(options))
        .route("/wallet/deposit-orders", get(orders).post(create))
        .route("/wallet/deposit-orders/:id", get(order))
}

/// 配置和人工核对使用钱包域独立权限映射；不开放任何匿名链上观测写入端点。
pub(super) fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/wallet/deposit-settings", get(settings))
        .route(
            "/wallet/deposit-settings/scanners/:network",
            put(save_scanner),
        )
        .route("/wallet/deposit-settings/assets", put(save_asset))
        .route("/wallet/deposit-settings/addresses/:id", put(save_address))
        .route("/wallet/deposit-orders", get(admin_orders))
        .route("/wallet/deposit-orders/:id", get(admin_order))
        .route("/wallet/deposit-receipts", get(receipts))
        .route("/wallet/deposit-receipts/:id/review", post(review))
}

async fn options(
    UserAuth(_): UserAuth,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<DepositOption>>> {
    Ok(Json(app::options(&mysql_pool(&state)?).await?))
}
async fn orders(
    UserAuth(c): UserAuth,
    State(state): State<AppState>,
    Query(q): Query<DepositPageQuery>,
) -> AppResult<Json<DepositPage<DepositOrder>>> {
    Ok(Json(
        app::list_orders(
            &mysql_pool(&state)?,
            Some(user_id_from_subject(&c.sub)?),
            &q,
        )
        .await?,
    ))
}
async fn create(
    UserAuth(c): UserAuth,
    State(state): State<AppState>,
    Json(r): Json<CreateDepositOrder>,
) -> AppResult<Json<DepositOrder>> {
    Ok(Json(
        app::create_order(&mysql_pool(&state)?, user_id_from_subject(&c.sub)?, r).await?,
    ))
}
async fn order(
    UserAuth(c): UserAuth,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> AppResult<Json<DepositOrder>> {
    Ok(Json(
        app::get_order(
            &mysql_pool(&state)?,
            Some(user_id_from_subject(&c.sub)?),
            id,
        )
        .await?,
    ))
}
async fn admin_orders(
    AdminAuth(_): AdminAuth,
    State(state): State<AppState>,
    Query(q): Query<DepositPageQuery>,
) -> AppResult<Json<DepositPage<DepositOrder>>> {
    Ok(Json(
        app::list_orders(&mysql_pool(&state)?, None, &q).await?,
    ))
}
async fn admin_order(
    AdminAuth(_): AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> AppResult<Json<DepositOrder>> {
    Ok(Json(app::get_order(&mysql_pool(&state)?, None, id).await?))
}
async fn settings(
    AdminAuth(_): AdminAuth,
    State(state): State<AppState>,
) -> AppResult<Json<DepositSettings>> {
    Ok(Json(app::settings(&mysql_pool(&state)?).await?))
}
async fn save_scanner(
    AdminAuth(c): AdminAuth,
    State(state): State<AppState>,
    Path(network): Path<String>,
    Json(r): Json<SaveScanner>,
) -> AppResult<StatusCode> {
    let key = state
        .settings
        .exposed_credential_encryption_key()
        .ok_or_else(|| AppError::Validation("credential encryption key is required".into()))?;
    app::save_scanner(
        &mysql_pool(&state)?,
        admin_id_from_subject(&c.sub)?,
        &network,
        key,
        r,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn save_asset(
    AdminAuth(c): AdminAuth,
    State(state): State<AppState>,
    Json(r): Json<SaveChainAsset>,
) -> AppResult<StatusCode> {
    app::save_asset(&mysql_pool(&state)?, admin_id_from_subject(&c.sub)?, r).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn save_address(
    AdminAuth(c): AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(r): Json<SaveSharedAddress>,
) -> AppResult<StatusCode> {
    app::save_address(&mysql_pool(&state)?, admin_id_from_subject(&c.sub)?, id, r).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn receipts(
    AdminAuth(_): AdminAuth,
    State(state): State<AppState>,
    Query(q): Query<DepositPageQuery>,
) -> AppResult<Json<DepositPage<ReceiptView>>> {
    Ok(Json(app::receipts(&mysql_pool(&state)?, &q).await?))
}
async fn review(
    AdminAuth(c): AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(r): Json<ReviewDepositReceipt>,
) -> AppResult<StatusCode> {
    app::review(&mysql_pool(&state)?, admin_id_from_subject(&c.sub)?, id, r).await?;
    Ok(StatusCode::NO_CONTENT)
}
