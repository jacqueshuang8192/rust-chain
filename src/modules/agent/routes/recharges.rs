//! 代理充值传输入口；不接受客户端agent_id或审核状态，身份来自AgentAuth。

use crate::{
    error::AppResult,
    modules::{
        agent::{application::recharges as app, presentation::recharges::*},
        auth::{AgentAuth, claims_auth_session_version},
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
        .route("/recharges/options", get(options))
        .route("/recharges", get(list))
        .route("/users/:user_id/recharge", post(submit))
}

async fn options(
    AgentAuth(claims): AgentAuth,
    State(state): State<AppState>,
) -> AppResult<Json<AgentRechargeOptions>> {
    Ok(Json(
        app::get_recharge_options(state.mysql.clone(), &claims.sub).await?,
    ))
}

async fn list(
    AgentAuth(claims): AgentAuth,
    State(state): State<AppState>,
    Query(query): Query<AgentRechargeQuery>,
) -> AppResult<Json<AgentRechargesResponse>> {
    Ok(Json(
        app::list_own_recharges(state.mysql.clone(), &claims.sub, query).await?,
    ))
}

async fn submit(
    AgentAuth(claims): AgentAuth,
    State(state): State<AppState>,
    Path(user): Path<u64>,
    Json(request): Json<AgentRechargeRequest>,
) -> AppResult<Json<AgentRechargeRecord>> {
    Ok(Json(
        app::submit_recharge(
            state.mysql.clone(),
            &claims.sub,
            claims_auth_session_version(&claims)?,
            user,
            request,
        )
        .await?,
    ))
}
