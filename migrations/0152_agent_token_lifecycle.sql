-- NULL policy preserves historical contracts; only explicitly approved v2 applications opt in.
ALTER TABLE agent_new_coin_applications ADD COLUMN issuance_policy JSON NULL;
ALTER TABLE new_coin_projects
    ADD COLUMN issuance_policy JSON NULL,
    ADD COLUMN issuer_agent_id BIGINT UNSIGNED NULL,
    ADD COLUMN actual_purchase_at DATETIME(3) NULL,
    ADD COLUMN next_transition_at DATETIME(3) NULL,
    ADD COLUMN next_transition_attempt_at DATETIME(3) NULL,
    ADD COLUMN transition_error VARCHAR(64) NULL,
    ADD INDEX idx_new_coin_schedule (next_transition_attempt_at, next_transition_at, id),
    ADD CONSTRAINT fk_new_coin_issuer FOREIGN KEY (issuer_agent_id) REFERENCES agents(id);
