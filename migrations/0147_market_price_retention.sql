-- 订单证据与短期行情解耦；旧订单没有此行，继续使用原有定价策略。
CREATE TABLE seconds_contract_price_evidence (
    order_id BIGINT UNSIGNED NOT NULL PRIMARY KEY,
    policy VARCHAR(32) NOT NULL DEFAULT 'redis_window_v1',
    entry_json JSON NOT NULL,
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_seconds_price_evidence_order FOREIGN KEY (order_id)
        REFERENCES seconds_contract_orders(id),
    CONSTRAINT chk_seconds_price_policy CHECK (policy = 'redis_window_v1')
) COMMENT='订单开仓永久价格证据，结算证据保存在原订单字段';

-- 清理按时间小批推进，不在迁移中删除历史数据。
ALTER TABLE market_price_ticks
    ADD INDEX idx_market_price_ticks_retention (observed_at, id),
    ADD INDEX idx_market_price_ticks_source_head (symbol, source, observed_at, id),
    COMMENT='Short-lived price history with permanent order-linked evidence and continuity heads';
