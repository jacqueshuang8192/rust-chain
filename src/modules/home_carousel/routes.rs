use super::{
    application::{get_admin_home_carousel, get_public_home_carousel, save_admin_home_carousel},
    presentation::{HomeCarouselQuery, SaveHomeCarouselRequest},
};
use crate::{
    error::AppResult,
    modules::{admin::service::admin_id_from_subject, auth::AdminAuth},
    state::AppState,
};
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};

pub fn public_routes() -> Router<AppState> {
    Router::new().route("/home/carousel", get(get_public_home_carousel_route))
}

pub fn admin_routes() -> Router<AppState> {
    Router::new().route(
        "/home/carousel",
        get(get_admin_home_carousel_route).patch(save_admin_home_carousel_route),
    )
}

async fn get_public_home_carousel_route(
    State(state): State<AppState>,
    Query(query): Query<HomeCarouselQuery>,
) -> AppResult<Json<super::presentation::PublicHomeCarouselResponse>> {
    let pool = state.mysql.clone().ok_or_else(|| {
        crate::error::AppError::Internal(
            "mysql pool is not configured for home carousel".to_owned(),
        )
    })?;
    Ok(Json(get_public_home_carousel(&pool, query.locale).await?))
}

async fn get_admin_home_carousel_route(
    _auth: AdminAuth,
    State(state): State<AppState>,
) -> AppResult<Json<super::presentation::AdminHomeCarouselResponse>> {
    let pool = state.mysql.clone().ok_or_else(|| {
        crate::error::AppError::Internal(
            "mysql pool is not configured for home carousel".to_owned(),
        )
    })?;
    Ok(Json(get_admin_home_carousel(&pool).await?))
}

async fn save_admin_home_carousel_route(
    AdminAuth(claims): AdminAuth,
    State(state): State<AppState>,
    Json(request): Json<SaveHomeCarouselRequest>,
) -> AppResult<Json<super::presentation::AdminHomeCarouselResponse>> {
    let admin_id = admin_id_from_subject(&claims.sub)?;
    let pool = state.mysql.clone().ok_or_else(|| {
        crate::error::AppError::Internal(
            "mysql pool is not configured for home carousel".to_owned(),
        )
    })?;
    Ok(Json(
        save_admin_home_carousel(&pool, admin_id, request).await?,
    ))
}
