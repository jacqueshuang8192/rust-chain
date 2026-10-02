//! 管理员充值审核与免审授权入口；各动作由AdminAuth按独立RBAC权限鉴权。

use crate::{
    error::AppResult,
    modules::{
        admin::service::admin_id_from_subject,
        agent::{application::recharges as app, presentation::recharges::*},
        auth::AdminAuth,
    },
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/agent-recharges", get(list))
        .route("/agent-recharges/:id", get(detail))
        .route("/agent-recharges/:id/review", post(review))
        .route(
            "/agents/:id/recharge-policy",
            get(policy).patch(update_policy),
        )
}

async fn detail(
    _: AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> AppResult<Json<AgentRechargeRecord>> {
    Ok(Json(
        app::get_admin_recharge(state.mysql.clone(), id).await?,
    ))
}

async fn list(
    _: AdminAuth,
    State(state): State<AppState>,
    Query(query): Query<AdminAgentRechargeQuery>,
) -> AppResult<Json<AgentRechargesResponse>> {
    Ok(Json(
        app::list_admin_recharges(state.mysql.clone(), query).await?,
    ))
}

async fn review(
    AdminAuth(claims): AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(request): Json<ReviewAgentRechargeRequest>,
) -> AppResult<Json<AgentRechargeRecord>> {
    Ok(Json(
        app::review_recharge(
            state.mysql.clone(),
            admin_id_from_subject(&claims.sub)?,
            id,
            request,
        )
        .await?,
    ))
}

async fn policy(
    _: AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> AppResult<Json<AgentRechargePolicy>> {
    Ok(Json(
        app::get_recharge_policy(state.mysql.clone(), id).await?,
    ))
}

async fn update_policy(
    AdminAuth(claims): AdminAuth,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(request): Json<UpdateAgentRechargePolicy>,
) -> AppResult<Json<AgentRechargePolicy>> {
    Ok(Json(
        app::update_recharge_policy(
            state.mysql.clone(),
            admin_id_from_subject(&claims.sub)?,
            id,
            request,
        )
        .await?,
    ))
}
