-- 秒合约控输控赢：订单级与用户级控制，以及追加式控制流水。
-- 控制只决定胜负方向，不改写真实市场行情；结算时另存真实市场价用于对账。

ALTER TABLE seconds_contract_orders
    ADD COLUMN control_result VARCHAR(16) NULL COMMENT '人工控制的结算结果：win / loss' AFTER result,
    ADD COLUMN control_actor_type VARCHAR(16) NULL COMMENT '控制来源：admin / agent' AFTER control_result,
    ADD COLUMN control_actor_id BIGINT UNSIGNED NULL COMMENT '控制操作人编号，按来源对应 admin_users 或 agent_admin_users' AFTER control_actor_type,
    ADD COLUMN control_reason VARCHAR(255) NULL COMMENT '控制原因' AFTER control_actor_id,
    ADD COLUMN control_at TIMESTAMP(6) NULL COMMENT '控制设置时刻' AFTER control_reason,
    ADD COLUMN settlement_market_price DECIMAL(38,18) NULL COMMENT '真实市场结算价；受控订单与被控结果不一致时保留原值' AFTER settlement_price,
    ADD CONSTRAINT chk_seconds_contract_orders_control_result
        CHECK (control_result IS NULL OR control_result IN ('win', 'loss')),
    ADD CONSTRAINT chk_seconds_contract_orders_control_actor
        CHECK ((control_result IS NULL AND control_actor_type IS NULL AND control_actor_id IS NULL)
            OR (control_actor_type IN ('admin', 'agent') AND control_actor_id IS NOT NULL));

CREATE TABLE seconds_contract_user_controls (
    user_id BIGINT UNSIGNED NOT NULL PRIMARY KEY,
    control_result VARCHAR(16) NOT NULL COMMENT '该用户后续秒合约订单的结算结果：win / loss',
    control_actor_type VARCHAR(16) NOT NULL COMMENT '控制来源：admin / agent',
    control_actor_id BIGINT UNSIGNED NOT NULL COMMENT '控制操作人编号',
    control_reason VARCHAR(255) NOT NULL COMMENT '控制原因',
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_seconds_contract_user_controls_user FOREIGN KEY (user_id) REFERENCES users(id),
    CONSTRAINT chk_seconds_contract_user_controls_result CHECK (control_result IN ('win', 'loss')),
    CONSTRAINT chk_seconds_contract_user_controls_actor CHECK (control_actor_type IN ('admin', 'agent'))
) COMMENT='秒合约用户级控输控赢设置';

CREATE TABLE seconds_contract_result_control_events (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    scope VARCHAR(16) NOT NULL COMMENT '控制作用域：order / user',
    order_id BIGINT UNSIGNED NULL COMMENT '订单级控制对应的订单编号',
    user_id BIGINT UNSIGNED NOT NULL COMMENT '受控用户编号',
    control_result VARCHAR(16) NULL COMMENT '本次设置的结算结果；NULL 表示取消控制',
    actor_type VARCHAR(16) NOT NULL COMMENT '操作来源：admin / agent',
    actor_id BIGINT UNSIGNED NOT NULL COMMENT '操作人编号',
    reason VARCHAR(255) NOT NULL COMMENT '操作原因',
    created_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    INDEX idx_seconds_contract_control_events_user (user_id, id),
    INDEX idx_seconds_contract_control_events_order (order_id, id),
    CONSTRAINT fk_seconds_contract_control_events_user FOREIGN KEY (user_id) REFERENCES users(id),
    CONSTRAINT fk_seconds_contract_control_events_order FOREIGN KEY (order_id) REFERENCES seconds_contract_orders(id),
    CONSTRAINT chk_seconds_contract_control_events_scope CHECK (scope IN ('order', 'user')),
    CONSTRAINT chk_seconds_contract_control_events_result
        CHECK (control_result IS NULL OR control_result IN ('win', 'loss')),
    CONSTRAINT chk_seconds_contract_control_events_actor CHECK (actor_type IN ('admin', 'agent'))
) COMMENT='秒合约控输控赢追加式流水';
