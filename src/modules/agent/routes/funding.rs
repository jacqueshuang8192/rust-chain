//! 代理充提只读路由，范围完全由令牌对应代理派生。
use crate::{
    error::AppResult,
    modules::{
        agent::{
            application::funding::list_records,
            presentation::funding::{AgentFundingPage, AgentFundingQuery},
        },
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
    Router::new().route("/funding/:kind", get(records))
}
async fn records(
    AgentAuth(claims): AgentAuth,
    State(state): State<AppState>,
    Path(kind): Path<String>,
    Query(query): Query<AgentFundingQuery>,
) -> AppResult<Json<AgentFundingPage>> {
    Ok(Json(
        list_records(state.mysql, &claims.sub, &kind, query).await?,
    ))
}
