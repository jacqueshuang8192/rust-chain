use super::*;

#[derive(ToSchema)]
pub(super) struct HomeCarouselTarget {
    #[schema(rename = "type", pattern = "^(news_detail|news_center|referrals)$")]
    r#type: String,
    #[schema(minimum = 1, maximum = 9007199254740991u64)]
    id: Option<u64>,
}

#[derive(ToSchema)]
pub(super) struct HomeCarouselSlide {
    #[schema(min_length = 1, max_length = 64, pattern = "^[A-Za-z0-9_-]+$")]
    id: String,
    #[schema(minimum = 0, maximum = 11)]
    position: u8,
    enabled: bool,
    locale: Option<String>,
    #[schema(min_length = 1, max_length = 80)]
    title: String,
    #[schema(min_length = 1, max_length = 180)]
    description: String,
    #[schema(max_length = 2048)]
    image_url: String,
    target: HomeCarouselTarget,
}

#[derive(ToSchema)]
pub(super) struct PublicHomeCarouselResponse {
    #[schema(max_items = 12)]
    slides: Vec<HomeCarouselSlide>,
}

#[derive(ToSchema)]
pub(super) struct AdminHomeCarouselResponse {
    revision: u64,
    #[schema(max_items = 12)]
    slides: Vec<HomeCarouselSlide>,
}

#[derive(ToSchema)]
pub(super) struct SaveHomeCarouselRequest {
    expected_revision: u64,
    #[schema(max_items = 12)]
    slides: Vec<HomeCarouselSlide>,
    #[schema(min_length = 1, max_length = 512)]
    reason: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/home/carousel",
    tag = "home-carousel",
    summary = "查询首页公开轮播",
    params(("locale" = Option<String>, Query, description = "语言代码")),
    responses(
        (status = 200, description = "查询成功", body = PublicHomeCarouselResponse),
        (status = 400, description = "语言参数错误", body = ErrorResponse),
        (status = 500, description = "服务内部错误", body = ErrorResponse)
    )
)]
fn get_public_home_carousel() {}

#[utoipa::path(
    get,
    path = "/admin/api/v1/home/carousel",
    tag = "home-carousel",
    summary = "查询后台首页轮播配置",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "查询成功", body = AdminHomeCarouselResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 500, description = "服务内部错误", body = ErrorResponse)
    )
)]
fn get_admin_home_carousel() {}

#[utoipa::path(
    patch,
    path = "/admin/api/v1/home/carousel",
    tag = "home-carousel",
    summary = "保存后台首页轮播配置",
    request_body = SaveHomeCarouselRequest,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "保存成功", body = AdminHomeCarouselResponse),
        (status = 400, description = "参数错误", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限不足", body = ErrorResponse),
        (status = 409, description = "版本冲突", body = ErrorResponse),
        (status = 500, description = "服务内部错误", body = ErrorResponse)
    )
)]
fn save_admin_home_carousel() {}
