-- Recovery requests and historical work survive restarts without deleting the live checkpoint.
ALTER TABLE wallet_deposit_scanners
    ADD COLUMN recovery_request_json JSON NULL,
    ADD COLUMN backfill_cursor_json JSON NULL,
    ADD COLUMN backfill_target_json JSON NULL,
    ADD COLUMN backfill_last_error VARCHAR(255) NULL,
    ADD COLUMN backfill_blocked BOOLEAN NULL DEFAULT FALSE,
    ADD COLUMN backfill_next_at DATETIME(6) NULL;
