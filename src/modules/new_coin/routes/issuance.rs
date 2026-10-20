//! 新币第二版报价、持仓及确认路由；认证用户不由请求体指定。
use crate::{
    error::AppResult,
    modules::{
        auth::UserAuth,
        new_coin::{
            application::issuance as app,
            presentation::issuance::{
                HoldingsQuery, MarketPurchaseRequest, TradeQuote, TradeQuoteRequest,
            },
        },
    },
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use serde_json::Value;
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/new-coins/holdings", get(holdings))
        .route("/new-coins/:symbol/market-purchases", post(market_purchase))
        .route("/new-coins/:symbol/quotes", post(quote))
        .route("/new-coins/quotes/:id/confirm", post(confirm))
}
async fn market_purchase(
    UserAuth(c): UserAuth,
    State(s): State<AppState>,
    Path(symbol): Path<String>,
    Json(req): Json<MarketPurchaseRequest>,
) -> AppResult<Json<Value>> {
    Ok(Json(
        app::market_purchase(s.mysql, s.redis.as_ref(), &c.sub, &symbol, req).await?,
    ))
}
async fn quote(
    UserAuth(c): UserAuth,
    State(s): State<AppState>,
    Path(symbol): Path<String>,
    Json(req): Json<TradeQuoteRequest>,
) -> AppResult<Json<TradeQuote>> {
    Ok(Json(
        app::quote(s.mysql, s.redis.as_ref(), &c.sub, &symbol, req).await?,
    ))
}
async fn confirm(
    UserAuth(c): UserAuth,
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    Ok(Json(app::confirm(s.mysql, &c.sub, &id).await?))
}
async fn holdings(
    UserAuth(c): UserAuth,
    State(s): State<AppState>,
    Query(query): Query<HoldingsQuery>,
) -> AppResult<Json<Value>> {
    Ok(Json(
        app::holdings(s.mysql, s.redis.as_ref(), &c.sub, query).await?,
    ))
}
