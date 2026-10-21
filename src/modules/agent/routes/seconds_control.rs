//! 代理门户秒合约控输控赢路由；订单号与用户号只是资源标识，代理范围仍由令牌决定。

use crate::{
    error::AppResult,
    modules::{
        agent::{
            application::seconds_control as app,
            presentation::{AgentSecondsControlRequest, AgentSecondsUserControlResponse},
        },
        auth::AgentAuth,
        seconds_contract::presentation::SecondsContractControlResponse,
    },
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};

/// 订单级控制只开放 POST，用户级控制同时提供读取与覆盖写，全部要求代理会话。
/// 代理身份不接受请求参数指定，越权由应用层按物化路径子树统一拦截。
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/seconds-contract-orders/:order_id/control",
            post(set_order_control),
        )
        .route(
            "/users/:user_id/seconds-contract-control",
            get(get_user_control).put(set_user_control),
        )
}

/// 对子树内某笔秒合约订单设置必胜、必输或取消控制，返回落库后的控制快照。
async fn set_order_control(
    AgentAuth(claims): AgentAuth,
    State(state): State<AppState>,
    Path(order_id): Path<u64>,
    Json(request): Json<AgentSecondsControlRequest>,
) -> AppResult<Json<SecondsContractControlResponse>> {
    app::validate_order_id(order_id)?;
    Ok(Json(
        app::set_order_control(state.mysql.clone(), &claims.sub, order_id, request).await?,
    ))
}

/// 对子树内某个用户设置默认控制，作用于该用户后续的秒合约订单。
async fn set_user_control(
    AgentAuth(claims): AgentAuth,
    State(state): State<AppState>,
    Path(user_id): Path<u64>,
    Json(request): Json<AgentSecondsControlRequest>,
) -> AppResult<Json<SecondsContractControlResponse>> {
    Ok(Json(
        app::set_user_control(state.mysql.clone(), &claims.sub, user_id, request).await?,
    ))
}

/// 读取子树内某个用户当前生效的控制设置，未设置时返回 `active=false`。
async fn get_user_control(
    AgentAuth(claims): AgentAuth,
    State(state): State<AppState>,
    Path(user_id): Path<u64>,
) -> AppResult<Json<AgentSecondsUserControlResponse>> {
    Ok(Json(
        app::get_user_control(state.mysql.clone(), &claims.sub, user_id).await?,
    ))
}
