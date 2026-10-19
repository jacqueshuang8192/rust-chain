-- Admin direct issuance keeps a durable immutable intent for unknown-result retries.
CREATE TABLE admin_token_issuance_receipts (
    admin_id BIGINT UNSIGNED NOT NULL,
    idempotency_key VARBINARY(128) NOT NULL,
    intent_json JSON NOT NULL,
    project_id BIGINT UNSIGNED NOT NULL,
    created_at DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (admin_id, idempotency_key),
    FOREIGN KEY (admin_id) REFERENCES admin_users(id),
    FOREIGN KEY (project_id) REFERENCES new_coin_projects(id)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
