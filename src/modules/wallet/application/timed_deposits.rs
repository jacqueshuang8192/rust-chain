//! 限时充值用例：校验输入、保留认证身份，并编排持久化和只读链监听。

use crate::{
    error::{AppError, AppResult},
    modules::wallet::{
        domain::timed_deposits::canonical_address, infrastructure::timed_deposits as storage,
        presentation::timed_deposits::*,
    },
};
use sqlx::{MySql, Pool};

/// 创建订单只接受认证用户和精确十进制币种数量；幂等键不能包含空白或超过边界。
pub async fn create_order(
    pool: &Pool<MySql>,
    user_id: u64,
    mut request: CreateDepositOrder,
) -> AppResult<DepositOrder> {
    request.network = super::normalize_deposit_network(&request.network)?;
    request.asset_symbol = super::normalize_asset_symbol(&request.asset_symbol)?;
    request.idempotency_key = request.idempotency_key.trim().to_owned();
    if request.idempotency_key.is_empty()
        || request.idempotency_key.len() > 128
        || !request
            .idempotency_key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(AppError::Validation(
            "invalid deposit idempotency key".into(),
        ));
    }
    crate::numeric::ensure_amount_storage(&request.amount, "deposit amount")?;
    storage::orders::create(pool, user_id, &request).await
}

/// 查询限时充值准入选项，不返回未完成监听配置的币种和网络。
pub async fn options(pool: &Pool<MySql>) -> AppResult<Vec<DepositOption>> {
    storage::orders::options(pool).await
}

/// 用户接口必须传本人编号；仅后台入口允许不限制所有人，不存在客户端传用户编号的旁路。
pub async fn get_order(
    pool: &Pool<MySql>,
    user_id: Option<u64>,
    id: u64,
) -> AppResult<DepositOrder> {
    storage::orders::get(pool, user_id, id).await
}

/// 列出经过身份范围约束的充值订单；分页参数在存储边界统一设上限。
pub async fn list_orders(
    pool: &Pool<MySql>,
    user_id: Option<u64>,
    query: &DepositPageQuery,
) -> AppResult<DepositPage<DepositOrder>> {
    storage::orders::list(pool, user_id, query).await
}

/// 管理端只读全部配置，凭据字段仅暴露配置状态。
pub(crate) async fn settings(pool: &Pool<MySql>) -> AppResult<DepositSettings> {
    storage::settings::load(pool).await
}

/// 节点配置限制协议、长度和确认数并要求审计原因；传入加密密钥只用于封装凭据，不进入日志。
pub(crate) async fn save_scanner(
    pool: &Pool<MySql>,
    admin_id: u64,
    network: &str,
    key: &str,
    mut request: SaveScanner,
) -> AppResult<()> {
    let network = super::normalize_deposit_network(network)?;
    request.reason = reason(request.reason)?;
    if !(1..=10000).contains(&request.confirmations) {
        return Err(AppError::Validation(
            "confirmations must be between 1 and 10000".into(),
        ));
    }
    if let Some(endpoint) = request.endpoint.as_deref().filter(|s| !s.trim().is_empty()) {
        let url = url::Url::parse(endpoint.trim())
            .map_err(|_| AppError::Validation("invalid scanner endpoint".into()))?;
        if endpoint.len() > 2000
            || !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || url.fragment().is_some()
        {
            return Err(AppError::Validation(
                "scanner endpoint must be an HTTP or HTTPS URL".into(),
            ));
        }
    }
    if request
        .api_key
        .as_ref()
        .is_some_and(|s| s.len() > 1024 || s.contains(['\r', '\n']))
    {
        return Err(AppError::Validation("invalid scanner API key".into()));
    }
    storage::settings::save_scanner(pool, admin_id, &network, key, &request).await
}

