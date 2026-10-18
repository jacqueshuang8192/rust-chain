//! 代理发行阶段及选中批次派发入口；归属在应用层事务内重新验证。
use crate::{
    error::AppResult,
    modules::{
        auth::{AgentAuth, claims_auth_session_version},
        new_coin::{
            application::issuance as app,
            presentation::issuance::{AllocationRequest, ProjectsQuery, StageRequest},
        },
    },
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde_json::{Value, json};
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/token-projects", get(projects))
        .route("/token-projects/:id/stage", post(stage))
        .route("/new-coin-allocations", post(allocate))
}
async fn projects(
    AgentAuth(c): AgentAuth,
    State(s): State<AppState>,
    Query(query): Query<ProjectsQuery>,
) -> AppResult<Json<Value>> {
    Ok(Json(app::agent_projects(s.mysql, &c.sub, query).await?))
}
async fn stage(
    AgentAuth(c): AgentAuth,
    State(s): State<AppState>,
    Path(id): Path<u64>,
    Json(req): Json<StageRequest>,
) -> AppResult<Json<Value>> {
    Ok(Json(json!(
        app::agent_stage(s.mysql, &c.sub, claims_auth_session_version(&c)?, id, req).await?
    )))
}
async fn allocate(
    AgentAuth(c): AgentAuth,
    State(s): State<AppState>,
    Json(req): Json<AllocationRequest>,
) -> AppResult<Json<Value>> {
    Ok(Json(
        app::allocate(s.mysql, &c.sub, claims_auth_session_version(&c)?, req).await?,
    ))
}
