use super::*;

pub(super) async fn verify_cancellation(
    pool: &sqlx::MySqlPool,
    admin: u64,
    user: u64,
    other: u64,
    symbol: &str,
) {
    let first = app::create_order(pool, user, request(symbol, "701", "cancel-first"))
        .await
        .unwrap();
    assert!(matches!(
        app::cancel_order(pool, other, first.id).await,
        Err(crate::error::AppError::NotFound)
    ));
    let ledgers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM wallet_ledger")
        .fetch_one(pool)
        .await
        .unwrap();
    let cancelled = app::cancel_order(pool, user, first.id).await.unwrap();
    assert_eq!(cancelled.status, "cancelled");
    assert!(cancelled.cancelled_at.is_some());
    assert_eq!(cancelled.created_at, first.created_at);
    assert_eq!(cancelled.expires_at, first.expires_at);
    let repeated = app::cancel_order(pool, user, first.id).await.unwrap();
    assert_eq!(repeated.cancelled_at, cancelled.cancelled_at);
    let replay = app::create_order(pool, user, request(symbol, "701", "cancel-first"))
        .await
        .unwrap();
    assert_eq!(replay.id, first.id);
    assert_eq!(replay.status, "cancelled");
    let replacement = app::create_order(pool, other, request(symbol, "701", "cancel-reuse"))
        .await
        .unwrap();
    assert_eq!(
        replacement.address, first.address,
        "cancellation releases the combination before its original deadline"
    );
    let id = settlement::observe(pool, &evidence(&replacement, "701"))
        .await
        .unwrap();
    let receipt: (String, Option<u64>) =
        sqlx::query_as("SELECT status,order_id FROM wallet_deposit_receipts WHERE id=?")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        receipt,
        ("review".into(), None),
        "overlapping windows must not guess a payer"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM wallet_ledger")
            .fetch_one(pool)
            .await
            .unwrap(),
        ledgers
    );
    assert!(
        app::cancel_order(pool, other, replacement.id)
            .await
            .is_err(),
        "unassigned matching evidence must also block cancellation"
    );
    // An explicit reviewed decision can credit the cancelled original, preserving its audit timestamp.
    app::review(
        pool,
        admin,
        id,
        ReviewDepositReceipt {
            action: "credit".into(),
            order_id: Some(first.id),
            reason: "verified original payer after cancellation".into(),
        },
    )
    .await
    .unwrap();
    let credited = app::get_order(pool, None, first.id).await.unwrap();
    assert_eq!(credited.status, "credited");
    assert_eq!(credited.cancelled_at, cancelled.cancelled_at);
    assert!(app::cancel_order(pool, user, first.id).await.is_err());
    app::review(
        pool,
        admin,
        id,
        ReviewDepositReceipt {
            action: "credit".into(),
            order_id: Some(first.id),
            reason: "replay same review".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM wallet_ledger")
            .fetch_one(pool)
            .await
            .unwrap(),
        ledgers + 1
    );

    let single = app::create_order(pool, user, request(symbol, "702", "cancel-late"))
        .await
        .unwrap();
    app::cancel_order(pool, user, single.id).await.unwrap();
    let late_id = settlement::observe(pool, &evidence(&single, "702"))
        .await
        .unwrap();
    let reason: (String, String) =
        sqlx::query_as("SELECT status,review_reason FROM wallet_deposit_receipts WHERE id=?")
            .bind(late_id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(
        reason,
        (
            "review".into(),
            "cancelled_combination_requires_review".into()
        )
    );
    // Replays after manual exclusion must still never auto-credit a cancelled order.
    app::review(
        pool,
        admin,
        late_id,
        ReviewDepositReceipt {
            action: "ignore".into(),
            order_id: None,
            reason: "unrelated payer".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        app::get_order(pool, None, single.id).await.unwrap().status,
        "cancelled"
    );
    let again = settlement::observe(pool, &evidence(&single, "702"))
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM wallet_deposit_receipts WHERE id=?")
            .bind(again)
            .fetch_one(pool)
            .await
            .unwrap(),
        "review"
    );

    // Once the original window has elapsed, its cancellation history still protects a later occupant.
    sqlx::query("UPDATE wallet_deposit_orders SET created_at=DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 60 MINUTE),expires_at=DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 30 MINUTE) WHERE id=?").bind(single.id).execute(pool).await.unwrap();
    let reused = app::create_order(pool, other, request(symbol, "702", "cancel-after-window"))
        .await
        .unwrap();
    let reused_id = settlement::observe(pool, &evidence(&reused, "702"))
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status FROM wallet_deposit_receipts WHERE id=?")
            .bind(reused_id)
            .fetch_one(pool)
            .await
            .unwrap(),
        "review"
    );
    assert!(app::cancel_order(pool, other, reused.id).await.is_err());

    let observed = app::create_order(pool, user, request(symbol, "703", "cancel-observed"))
        .await
        .unwrap();
    let mut pending = evidence(&observed, "703");
    pending.confirmations = 1;
    settlement::observe(pool, &pending).await.unwrap();
    assert!(app::cancel_order(pool, user, observed.id).await.is_err());
    assert_eq!(
        app::get_order(pool, None, observed.id)
            .await
            .unwrap()
            .status,
        "pending"
    );
    let expired = app::create_order(pool, user, request(symbol, "704", "cancel-expired"))
        .await
        .unwrap();
    sqlx::query("UPDATE wallet_deposit_orders SET expires_at=DATE_SUB(CURRENT_TIMESTAMP(6),INTERVAL 1 SECOND) WHERE id=?").bind(expired.id).execute(pool).await.unwrap();
    assert!(app::cancel_order(pool, user, expired.id).await.is_err());

    // Competing cancellation and scanning serialize on the network lock: either cancellation
    // wins and evidence requires review, or payment wins and cancellation is rejected.
    let racing = app::create_order(pool, user, request(symbol, "705", "cancel-race"))
        .await
        .unwrap();
    let receipt = evidence(&racing, "705");
    let (cancel_result, observed_result) = tokio::join!(
        app::cancel_order(pool, user, racing.id),
        settlement::observe(pool, &receipt)
    );
    let rid = observed_result.unwrap();
    let result = app::get_order(pool, None, racing.id).await.unwrap();
    let receipt_state: String =
        sqlx::query_scalar("SELECT status FROM wallet_deposit_receipts WHERE id=?")
            .bind(rid)
            .fetch_one(pool)
            .await
            .unwrap();
    if cancel_result.is_ok() {
        assert_eq!(result.status, "cancelled");
        assert_eq!(receipt_state, "review");
    } else {
        assert_eq!(result.status, "credited");
        assert_eq!(receipt_state, "credited");
        assert!(result.cancelled_at.is_none());
    }
}
