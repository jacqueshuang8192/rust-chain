//! 链上证据去重、历史窗口匹配与人工归属裁决；订单、账务和审计在同一事务提交。

use super::{audit, lock_network};
use crate::{
    error::{AppError, AppResult},
    modules::wallet::{
        amount_fits_asset_precision, deposit_net_credit_amount,
        domain::timed_deposits::{ChainReceipt, identity_key, within_window},
        presentation::{WalletDepositEventResponse, timed_deposits::*},
    },
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{MySql, Pool, QueryBuilder, Transaction};

const RECEIPT_SELECT: &str = "SELECT id,network,tx_hash,event_index,asset_kind,token_address,address,amount,block_height,block_hash,block_time,confirmations,status,review_reason,order_id,review_note FROM wallet_deposit_receipts";
const MATCH_SELECT: &str = "SELECT id,user_id,asset_id,asset_symbol,network,identity_key,address_key,address,amount,fee_amount,precision_scale,required_confirmations,status,created_at,expires_at FROM wallet_deposit_orders";

#[derive(sqlx::FromRow)]
struct MatchOrder {
    id: u64,
    user_id: u64,
    asset_id: u64,
    asset_symbol: String,
    network: String,
    identity_key: String,
    address_key: String,
    fee_amount: BigDecimal,
    precision_scale: i32,
    required_confirmations: u32,
    status: String,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

/// 扫描入账先锁网络并核对节点存在及配置版本；关闭新单准入仍处理历史订单，旧版本节点任务不得提交。
/// 以不可变链证据匹配历史付款窗口，歧义转人工；订单、钱包、流水和平台分录同事务提交，回放不重复入账。
pub(crate) async fn observe_scanned(
    pool: &Pool<MySql>,
    receipt: &ChainReceipt,
    revision: u64,
) -> AppResult<u64> {
    process_receipt(pool, receipt, Some(revision)).await
}

/// 隔离数据库测试直接注入确定的链证据；生产构建只保留要求配置版本的扫描入口。
#[cfg(test)]
pub(crate) async fn observe(pool: &Pool<MySql>, receipt: &ChainReceipt) -> AppResult<u64> {
    process_receipt(pool, receipt, None).await
}

async fn process_receipt(
    pool: &Pool<MySql>,
    receipt: &ChainReceipt,
    expected_revision: Option<u64>,
) -> AppResult<u64> {
    let mut tx = pool.begin().await?;
    lock_network(&mut tx, &receipt.network).await?;
    if let Some(expected) = expected_revision {
        let (configured, revision): (bool, u64) =
            sqlx::query_as("SELECT endpoint_encrypted IS NOT NULL,revision FROM wallet_deposit_scanners WHERE network=?")
                .bind(&receipt.network)
                .fetch_one(&mut *tx)
                .await?;
        if !configured || revision != expected {
            return Err(AppError::Conflict(
                "deposit scanner configuration changed during scan".into(),
            ));
        }
    }
    let identity = identity_key(&[&receipt.asset_kind, &receipt.token_address]);
    let address_key = identity_key(&[&receipt.address]);
    let existing = sqlx::query_as::<_, ReceiptView>(&format!(
        "{RECEIPT_SELECT} WHERE network=? AND tx_key=SHA2(?,256) AND event_index=? FOR UPDATE"
    ))
    .bind(&receipt.network)
    .bind(&receipt.tx_hash)
    .bind(receipt.event_index)
    .fetch_optional(&mut *tx)
    .await?;
    let id = if let Some(old) = existing {
        if old.amount != receipt.amount
            || old.address != receipt.address
            || old.asset_kind != receipt.asset_kind
            || old.token_address != receipt.token_address
            || old.block_hash != receipt.block_hash
            || old.block_height != receipt.block_height
            || old.block_time != receipt.block_time
        {
            return Err(AppError::Conflict(
                "deposit chain evidence changed; manual reconciliation required".into(),
            ));
        }
        if matches!(old.status.as_str(), "credited" | "ignored") {
            tx.commit().await?;
            return Ok(old.id);
        }
        sqlx::query(
            "UPDATE wallet_deposit_receipts SET confirmations=GREATEST(confirmations,?) WHERE id=?",
        )
        .bind(receipt.confirmations)
        .bind(old.id)
        .execute(&mut *tx)
        .await?;
        old.id
    } else {
        sqlx::query("INSERT INTO wallet_deposit_receipts (network,tx_hash,event_index,asset_kind,token_address,identity_key,address,address_key,amount,block_height,block_hash,block_time,confirmations) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&receipt.network).bind(&receipt.tx_hash).bind(receipt.event_index).bind(&receipt.asset_kind).bind(&receipt.token_address).bind(&identity).bind(&receipt.address).bind(&address_key).bind(&receipt.amount).bind(receipt.block_height).bind(&receipt.block_hash).bind(receipt.block_time).bind(receipt.confirmations).execute(&mut *tx).await?.last_insert_id()
    };
    let orders=sqlx::query_as::<_,MatchOrder>(&format!("{MATCH_SELECT} WHERE network=? AND identity_key=? AND address_key=? AND amount=? AND created_at<=? ORDER BY created_at,id FOR UPDATE"))
        .bind(&receipt.network).bind(&identity).bind(&address_key).bind(&receipt.amount).bind(receipt.block_time).fetch_all(&mut *tx).await?;
    let candidates: Vec<_> = orders
        .iter()
        .filter(|o| within_window(receipt.block_time, o.created_at, o.expires_at))
        .collect();
    if candidates.len() != 1 {
        mark_review(&mut tx, id, None, "no_unique_amount_and_time_window").await?;
    } else {
        let order = candidates[0];
        let unresolved = orders.iter().any(|old| {
            old.id != order.id && old.expires_at <= order.created_at && old.status != "credited"
        });
        let reason = if order.status == "credited" {
            Some("additional_transfer_after_payment")
        } else if unresolved {
            Some("expired_unpaid_combination_reused")
        } else if !amount_fits_asset_precision(&receipt.amount, order.precision_scale) {
            Some("amount_exceeds_wallet_precision")
        } else {
            None
        };
        if let Some(reason) = reason {
            mark_review(&mut tx, id, Some(order), reason).await?;
        } else if receipt.confirmations < order.required_confirmations {
            sqlx::query("UPDATE wallet_deposit_receipts SET status='observed',order_id=?,review_reason='awaiting_confirmations' WHERE id=?").bind(order.id).bind(id).execute(&mut *tx).await?;
        } else {
            credit(&mut tx, id, order, receipt).await?;
        }
    }
    tx.commit().await?;
    Ok(id)
}

async fn mark_review(
    tx: &mut Transaction<'_, MySql>,
    id: u64,
    order: Option<&MatchOrder>,
    reason: &str,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE wallet_deposit_receipts SET status='review',review_reason=?,order_id=? WHERE id=?",
    )
    .bind(reason)
    .bind(order.map(|o| o.id))
    .bind(id)
    .execute(&mut **tx)
    .await?;
    if let Some(order) = order {
        sqlx::query("UPDATE wallet_deposit_orders SET status='review' WHERE id=? AND status IN ('pending','expired')").bind(order.id).execute(&mut **tx).await?;
    }
    Ok(())
}

async fn credit(
    tx: &mut Transaction<'_, MySql>,
    receipt_id: u64,
    order: &MatchOrder,
    receipt: &ChainReceipt,
) -> AppResult<()> {
    if matches!(order.status.as_str(), "credited" | "reversed") {
        return Err(AppError::Conflict(
            "deposit order already has a payment".into(),
        ));
    }
    if receipt.confirmations < order.required_confirmations {
        return Err(AppError::Conflict(
            "deposit confirmations are insufficient".into(),
        ));
    }
    if !amount_fits_asset_precision(&receipt.amount, order.precision_scale) {
        return Err(AppError::Validation(
            "received amount exceeds wallet precision".into(),
        ));
    }
    deposit_net_credit_amount(&receipt.amount, &order.fee_amount, order.precision_scale)
        .map_err(AppError::Validation)?;
    let event = WalletDepositEventResponse {
        id: 0,
        user_id: order.user_id,
        asset_id: order.asset_id,
        asset_symbol: order.asset_symbol.clone(),
        network: receipt.network.clone(),
        address: receipt.address.clone(),
        memo: None,
        tx_hash: receipt.tx_hash.clone(),
        event_index: receipt.event_index,
        amount: receipt.amount.clone(),
        fee_amount: order.fee_amount.clone(),
        block_height: Some(receipt.block_height),
        confirmations: receipt.confirmations,
        required_confirmations: order.required_confirmations,
        status: "observed".into(),
        failure_reason: None,
        credited_at: None,
        reversed_at: None,
        created_at: receipt.block_time,
    };
    let event_id = super::super::deposits::credit_matched_order_in_tx(tx, &event).await?;
    sqlx::query("UPDATE wallet_deposit_orders SET status='credited',paid_amount=?,deposit_event_id=?,paid_at=CURRENT_TIMESTAMP(6) WHERE id=?")
        .bind(&receipt.amount).bind(event_id).bind(order.id).execute(&mut **tx).await?;
    sqlx::query("UPDATE wallet_deposit_receipts SET status='credited',order_id=?,review_reason=NULL WHERE id=?").bind(order.id).bind(receipt_id).execute(&mut **tx).await?;
    Ok(())
}

/// 人工归属只允许选择相同网络、链资产和收款地址的已存在订单；确认数、精度和单次入账约束无法绕过。
/// 可显式处理迟付或错额，但原因、实际金额、订单所有人及资金写入一起审计；相同裁决重放不重复加币。
pub(crate) async fn review(
    pool: &Pool<MySql>,
    admin_id: u64,
    id: u64,
    request: &ReviewDepositReceipt,
) -> AppResult<()> {
    let network =
        sqlx::query_scalar::<_, String>("SELECT network FROM wallet_deposit_receipts WHERE id=?")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(AppError::NotFound)?;
    let mut tx = pool.begin().await?;
    lock_network(&mut tx, &network).await?;
    let receipt =
        sqlx::query_as::<_, ReceiptView>(&format!("{RECEIPT_SELECT} WHERE id=? FOR UPDATE"))
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if receipt.status == "credited"
        && request.action == "credit"
        && receipt.order_id == request.order_id
    {
        tx.commit().await?;
        return Ok(());
    }
    if receipt.status == "ignored" && request.action == "ignore" {
        tx.commit().await?;
        return Ok(());
    }
    if !matches!(receipt.status.as_str(), "review" | "observed") {
        return Err(AppError::Conflict("receipt was already resolved".into()));
    }
    if request.action == "credit" {
        let order_id = request
            .order_id
            .ok_or_else(|| AppError::Validation("a target deposit order is required".into()))?;
        let order =
            sqlx::query_as::<_, MatchOrder>(&format!("{MATCH_SELECT} WHERE id=? FOR UPDATE"))
                .bind(order_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(AppError::NotFound)?;
        if order.network != receipt.network
            || order.identity_key != identity_key(&[&receipt.asset_kind, &receipt.token_address])
            || order.address_key != identity_key(&[&receipt.address])
            // 链区块时间只有秒级精度；同一秒内先后不明确的款项仍只允许人工核对。
            || receipt.block_time.timestamp() < order.created_at.timestamp()
        {
            return Err(AppError::Conflict(
                "receipt does not belong to this order network, asset, address or creation window"
                    .into(),
            ));
        }
        credit(
            &mut tx,
            id,
            &order,
            &ChainReceipt {
                network: receipt.network.clone(),
                tx_hash: receipt.tx_hash.clone(),
                event_index: receipt.event_index,
                asset_kind: receipt.asset_kind.clone(),
                token_address: receipt.token_address.clone(),
                address: receipt.address.clone(),
                amount: receipt.amount.clone(),
                block_height: receipt.block_height,
                block_hash: receipt.block_hash.clone(),
                block_time: receipt.block_time,
                confirmations: receipt.confirmations,
            },
        )
        .await?;
    } else {
        sqlx::query("UPDATE wallet_deposit_receipts SET status='ignored' WHERE id=?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(previous_order) = receipt.order_id
        && (request.action == "ignore" || request.order_id != Some(previous_order))
    {
        sqlx::query("UPDATE wallet_deposit_orders o SET status=IF(expires_at<=CURRENT_TIMESTAMP(6),'expired','pending') WHERE id=? AND status='review' AND NOT EXISTS(SELECT 1 FROM wallet_deposit_receipts r WHERE r.order_id=o.id AND r.status IN ('review','observed'))")
            .bind(previous_order).execute(&mut *tx).await?;
    }
    sqlx::query("UPDATE wallet_deposit_receipts SET reviewed_by=?,review_note=? WHERE id=?")
        .bind(admin_id)
        .bind(&request.reason)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    audit(
        &mut tx,
        admin_id,
        "deposit_receipt.review",
        id,
        Some(json!(receipt)),
        json!({"action":request.action,"order_id":request.order_id}),
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// 管理员分页核对链上证据；未知与不匹配款项保持可见，不伪装为已入账余额。
pub(crate) async fn list(
    pool: &Pool<MySql>,
    query: &DepositPageQuery,
) -> AppResult<DepositPage<ReceiptView>> {
    let mut rows = QueryBuilder::<MySql>::new(format!("{RECEIPT_SELECT} r"));
    let mut count = QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM wallet_deposit_receipts r");
    for builder in [&mut rows, &mut count] {
        builder.push(" WHERE 1=1");
        if let Some(status) = &query.status {
            builder.push(" AND r.status=").push_bind(status);
        }
        if let Some(network) = &query.network {
            builder.push(" AND r.network=").push_bind(network);
        }
        if let Some(symbol) = &query.asset_symbol {
            builder.push(" AND EXISTS(SELECT 1 FROM wallet_deposit_chain_assets c WHERE c.network=r.network AND c.identity_key=r.identity_key AND c.asset_symbol=").push_bind(symbol).push(")");
        }
        if let Some(order_no) = &query.order_no {
            builder.push(" AND EXISTS(SELECT 1 FROM wallet_deposit_orders o WHERE o.id=r.order_id AND o.order_no=").push_bind(order_no.trim()).push(")");
        }
    }
    rows.push(" ORDER BY r.id DESC LIMIT ")
        .push_bind(query.limit.unwrap_or(20).clamp(1, 100))
        .push(" OFFSET ")
        .push_bind(query.offset.unwrap_or(0));
    Ok(DepositPage {
        items: rows.build_query_as().fetch_all(pool).await?,
        total: count.build_query_scalar().fetch_one(pool).await?,
    })
}
