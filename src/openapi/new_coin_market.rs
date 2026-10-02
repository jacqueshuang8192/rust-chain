//! 新币项目行情草稿与发布接口文档；行情参数使用十进制字符串，保存和发布具有独立副作用。

use utoipa::{OpenApi, ToSchema};

#[derive(ToSchema)]
struct NewCoinMarketFollow {
    reference_pair_id: u64,
    multiplier: String,
    max_move_ratio: String,
    stale_after_seconds: u32,
}

#[derive(ToSchema)]
struct NewCoinMarketParameters {
    mode: String,
    follow: Option<NewCoinMarketFollow>,
    volatility: String,
    mean_reversion: String,
    price_min: Option<String>,
    price_max: Option<String>,
    volume_min: String,
    volume_max: String,
    wick_strength: String,
    depth_levels: u32,
}

#[derive(ToSchema)]
struct NewCoinMarketPair {
    id: u64,
    symbol: String,
    status: String,
    market_type: String,
    price_precision: i32,
    qty_precision: i32,
}

#[derive(ToSchema)]
struct NewCoinMarketDraft {
    pair_id: u64,
    revision: u32,
    base_version: u32,
    initial_price: Option<String>,
    config: NewCoinMarketParameters,
    published_version: Option<u32>,
    /// Unix 毫秒时间戳。
    updated_at: i64,
}

#[derive(ToSchema)]
struct NewCoinMarketSetup {
    project_id: u64,
    pairs: Vec<NewCoinMarketPair>,
    draft: Option<NewCoinMarketDraft>,
}

#[derive(ToSchema)]
struct SaveNewCoinMarketDraft {
    pair_id: u64,
    expected_revision: u32,
    expected_market_version: u32,
    initial_price: Option<String>,
    config: NewCoinMarketParameters,
    reason: String,
}

#[derive(ToSchema)]
struct PublishNewCoinMarket {
    pair_id: u64,
    expected_revision: u32,
    reason: String,
}

#[derive(ToSchema)]
struct NewCoinMarketPublication {
    revision: u32,
    published_version: u32,
    replayed: bool,
}

/// 同时要求 new_coin.projects.read 和 market.pairs.read；只按发行/计价资产 ID 查候选，不建立绑定。
#[utoipa::path(get, path = "/admin/api/v1/new-coins/{id}/market-setup",
    params(("id" = u64, Path)), responses((status = 200, body = NewCoinMarketSetup),
        (status = 401, description = "未认证"), (status = 403, description = "任一读取权限缺失")),
    security(("bearerAuth" = [])), tag = "new-coin-market")]
fn get_setup() {}

/// 同时要求 new_coin.projects.write 和 market.pairs.write；仅保存草稿及审计，不启用行情或交易。
/// 草稿修订与线上版本均须匹配；金额遵守 DECIMAL(38,18) 及交易对精度，不允许静默取整。
#[utoipa::path(patch, path = "/admin/api/v1/new-coins/{id}/market-setup",
    params(("id" = u64, Path)), request_body = SaveNewCoinMarketDraft,
    responses((status = 200, body = NewCoinMarketDraft),
        (status = 400, description = "资产、行情类型或参数不合法"),
        (status = 403, description = "任一写入权限缺失"),
        (status = 409, description = "草稿或线上版本已变化")),
    security(("bearerAuth" = [])), tag = "new-coin-market")]
fn save_draft() {}

/// 同时要求两项写入权限；共享生成锁内原子更新默认生成器、不可变版本、审计及草稿发布凭证。
/// 同修订重放返回原版本，不恢复已暂停行情；不启用交易对，不改变上市/购买状态或历史价格。
#[utoipa::path(post, path = "/admin/api/v1/new-coins/{id}/market-setup/publish",
    params(("id" = u64, Path)), request_body = PublishNewCoinMarket,
    responses((status = 200, body = NewCoinMarketPublication),
        (status = 400, description = "项目/交易对不可发布或参数无效"),
        (status = 403, description = "任一写入权限缺失"),
        (status = 409, description = "草稿修订、线上版本或行情所有权冲突")),
    security(("bearerAuth" = [])), tag = "new-coin-market")]
fn publish_draft() {}

#[derive(OpenApi)]
#[openapi(paths(get_setup, save_draft, publish_draft),
    components(schemas(NewCoinMarketFollow, NewCoinMarketParameters, NewCoinMarketPair,
        NewCoinMarketDraft, NewCoinMarketSetup, SaveNewCoinMarketDraft,
        PublishNewCoinMarket, NewCoinMarketPublication)),
    tags((name = "new-coin-market", description = "新币行情草稿与显式发布")))]
pub(super) struct NewCoinMarketApiDoc;
