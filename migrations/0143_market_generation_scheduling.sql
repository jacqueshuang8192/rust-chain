-- Scheduling attempts are not evidence that a price was generated or published.
CREATE TABLE market_generation_attempts (
    pair_id BIGINT UNSIGNED NOT NULL PRIMARY KEY,
    last_attempt_at DATETIME(6) NOT NULL,
    CONSTRAINT fk_market_attempt_pair FOREIGN KEY (pair_id) REFERENCES trading_pairs(id) ON DELETE CASCADE,
    KEY idx_market_attempt_time (last_attempt_at, pair_id)
);
