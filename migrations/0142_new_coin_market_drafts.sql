-- Drafts never enable trading or mutate live generator configuration.
CREATE TABLE new_coin_market_drafts (
    project_id BIGINT UNSIGNED NOT NULL PRIMARY KEY,
    pair_id BIGINT UNSIGNED NOT NULL,
    revision INT UNSIGNED NOT NULL,
    base_version INT UNSIGNED NOT NULL,
    initial_price DECIMAL(38,18) NULL,
    config_json JSON NOT NULL,
    published_version INT UNSIGNED NULL,
    updated_by BIGINT UNSIGNED NOT NULL,
    updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    CONSTRAINT fk_new_coin_market_project FOREIGN KEY (project_id) REFERENCES new_coin_projects(id),
    CONSTRAINT fk_new_coin_market_pair FOREIGN KEY (pair_id) REFERENCES trading_pairs(id),
    CONSTRAINT chk_new_coin_market_revision CHECK (revision > 0),
    CONSTRAINT chk_new_coin_market_price CHECK (initial_price IS NULL OR initial_price > 0),
    CONSTRAINT chk_new_coin_market_published CHECK (published_version IS NULL OR published_version > base_version)
);
