-- New agents and existing agents require review unless explicitly authorized.
ALTER TABLE agents
    ADD COLUMN direct_recharge_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN recharge_policy_version BIGINT UNSIGNED NOT NULL DEFAULT 0,
    ADD CONSTRAINT chk_agent_recharge_policy_version
        CHECK (recharge_policy_version <= 9007199254740991);

CREATE TABLE agent_wallet_recharges (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    agent_id BIGINT UNSIGNED NOT NULL,
    agent_admin_id BIGINT UNSIGNED NOT NULL,
    user_id BIGINT UNSIGNED NOT NULL,
    asset_id BIGINT UNSIGNED NOT NULL,
    asset_symbol VARCHAR(64) NOT NULL,
    amount DECIMAL(38,18) NOT NULL,
    reason VARCHAR(500) NOT NULL,
    idempotency_key VARBINARY(128) NOT NULL,
    mode VARCHAR(16) NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'pending',
    reviewed_by BIGINT UNSIGNED NULL,
    review_reason VARCHAR(500) NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    reviewed_at DATETIME(3) NULL,
    credited_at DATETIME(3) NULL,
    UNIQUE KEY uk_agent_recharge_intent (agent_admin_id, idempotency_key),
    KEY idx_agent_recharge_queue (status, id),
    KEY idx_agent_recharge_owner (agent_id, id),
    KEY idx_agent_recharge_user (user_id, id),
    CONSTRAINT fk_agent_recharge_agent FOREIGN KEY (agent_id) REFERENCES agents(id),
    CONSTRAINT fk_agent_recharge_actor FOREIGN KEY (agent_admin_id) REFERENCES agent_admin_users(id),
    CONSTRAINT fk_agent_recharge_user FOREIGN KEY (user_id) REFERENCES users(id),
    CONSTRAINT fk_agent_recharge_asset FOREIGN KEY (asset_id) REFERENCES assets(id),
    CONSTRAINT fk_agent_recharge_reviewer FOREIGN KEY (reviewed_by) REFERENCES admin_users(id),
    CONSTRAINT chk_agent_recharge_amount CHECK (amount > 0),
    CONSTRAINT chk_agent_recharge_mode CHECK (mode IN ('review', 'direct')),
    CONSTRAINT chk_agent_recharge_status CHECK (status IN ('pending', 'credited', 'rejected')),
    CONSTRAINT chk_agent_recharge_credit CHECK (
        (status = 'credited' AND credited_at IS NOT NULL)
        OR (status <> 'credited' AND credited_at IS NULL)
    ),
    CONSTRAINT chk_agent_recharge_review CHECK (
        (status = 'pending' AND mode = 'review' AND reviewed_by IS NULL
            AND review_reason IS NULL AND reviewed_at IS NULL)
        OR (status = 'credited' AND mode = 'direct' AND reviewed_by IS NULL
            AND review_reason IS NULL AND reviewed_at IS NULL)
        OR (status IN ('credited', 'rejected') AND mode = 'review' AND reviewed_by IS NOT NULL
            AND review_reason IS NOT NULL AND reviewed_at IS NOT NULL)
    )
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE agent_recharge_audits (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    request_id BIGINT UNSIGNED NOT NULL,
    actor_type VARCHAR(16) NOT NULL,
    actor_id BIGINT UNSIGNED NOT NULL,
    action VARCHAR(32) NOT NULL,
    reason VARCHAR(500) NOT NULL,
    created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    KEY idx_agent_recharge_audit_request (request_id, id),
    CONSTRAINT fk_agent_recharge_audit_request FOREIGN KEY (request_id) REFERENCES agent_wallet_recharges(id),
    CONSTRAINT chk_agent_recharge_audit_actor CHECK (actor_type IN ('agent', 'admin'))
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
