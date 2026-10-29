-- Keep the learned Base transaction batch cap across retries and worker restarts.
-- Existing pacing, cooldowns, checkpoints and financial records remain unchanged.
ALTER TABLE wallet_deposit_scanner_keys
    ADD COLUMN rpc_batch_limit INT UNSIGNED NULL;
UPDATE wallet_deposit_scanner_keys SET rpc_batch_limit = 25;
ALTER TABLE wallet_deposit_scanner_keys
    MODIFY COLUMN rpc_batch_limit INT UNSIGNED NOT NULL DEFAULT 25;
