-- 新订单采用实际执行时 Redis 最新价格；已经完成的旧订单凭据不重写。
ALTER TABLE seconds_contract_price_evidence
    DROP CHECK chk_seconds_price_policy,
    ALTER COLUMN policy SET DEFAULT 'redis_latest_v1',
    ADD CONSTRAINT chk_seconds_price_policy CHECK (policy IN ('redis_window_v1', 'redis_latest_v1'));
UPDATE seconds_contract_price_evidence evidence
JOIN seconds_contract_orders orders ON orders.id = evidence.order_id
SET evidence.policy = 'redis_latest_v1'
WHERE orders.status IN ('opened', 'manual_review');

-- 生成器防倒退/重放检查点，每个币种只有当前一行，不累计逐笔历史。
CREATE TABLE market_generation_price_checkpoints (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    symbol VARCHAR(64) NOT NULL,
    event_key CHAR(64) NOT NULL,
    price DECIMAL(38,18) NOT NULL,
    source VARCHAR(32) NOT NULL,
    observed_at TIMESTAMP(6) NOT NULL,
    generation BIGINT UNSIGNED NOT NULL,
    source_version VARCHAR(128) NOT NULL,
    strategy_id BIGINT UNSIGNED NULL,
    strategy_version INT NULL,
    UNIQUE KEY uq_generation_price_symbol (symbol),
    UNIQUE KEY uq_generation_price_event (event_key),
    INDEX idx_generation_price_strategy (strategy_id),
    CONSTRAINT chk_generation_price_positive CHECK (price > 0),
    CONSTRAINT chk_generation_price_source CHECK (source IN ('strategy', 'default')),
    CONSTRAINT chk_generation_price_identity CHECK (
        (source = 'default' AND strategy_id IS NULL AND strategy_version IS NULL)
        OR (source = 'strategy' AND strategy_id IS NOT NULL AND strategy_version > 0 AND generation = strategy_version))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
COMMENT='生成器当前运行价格检查点，每个币种覆盖更新一行，非历史ticker';

-- 升级时仅接续每个生成币种的最后检查点，不修改历史订单引用。
INSERT INTO market_generation_price_checkpoints
    (symbol,event_key,price,source,observed_at,generation,source_version,strategy_id,strategy_version)
SELECT symbol,event_key,price,source,observed_at,generation,source_version,strategy_id,strategy_version
FROM (
    SELECT symbol,event_key,price,source,observed_at,generation,source_version,strategy_id,strategy_version,
           ROW_NUMBER() OVER (PARTITION BY symbol ORDER BY observed_at DESC,id DESC) AS row_num
    FROM market_price_ticks WHERE source = 'default' OR (source = 'strategy' AND strategy_id IS NOT NULL AND strategy_version > 0)
) latest WHERE row_num = 1;
