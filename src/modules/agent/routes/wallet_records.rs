//! 代理钱包记录读取入口；仅接受目标用户和分页筛选，不接受可见代理范围。

use crate::{
    error::AppResult,
    modules::{
        agent::{
            application::wallet_records::list_records,
            presentation::wallet_records::{AgentWalletRecordsQuery, AgentWalletRecordsResponse},
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

/// 挂载团队用户只读账变查询，不注册任何钱包修改或流水写入动作。
pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/users/:user_id/wallet-ledger", get(records))
}

/// 从代理令牌派生范围，返回用户钱包变动与业务流水共用的不可变账本记录。
async fn records(
    AgentAuth(claims): AgentAuth,
    State(state): State<AppState>,
    Path(user_id): Path<u64>,
    Query(query): Query<AgentWalletRecordsQuery>,
) -> AppResult<Json<AgentWalletRecordsResponse>> {
    Ok(Json(
        list_records(state.mysql, &claims.sub, user_id, query).await?,
    ))
}
