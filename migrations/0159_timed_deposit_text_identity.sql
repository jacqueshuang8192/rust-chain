-- SQLx String requires non-binary MySQL text metadata. Preserve case-sensitive
-- identities in generated SHA-256 keys instead of relying on text collation.
ALTER TABLE wallet_deposit_orders
    ADD COLUMN request_key CHAR(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci
        GENERATED ALWAYS AS (SHA2(idempotency_key, 256)) STORED,
    DROP INDEX uq_deposit_order_request,
    MODIFY COLUMN idempotency_key VARCHAR(128) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
    ADD UNIQUE KEY uq_deposit_order_request (user_id, request_key);
ALTER TABLE wallet_deposit_receipts
    ADD COLUMN tx_key CHAR(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci
        GENERATED ALWAYS AS (SHA2(tx_hash, 256)) STORED,
    DROP INDEX uq_deposit_receipt_event,
    MODIFY COLUMN tx_hash VARCHAR(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci NOT NULL,
    ADD UNIQUE KEY uq_deposit_receipt_event (network, tx_key, event_index);
