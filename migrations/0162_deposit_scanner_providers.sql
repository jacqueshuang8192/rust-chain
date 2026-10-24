-- Independent channels and key slots; preserve encrypted legacy credentials and checkpoints.
CREATE TABLE wallet_deposit_scanner_providers (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    network VARCHAR(32) NOT NULL,
    name VARCHAR(80) NOT NULL,
    priority INT UNSIGNED NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    endpoint_encrypted TEXT NOT NULL,
    request_interval_ms INT UNSIGNED NOT NULL DEFAULT 250,
    next_request_at DATETIME(6) NULL,
    cooldown_until DATETIME(6) NULL,
    last_error VARCHAR(255) NULL,
    last_success_at DATETIME(6) NULL,
    failure_count INT UNSIGNED NOT NULL DEFAULT 0,
    CONSTRAINT fk_deposit_provider_network FOREIGN KEY (network) REFERENCES wallet_deposit_scanners(network),
    INDEX idx_deposit_provider_priority(network, priority, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE wallet_deposit_scanner_keys (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    provider_id BIGINT UNSIGNED NOT NULL,
    name VARCHAR(80) NOT NULL,
    priority INT UNSIGNED NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    api_key_encrypted TEXT NULL,
    request_interval_ms INT UNSIGNED NOT NULL DEFAULT 250,
    next_request_at DATETIME(6) NULL,
    cooldown_until DATETIME(6) NULL,
    last_error VARCHAR(255) NULL,
    last_success_at DATETIME(6) NULL,
    failure_count INT UNSIGNED NOT NULL DEFAULT 0,
    CONSTRAINT fk_deposit_key_provider FOREIGN KEY (provider_id) REFERENCES wallet_deposit_scanner_providers(id) ON DELETE CASCADE,
    INDEX idx_deposit_key_priority(provider_id, priority, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT INTO wallet_deposit_scanner_providers (network,name,endpoint_encrypted)
SELECT network,'默认渠道',endpoint_encrypted FROM wallet_deposit_scanners WHERE endpoint_encrypted IS NOT NULL;
INSERT INTO wallet_deposit_scanner_keys (provider_id,name,api_key_encrypted)
SELECT p.id,'默认凭据',s.api_key_encrypted FROM wallet_deposit_scanner_providers p JOIN wallet_deposit_scanners s ON s.network=p.network;
