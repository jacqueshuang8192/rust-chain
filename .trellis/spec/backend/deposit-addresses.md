# Recharge Addresses and Legacy Retirement

## 1. Scope / Trigger

The test environment uses only quick recharge and amount-first on-chain recharge.
Permanent user address allocation is retired. See [Timed On-chain Deposits](./timed-deposits.md)
for reservations, scanner evidence and settlement; this contract owns the removal
boundary and the withdrawal directory that must remain unaffected.

## 2. Signatures

- Active address APIs: `GET|POST /admin/api/v1/wallet/deposit-settings/addresses`
  and `PUT /admin/api/v1/wallet/deposit-settings/addresses/:id`.
- Retired: `POST /api/v1/wallet/deposit-address`,
  `POST /admin/api/v1/wallet/deposits/observe`, and all old
  `/admin/api/v1/deposit-address-pool` list/create/update/batch/reclaim routes
  (including `/wallet/deposit-address-pool` aliases).
- Retained for withdrawals: `GET /api/v1/wallet/deposit-networks?asset_symbol=USDT`
  and Admin `/admin/api/v1/deposit-network-configs` list/create/update.
- Storage: migration 0160 makes `wallet_deposit_shared_addresses.pool_id` an
  independent AUTO_INCREMENT key and retains order foreign keys.

## 3. Contracts

Import `{network,addresses:["0x…"],reason}` directly into the selected network's
shared addresses; do not create an intermediate inventory row or assign users.
Networks do not inherit one another's address groups. On-chain options and order
creation never query the legacy directory or inventory allowlists.

`deposit_network_configs` continues to control withdrawal network selection and
validation. Stored `address_group_code`/`address_group_name` fields and API shape
remain intact; the Admin editor is `/admin/wallet/withdrawal-networks`, with no
recharge inventory/group allocation controls. Mobile/PC retain the directory API
for withdrawals and use `/wallet/deposit-options` for on-chain recharge.

Historical tables/ledger entries are retained. The wallet chain gateway continues
withdrawal broadcast/confirmation, but legacy deposit observations enter dead
letters and cannot credit by a permanent user/address relationship. New deposits
are credited exclusively from scanner evidence and explicit orders.

## 4. Validation & Error Matrix

| Condition | Result |
|---|---|
| Old allocator, inventory or observe request | 404, no allocation or credit |
| Unknown shared address ID on PUT | 404, no enrollment |
| Invalid address anywhere in import | Reject entire batch |
| Disabled old network/old inventory whitelist | No effect on new deposit admission |
| Disabled withdrawal directory network | Existing withdrawal rejection remains |
| Gateway legacy deposit receipt | Audited dead letter, no wallet increment |

## 5. Good / Base / Bad Cases

- Good: import ten ETH addresses directly, configure USDT identity and a healthy
  scanner, then reserve exact coin quantities for 30 minutes.
- Base: configure the same EVM address separately on Base; it belongs to that
  network independently.
- Bad: create permanent inventory rows, infer an owner from them, or use a
  legacy network allowlist to hide a ready on-chain option.

## 6. Tests Required

- Isolated MySQL: apply migrations twice; direct import and audit rollback,
  independently configured networks, no dependence on legacy network settings.
- HTTP: old routes 404; independent address/network permissions; read-only users
  cannot write. New options/order APIs preserve ownership and exact amounts.
- Wallet gateway: legacy deposits dead-letter; withdrawal confirmation still
  settles once. Withdrawal quote, limits, freeze, approval and cooling regressions.
- UI: only two recharge entries; old URLs redirect, direct address form has no
  inventory enrollment step; withdrawal editor and clients keep directory access.

## 7. Wrong vs Correct

Wrong: `POST /wallet/deposit-address` to permanently assign a legacy row.

Correct: `POST /wallet/deposit-orders` with
`{asset_symbol:"USDT",network:"eth",amount:"100",idempotency_key:"unique-key"}`;
server selects a shared address and returns the fixed expiry.
