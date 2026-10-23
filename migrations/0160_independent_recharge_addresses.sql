-- The on-chain pool owns its addresses. Permanent allocation inventory is retired.
-- Keep existing order references and shared IDs; new IDs no longer come from legacy inventory.
ALTER TABLE wallet_deposit_orders DROP FOREIGN KEY wallet_deposit_orders_ibfk_4;
ALTER TABLE wallet_deposit_shared_addresses
    DROP FOREIGN KEY wallet_deposit_shared_addresses_ibfk_1,
    MODIFY COLUMN pool_id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    COMMENT='按网络直接维护的链上收款地址';
ALTER TABLE wallet_deposit_orders
    ADD CONSTRAINT fk_deposit_order_shared_address FOREIGN KEY (pool_id)
    REFERENCES wallet_deposit_shared_addresses(pool_id);
