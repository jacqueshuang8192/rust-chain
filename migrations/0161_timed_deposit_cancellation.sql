-- Preserve original attribution windows and an immutable cancellation timestamp.
ALTER TABLE wallet_deposit_orders
    ADD COLUMN cancelled_at TIMESTAMP(6) NULL AFTER paid_at,
    DROP CHECK chk_deposit_order_status,
    ADD CONSTRAINT chk_deposit_order_status CHECK (status IN ('pending','expired','review','credited','reversed','cancelled'));
