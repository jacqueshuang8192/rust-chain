-- Existing encrypted endpoints, keys, cooldowns and progress are unchanged.
-- Defaults are administrator-confirmed drafts, not implicit runtime fallbacks.
ALTER TABLE wallet_deposit_scanner_providers
    ADD COLUMN preset_id VARCHAR(80) NULL;