/// 原生币固定网络和精度；代币必须指定规范合约或 mint，Bitcoin 此版本只支持原生 BTC。
pub(crate) async fn save_asset(
    pool: &Pool<MySql>,
    admin_id: u64,
    mut request: SaveChainAsset,
) -> AppResult<()> {
    request.reason = reason(request.reason)?;
    request.network = super::normalize_deposit_network(&request.network)?;
    request.asset_symbol = super::normalize_asset_symbol(&request.asset_symbol)?;
    if !(0..=18).contains(&request.decimals) {
        return Err(AppError::Validation(
            "chain decimals must be between 0 and 18".into(),
        ));
    }
    match request.asset_kind.as_str() {
        "native" => {
            if request.network == "tron" {
                return Err(AppError::Validation(
                    "Tron currently supports explicitly configured TRC20 assets only".into(),
                ));
            }
            let (symbol, decimals) = match request.network.as_str() {
                "eth" | "base" => ("ETH", 18),
                "tron" => ("TRX", 6),
                "btc" => ("BTC", 8),
                "solana" => ("SOL", 9),
                _ => unreachable!(),
            };
            if request.asset_symbol != symbol
                || request.decimals != decimals
                || !request.token_address.trim().is_empty()
            {
                return Err(AppError::Validation(
                    "native asset symbol, decimals or empty contract is incorrect".into(),
                ));
            }
            request.token_address = String::new();
        }
        "token" if request.network != "btc" => {
            request.token_address = canonical_address(&request.network, &request.token_address)
                .map_err(AppError::Validation)?;
        }
        _ => {
            return Err(AppError::Validation(
                "unsupported on-chain asset kind".into(),
            ));
        }
    }
    storage::settings::save_asset(pool, admin_id, &request).await
}

/// 按网络规范完整地址批次后原子导入；去重不改变已有地址的启停状态，无永久用户绑定。
pub(crate) async fn import_addresses(
    pool: &Pool<MySql>,
    admin_id: u64,
    mut request: ImportSharedAddresses,
) -> AppResult<()> {
    request.network = super::normalize_deposit_network(&request.network)?;
    request.reason = reason(request.reason)?;
    if request.addresses.is_empty() || request.addresses.len() > 500 {
        return Err(AppError::Validation(
            "provide between 1 and 500 addresses".into(),
        ));
    }
    request.addresses = request
        .addresses
        .iter()
        .map(|address| canonical_address(&request.network, address).map_err(AppError::Validation))
        .collect::<AppResult<std::collections::BTreeSet<_>>>()?
        .into_iter()
        .collect();
    storage::settings::import_addresses(pool, admin_id, &request).await
}

/// 地址管理权限只读取收款地址，不借用网络配置权限或旧库存 API。
pub(crate) async fn addresses(pool: &Pool<MySql>) -> AppResult<Vec<SharedAddressView>> {
    storage::settings::addresses(pool).await
}

/// 仅切换已存在收款地址的准入状态；配置保存和审计由同一存储事务负责。
pub(crate) async fn save_address(
    pool: &Pool<MySql>,
    admin_id: u64,
    pool_id: u64,
    mut request: SaveSharedAddress,
) -> AppResult<()> {
    request.reason = reason(request.reason)?;
    storage::settings::save_address(pool, admin_id, pool_id, &request).await
}

/// 后台查询未匹配与已处理证据，保持原始交易哈希和实际币种数量。
pub(crate) async fn receipts(
    pool: &Pool<MySql>,
    query: &DepositPageQuery,
) -> AppResult<DepositPage<ReceiptView>> {
    storage::settlement::list(pool, query).await
}

/// 人工核对必须明确选择入账或排除并填写依据，不能通过任意状态字符串触发资金变更。
pub(crate) async fn review(
    pool: &Pool<MySql>,
    admin_id: u64,
    id: u64,
    mut request: ReviewDepositReceipt,
) -> AppResult<()> {
    request.reason = reason(request.reason)?;
    if !matches!(request.action.as_str(), "credit" | "ignore")
        || (request.action == "ignore" && request.order_id.is_some())
    {
        return Err(AppError::Validation(
            "invalid deposit receipt review action".into(),
        ));
    }
    storage::settlement::review(pool, admin_id, id, &request).await
}

fn reason(value: String) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 500 {
        return Err(AppError::Validation(
            "a review reason of 1 to 500 characters is required".into(),
        ));
    }
    Ok(value.to_owned())
}

/// 执行一次只读链监听和过期标签维护，单网络故障由扫描器隔离，不影响其他网络入账。
pub async fn scan_once(pool: &Pool<MySql>, key: Option<&str>) -> AppResult<u64> {
    storage::orders::expire(pool).await?;
    storage::scanners::run_once(pool, key).await
}
