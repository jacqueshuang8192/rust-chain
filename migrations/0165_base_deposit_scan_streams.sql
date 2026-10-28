-- Split Base at the existing checkpoint; never skip historical blocks or copy leases.
CREATE TABLE wallet_deposit_scanner_streams (
    network VARCHAR(32) NOT NULL,
    stream VARCHAR(16) NOT NULL,
    cursor_json JSON NULL,
    last_scan_at DATETIME(6) NULL,
    last_error VARCHAR(255) NULL,
    lease_token VARCHAR(64) NULL,
    lease_until DATETIME(6) NULL,
    recovery_request_json JSON NULL,
    backfill_cursor_json JSON NULL,
    backfill_target_json JSON NULL,
    backfill_last_error VARCHAR(255) NULL,
    backfill_blocked BOOLEAN NOT NULL DEFAULT FALSE,
    backfill_next_at DATETIME(6) NULL,
    PRIMARY KEY (network, stream),
    CONSTRAINT fk_deposit_stream_network FOREIGN KEY (network) REFERENCES wallet_deposit_scanners(network),
    CONSTRAINT chk_deposit_base_stream CHECK (network = 'base' AND stream IN ('native', 'token'))
) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

INSERT INTO wallet_deposit_scanner_streams
    (network,stream,cursor_json,last_error,recovery_request_json,backfill_cursor_json,
     backfill_target_json,backfill_last_error,backfill_blocked,backfill_next_at)
SELECT s.network,k.stream,s.cursor_json,s.last_error,s.recovery_request_json,
       s.backfill_cursor_json,s.backfill_target_json,s.backfill_last_error,
       COALESCE(s.backfill_blocked,FALSE),s.backfill_next_at
FROM wallet_deposit_scanners s
CROSS JOIN (SELECT 'native' AS stream UNION ALL SELECT 'token') k
WHERE s.network='base';

-- Fence pre-migration workers and require fresh checks. Legacy checkpoints remain
-- available for operator reconciliation; the new worker never scans this Base row.
UPDATE wallet_deposit_scanners
SET revision=revision+1,last_scan_at=NULL,lease_token=NULL,lease_until=NULL
WHERE network='base';

CREATE TABLE wallet_deposit_scanner_stream_failures (
    provider_id BIGINT UNSIGNED NOT NULL,
    stream VARCHAR(16) NOT NULL,
    cooldown_until DATETIME(6) NULL,
    last_error VARCHAR(255) NULL,
    failure_count INT UNSIGNED NOT NULL DEFAULT 0,
    PRIMARY KEY (provider_id,stream),
    CONSTRAINT fk_deposit_stream_provider FOREIGN KEY (provider_id) REFERENCES wallet_deposit_scanner_providers(id) ON DELETE CASCADE,
    CONSTRAINT chk_deposit_failure_stream CHECK (stream IN ('native','token'))
) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
