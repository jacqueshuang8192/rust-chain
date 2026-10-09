//! 团队订单只读路由，动态路径只接受应用层固定的四个业务枚举。

use crate::{
    error::AppResult,
    modules::{
        agent::{
            application::team_orders,
            presentation::team_orders::{AgentTeamOrdersQuery, AgentTeamOrdersResponse},
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

/// 代理身份仍从 AgentAuth 读取；业务参数不能选择表名或放大可见范围。
pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/orders/:kind", get(list_orders))
}

/// 透传已鉴权主体与筛选给应用层，处理器不访问数据库或调用金融写入口。
async fn list_orders(
    AgentAuth(claims): AgentAuth,
    State(state): State<AppState>,
    Path(kind): Path<String>,
    Query(query): Query<AgentTeamOrdersQuery>,
) -> AppResult<Json<AgentTeamOrdersResponse>> {
    Ok(Json(
        team_orders::list_orders(state.mysql, &claims.sub, &kind, query).await?,
    ))
}
