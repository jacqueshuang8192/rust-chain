//! 钱包展示估值与现货准入分离；只有已入账三桶余额参与资产总额，估值不产生交易权限。
use crate::{
    error::AppResult,
    modules::{
        market::infrastructure::cache::latest::load_latest_price,
        new_coin::infrastructure::issuance::{self as store, trading},
        wallet::presentation::{IssuanceValuation, WalletAccountResponse},
    },
};
use bigdecimal::{BigDecimal, RoundingMode};
use chrono::Utc;
use redis::aio::ConnectionManager;
use sqlx::{MySql, Pool};

/// 申购/派发按固定申购价，认购/上市按有效平台行情；计价币按新鲜 USDT 行情换算。
/// 缺价、停机或未知来源只令对应资产价格为空；不以发行价代替实时行情，也不修改余额。
pub(super) async fn enrich(
    pool: &Pool<MySql>,
    redis: Option<&ConnectionManager>,
    user: u64,
    accounts: &mut [WalletAccountResponse],
) -> AppResult<()> {
    let projects = store::wallet_projects(pool, user).await?;
    for p in projects {
        let Some(account) = accounts.iter_mut().find(|a| a.asset_id == p.asset_id) else {
            continue;
        };
        let now = Utc::now();
        let mut expires = now + chrono::Duration::seconds(60);
        let mut tx = pool.begin().await?;
        let price = if p.status != "active" || p.policy().is_err() {
            None
        } else if matches!(p.lifecycle_status.as_str(), "subscription" | "distribution") {
            Some(p.issue_price.clone())
        } else if matches!(p.lifecycle_status.as_str(), "purchase" | "listed") {
            if let Ok(pair) = trading::pair_symbol(&mut tx, &p).await {
                match trading::price(redis, &pair, now).await {
                    Ok(price)
                        if trading::engine_ready(&mut tx, &p, &price.source)
                            .await
                            .is_ok() =>
                    {
                        expires = expires.min(price.observed_at + chrono::Duration::seconds(60));
                        Some(price.price)
                    }
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };
        let quote = trading::quote_symbol(&mut tx, p.quote()?).await?;
        tx.commit().await?;
        let rate = if quote == "USDT" {
            Some(BigDecimal::from(1))
        } else if let Some(redis) = redis {
            match load_latest_price(redis, &format!("{quote}USDT")).await {
                Ok(Some(rate))
                    if (0..=60_000).contains(
                        &now.signed_duration_since(rate.observed_at)
                            .num_milliseconds(),
                    ) && rate.source != "unknown" =>
                {
                    expires = expires.min(rate.observed_at + chrono::Duration::seconds(60));
                    Some(rate.price)
                }
                _ => None,
            }
        } else {
            None
        };
        let price_usdt = price
            .zip(rate)
            .map(|(price, rate)| (price * rate).with_scale_round(18, RoundingMode::Down))
            .filter(|p| {
                p > &BigDecimal::from(0)
                    && crate::numeric::ensure_amount_storage(p, "资产估值价格").is_ok()
            });
        account.issuance_valuation = Some(IssuanceValuation {
            price_usdt,
            stage: p.lifecycle_status,
            expires_at: expires.timestamp_millis(),
        });
    }
    Ok(())
}
