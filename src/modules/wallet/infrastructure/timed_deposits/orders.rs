//! 网络级串行分配与永久保留的订单归属；到期不依赖清理任务才能释放。

use super::lock_network;
use crate::{
    error::{AppError, AppResult},
    modules::wallet::{
        amount_fits_asset_precision, deposit_net_credit_amount,
        domain::timed_deposits::RESERVATION_SECONDS, presentation::timed_deposits::*,
        truncate_amount_to_asset_precision,
    },
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{MySql, Pool, QueryBuilder};

const ORDER_SELECT: &str = "SELECT o.asset_kind,o.token_address,o.id,o.order_no,o.user_id,o.asset_symbol,o.network,o.address,o.amount,o.fee_amount,o.paid_amount,o.required_confirmations,CASE WHEN e.status='reversed' THEN 'reversed' WHEN e.status='manual_review' THEN 'review' WHEN o.status='pending' AND o.expires_at<=CURRENT_TIMESTAMP(6) THEN 'expired' ELSE o.status END AS status,e.tx_hash,o.created_at,o.expires_at,o.paid_at,CURRENT_TIMESTAMP(6) AS server_time FROM wallet_deposit_orders o LEFT JOIN wallet_deposit_events e ON e.id=o.deposit_event_id";

#[derive(sqlx::FromRow)]
struct Rule {
    id: u64,
    asset_id: u64,
    asset_symbol: String,
    asset_kind: String,
    token_address: String,
    identity_key: String,
    decimals: i32,
    precision_scale: i32,
    min_deposit_amount: BigDecimal,
    deposit_fee: BigDecimal,
    confirmations: u32,
}

/// 仅展示配置齐全且监听近期成功的资产网络；链资产映射和平台币种开关都必须满足。
pub(crate) async fn options(pool: &Pool<MySql>) -> AppResult<Vec<DepositOption>> {
    Ok(sqlx::query_as("SELECT c.asset_symbol,c.network,CASE c.network WHEN 'eth' THEN 'Ethereum' WHEN 'base' THEN 'Base' WHEN 'btc' THEN 'Bitcoin' WHEN 'tron' THEN 'Tron' ELSE 'Solana' END AS display_name,LEAST(a.precision_scale,c.decimals) AS precision_scale,a.min_deposit_amount,a.deposit_fee,s.confirmations AS required_confirmations FROM wallet_deposit_chain_assets c JOIN assets a ON a.id=c.asset_id JOIN wallet_deposit_scanners s ON s.network=c.network WHERE c.enabled=TRUE AND s.enabled=TRUE AND s.endpoint_encrypted IS NOT NULL AND s.last_error IS NULL AND s.last_scan_at>DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 120 SECOND) AND a.status='active' AND a.deposit_enabled=TRUE AND EXISTS(SELECT 1 FROM wallet_deposit_shared_addresses p WHERE p.network=c.network AND p.enabled=TRUE) ORDER BY a.symbol,c.network")
    .fetch_all(pool).await?)
}

/// 原子创建精确金额订单：先锁网络再校验幂等、链资产和库存；同金额满池返回冲突，绝不添加尾数。
/// 订单保存配置与费用快照，三十分钟内即使已付款也不把同组合提前分给他人；过期库存可立即重用。
pub(crate) async fn create(
    pool: &Pool<MySql>,
    user_id: u64,
    request: &CreateDepositOrder,
) -> AppResult<DepositOrder> {
    let mut tx = pool.begin().await?;
    let now = lock_network(&mut tx, &request.network).await?;
    sqlx::query("SELECT id FROM users WHERE id=? FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(AppError::NotFound)?;
    let replay=sqlx::query_as::<_,(u64,String,String,BigDecimal)>("SELECT id,network,asset_symbol,amount FROM wallet_deposit_orders WHERE user_id=? AND request_key=SHA2(?,256) FOR UPDATE").bind(user_id).bind(&request.idempotency_key).fetch_optional(&mut *tx).await?;
    if let Some((id, network, symbol, amount)) = replay {
        if network != request.network || symbol != request.asset_symbol || amount != request.amount
        {
            return Err(AppError::Conflict(
                "deposit request key was used with different parameters".into(),
            ));
        }
        tx.commit().await?;
        return get(pool, Some(user_id), id).await;
    }
    let rule=sqlx::query_as::<_,Rule>("SELECT c.id,c.asset_id,c.asset_symbol,c.asset_kind,c.token_address,c.identity_key,c.decimals,a.precision_scale,a.min_deposit_amount,a.deposit_fee,s.confirmations FROM wallet_deposit_chain_assets c JOIN assets a ON a.id=c.asset_id JOIN wallet_deposit_scanners s ON s.network=c.network WHERE c.network=? AND c.asset_symbol=? AND c.enabled=TRUE AND a.status='active' AND a.deposit_enabled=TRUE AND s.enabled=TRUE AND s.endpoint_encrypted IS NOT NULL AND s.last_error IS NULL AND s.last_scan_at>DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 120 SECOND) FOR SHARE").bind(&request.network).bind(&request.asset_symbol).fetch_optional(&mut *tx).await?.ok_or_else(||AppError::Validation("deposit network is not configured or scanner is unavailable".into()))?;
    let precision = rule.precision_scale.min(rule.decimals);
    if request.amount <= 0
        || !amount_fits_asset_precision(&request.amount, precision)
        || request.amount < rule.min_deposit_amount
    {
        return Err(AppError::Validation(
            "deposit amount is below minimum or exceeds asset precision".into(),
        ));
    }
    let fee = truncate_amount_to_asset_precision(&rule.deposit_fee, rule.precision_scale);
    deposit_net_credit_amount(&request.amount, &fee, rule.precision_scale)
        .map_err(AppError::Validation)?;
    let candidate=sqlx::query_as::<_,(u64,String,String)>("SELECT p.pool_id,p.address,p.address_key FROM wallet_deposit_shared_addresses p WHERE p.network=? AND p.enabled=TRUE AND NOT EXISTS(SELECT 1 FROM wallet_deposit_orders o WHERE o.network=p.network AND o.address_key=p.address_key AND o.identity_key=? AND o.amount=? AND o.expires_at>?) ORDER BY p.pool_id LIMIT 1 FOR UPDATE")
        .bind(&request.network).bind(&rule.identity_key).bind(&request.amount).bind(now).fetch_optional(&mut *tx).await?
        .ok_or_else(||AppError::Conflict("all addresses for this deposit amount are reserved; retry later".into()))?;
    let expires = crate::time::checked_expiry(now, RESERVATION_SECONDS as u64, "deposit expiry")?;
    let order_no = format!("CD{}", uuid::Uuid::now_v7().simple());
    let id=sqlx::query("INSERT INTO wallet_deposit_orders (order_no,user_id,idempotency_key,asset_id,asset_symbol,chain_asset_id,network,asset_kind,token_address,identity_key,decimals,precision_scale,pool_id,address,address_key,amount,fee_amount,required_confirmations,created_at,expires_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)")
        .bind(order_no).bind(user_id).bind(&request.idempotency_key).bind(rule.asset_id).bind(rule.asset_symbol).bind(rule.id).bind(&request.network).bind(rule.asset_kind).bind(rule.token_address).bind(rule.identity_key).bind(rule.decimals).bind(rule.precision_scale).bind(candidate.0).bind(candidate.1).bind(candidate.2).bind(&request.amount).bind(fee).bind(rule.confirmations).bind(now).bind(expires).execute(&mut *tx).await?.last_insert_id();
    tx.commit().await?;
    get(pool, Some(user_id), id).await
}

/// 读取本人订单或经过管理员鉴权的订单，任何用户越权都统一返回不存在。
pub(crate) async fn get(
    pool: &Pool<MySql>,
    user_id: Option<u64>,
    id: u64,
) -> AppResult<DepositOrder> {
    sqlx::query_as(&format!(
        "{ORDER_SELECT} WHERE o.id=? AND (? IS NULL OR o.user_id=?)"
    ))
    .bind(id)
    .bind(user_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

/// 分页返回订单的服务端状态，过期在读取时计算，绝不依赖客户端倒计时改变资金归属。
pub(crate) async fn list(
    pool: &Pool<MySql>,
    user_id: Option<u64>,
    query: &DepositPageQuery,
) -> AppResult<DepositPage<DepositOrder>> {
    let mut rows = QueryBuilder::<MySql>::new(ORDER_SELECT);
    let mut count = QueryBuilder::<MySql>::new(
        "SELECT COUNT(*) FROM wallet_deposit_orders o LEFT JOIN wallet_deposit_events e ON e.id=o.deposit_event_id",
    );
    for q in [&mut rows, &mut count] {
        q.push(" WHERE 1=1");
        if let Some(id) = user_id {
            q.push(" AND o.user_id=").push_bind(id);
        }
        if let Some(order_no) = &query.order_no {
            q.push(" AND o.order_no=").push_bind(order_no.trim());
        }
        if let Some(network) = &query.network {
            q.push(" AND o.network=").push_bind(network);
        }
        if let Some(asset) = &query.asset_symbol {
            q.push(" AND o.asset_symbol=").push_bind(asset);
        }
        if let Some(status) = &query.status {
            q.push(" AND (CASE WHEN e.status='reversed' THEN 'reversed' WHEN e.status='manual_review' THEN 'review' WHEN o.status='pending' AND o.expires_at<=CURRENT_TIMESTAMP(6) THEN 'expired' ELSE o.status END)=").push_bind(status);
        }
    }
    rows.push(" ORDER BY o.id DESC LIMIT ")
        .push_bind(query.limit.unwrap_or(20).clamp(1, 100))
        .push(" OFFSET ")
        .push_bind(query.offset.unwrap_or(0));
    Ok(DepositPage {
        items: rows.build_query_as().fetch_all(pool).await?,
        total: count.build_query_scalar().fetch_one(pool).await?,
    })
}

/// 扫描前保留历史过期状态，便于后台检索；只变更订单标签，不删除组合历史和链上证据。
pub(crate) async fn expire(pool: &Pool<MySql>) -> AppResult<()> {
    sqlx::query("UPDATE wallet_deposit_orders SET status='expired' WHERE status='pending' AND expires_at<=CURRENT_TIMESTAMP(6) LIMIT 1000").execute(pool).await?;
    Ok(())
}

/// 监听器只读获取所有曾纳入池的地址；停用后仍继续核对历史付款，防止迟到款静默丢失。
pub(crate) async fn watched_addresses(pool: &Pool<MySql>, network: &str) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT address FROM wallet_deposit_shared_addresses WHERE network=? ORDER BY pool_id",
    )
    .bind(network)
    .fetch_all(pool)
    .await?)
}

/// 返回扫描器需要保留的最早有效窗口，供首次监听定位历史扫描起点；没有订单时不编造付款时间。
pub(crate) async fn earliest_order_time(
    pool: &Pool<MySql>,
    network: &str,
) -> AppResult<Option<DateTime<Utc>>> {
    Ok(
        sqlx::query_scalar("SELECT MIN(created_at) FROM wallet_deposit_orders WHERE network=?")
            .bind(network)
            .fetch_one(pool)
            .await?,
    )
}
