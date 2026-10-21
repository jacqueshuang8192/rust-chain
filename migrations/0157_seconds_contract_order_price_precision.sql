-- 受控判定价的尾数按交易对价格精度换算，精度在下单时固化到订单，
-- 避免结算时读到已被后台改动的精度，导致同一交易对不同订单的尾数口径不一致。
ALTER TABLE seconds_contract_orders
    ADD COLUMN price_precision INT UNSIGNED NULL COMMENT '下单时固化的交易对价格精度，用于受控判定价尾数' AFTER payout_rate;
