CREATE TABLE new_coin_holdings (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    project_id BIGINT UNSIGNED NOT NULL,
    user_id BIGINT UNSIGNED NOT NULL,
    source_kind VARCHAR(16) NOT NULL,
    source_id BIGINT UNSIGNED NOT NULL,
    quantity DECIMAL(38,18) NOT NULL,
    cost DECIMAL(38,18) NOT NULL,
    policy_json JSON NOT NULL,
    acquired_at DATETIME(6) NOT NULL,
    unlock_at DATETIME(6) NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'held',
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    sold_at DATETIME(6) NULL,
    UNIQUE KEY uk_new_coin_holding_source (source_kind,source_id),
    KEY idx_new_coin_holdings_owner (user_id,id),
    FOREIGN KEY (project_id) REFERENCES new_coin_projects(id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    CHECK (quantity>0 AND cost>=0),
    CHECK (source_kind IN ('subscription','purchase')),
    CHECK (status IN ('held','sold'))
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE new_coin_trade_quotes (
    id CHAR(36) NOT NULL PRIMARY KEY,
    user_id BIGINT UNSIGNED NOT NULL,
    project_id BIGINT UNSIGNED NOT NULL,
    holding_id BIGINT UNSIGNED NULL,
    side VARCHAR(4) NOT NULL,
    quantity DECIMAL(38,18) NOT NULL,
    price DECIMAL(38,18) NOT NULL,
    gross DECIMAL(38,18) NOT NULL,
    cost DECIMAL(38,18) NOT NULL,
    fee DECIMAL(38,18) NOT NULL,
    net DECIMAL(38,18) NOT NULL,
    source VARCHAR(32) NOT NULL,
    source_version VARCHAR(128) NOT NULL,
    observed_at DATETIME(6) NOT NULL,
    expires_at DATETIME(6) NOT NULL,
    idempotency_key VARBINARY(128) NOT NULL,
    intent_json JSON NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'quoted',
    receipt_json JSON NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    executed_at DATETIME(6) NULL,
    UNIQUE KEY uk_new_coin_quote_intent (user_id,idempotency_key),
    KEY idx_new_coin_quote_project (project_id,id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (project_id) REFERENCES new_coin_projects(id),
    FOREIGN KEY (holding_id) REFERENCES new_coin_holdings(id),
    CHECK (side IN ('buy','sell')),
    CHECK (quantity>0 AND price>0 AND gross>0 AND cost>=0 AND fee>=0 AND net>=0 AND gross=fee+net),
    CHECK (status IN ('quoted','executed'))
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE new_coin_allocation_batches (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    agent_admin_id BIGINT UNSIGNED NOT NULL,
    idempotency_key VARBINARY(128) NOT NULL,
    intent_json JSON NOT NULL,
    response_json JSON NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    UNIQUE KEY uk_new_coin_batch_intent (agent_admin_id,idempotency_key),
    FOREIGN KEY (agent_admin_id) REFERENCES agent_admin_users(id)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE new_coin_operations (
    id BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
    project_id BIGINT UNSIGNED NOT NULL,
    actor_type VARCHAR(16) NOT NULL,
    actor_id BIGINT UNSIGNED NULL,
    action VARCHAR(64) NOT NULL,
    payload_json JSON NOT NULL,
    reason VARCHAR(500) NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    KEY idx_new_coin_operation_project (project_id,id),
    FOREIGN KEY (project_id) REFERENCES new_coin_projects(id),
    CHECK (actor_type IN ('admin','agent','scheduler','user'))
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
