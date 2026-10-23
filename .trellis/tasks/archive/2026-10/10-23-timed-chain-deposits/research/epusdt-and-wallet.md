# Reference implementation and existing wallet integration

## Reference and provenance

- Repository requested by the user: https://github.com/GMWalletApp/epusdt
- Inspected commit: `aed4a970a28d734c8a35499496604b868a24ef7f` (retrieved on 2026-10-23).
- Retrieved the README, tree and source archive through `gh api`; browser/web fetch returned no usable content and direct curl failed TLS. No upstream executable was run.
- The reference is used to understand mechanisms, not as a drop-in dependency or a claim that all its advertised network/asset combinations are implemented here.

## Observed reference mechanisms

1. `src/model/service/order_service.go::ReserveAvailableWalletAndAmount` loops over configured addresses for a requested amount and calls `data.LockTransaction`. When all addresses conflict, it increments the amount according to global precision and repeats. This application's user explicitly rejects that increment: retain the entered amount and return a capacity error instead.
2. `src/model/data/order_data.go::LockTransaction` persists network/address/token/amount reservations in runtime SQLite, removes expired reservations in a transaction, and rejects duplicate active amounts. It records scale/text/precision and an expiry; this is not merely an in-memory lock or necessarily Redis.
3. The create-order path converts a fiat amount using a rate and uses float64 alongside decimal normalization. Neither conversion nor floating-point ingress is suitable here. This repository already requires decimal text, BigDecimal and `DECIMAL(38,18)` bounds.
4. `src/task/listen.go` starts EVM WebSocket listeners plus persistent backfill scanners, a Tron block scanner and a five-second Solana polling job. Network and token settings drive activation.
5. EVM ERC20 listeners filter configured contract addresses and the Transfer event topic, decode recipient and raw integer amount, and resolve block time. `listen_evm_backfill.go` persists a per-network cursor and scans only through the configured confirmed head. WebSocket ingress alone is insufficient for durable delivery.
6. `src/model/service/task_service.go` converts raw token units using configured decimals and matches network/address/token/amount. Its EVM path can recover an expired order from durable order history for an in-window transfer; other paths differ. A new implementation should use one consistent persisted matching rule across networks.
7. The inspected Tron scanner recognizes successful direct TRX transfers and selected TRC20 calls; the Solana path inspects signatures, transaction instructions, token mint/decimals and destination ownership. These paths are chain-specific, not interchangeable generic HTTP polling.
8. `src/model/mdb/chain_token.go` distinguishes a contract/mint from an asset symbol, and uses an empty contract for native assets. The local implementation needs the same distinction and immutable order snapshots of relevant metadata.
9. The inspected EVM scanner files focus on token logs. README support claims do not prove native ETH transfer coverage. Native ETH and BTC require explicit local adapters and tests; Bitcoin is not in the inspected scheduler.

## Existing repository findings

- `migrations/0056_deposit_address_pool.sql`: inventory states are available/assigned/disabled and assigned rows require one user/asset/time. This shape cannot represent several active different-amount orders on one address.
- `migrations/0077_deposit_network_configs.sql` and `.trellis/spec/backend/deposit-addresses.md`: network keys are eth/base/tron/btc/solana; address groups can share EVM inventory. Group membership is address compatibility, not chain identity. New reservations must still include the actual requested network.
- `src/modules/wallet/application.rs::get_or_assign_deposit_address` and `infrastructure/deposits.rs` permanently bind an inventory row to a user. Existing assignments must not silently become another user's shared address while the old client still considers them permanent.
- `observe_deposit_event` currently resolves ownership from the current assigned address row, then atomically writes a deposit event, balance, ledger and platform journal. That ownership lookup must not be reused unchanged for short-lived orders.
- Existing deposit uniqueness is `(network, tx_hash, event_index)` and event amounts/fees are exact decimal fields. Reuse this final accounting boundary with an explicit matched-order owner rather than duplicating wallet mutation.
- `WalletChainDepositObservation` currently has no block timestamp, block hash, sender or contract/mint identity. It cannot safely match a 30-minute historical interval as-is.
- `src/workers/wallet_chain.rs` consumes an external `event_poll_url`; it does not itself read the five chains. It already retries infrastructure failures without advancing the page cursor and persists rejected events in dead letters.
- Mobile `DepositDetailView.vue` allocates an address on mount. PC `Recharge.vue` also allocates from coin/network selection. Both must become amount-entry then explicit order-creation flows, with server expiry and order refresh.
- Admin already supports address import, address/network configuration, permissions and audit. Extend those controls and add order/exception visibility instead of building a separate unrelated operations console.

## Confirmed decisions

- Each network starts with roughly 10 user-supplied addresses; no hard-coded maximum.
- Reservation key: actual network + on-chain asset identity + canonical recipient/memo + exact amount.
- Different amounts may share one address; equal amounts use another address; full capacity returns an error and never changes amount.
- Payment window is 30 minutes, based on server time. Same-user request retries must replay the original order and deadline.
- First release covers Ethereum, Base, Tron, Bitcoin and Solana adapters. Assets open only after complete configuration of their chain identity and precision.

## Identity limitation requiring a product decision

Two users can pay the same amount to a reused address. A transaction included during the new window could be either the new user's timely payment or the old user's late payment. Neither block time nor the public transaction hash proves which account paid. Preserving history and checking inclusion time handles delayed observation of an old in-window payment, but cannot solve a genuinely late payment by itself.

Confirmed by the user: keep the 30-minute reuse rule, but quarantine transfers on combinations with unresolved expired unpaid history for manual ownership review. Do not credit a later user merely because its order is currently active. A proof-of-sender flow is out of scope.

## Proposed implementation slices

1. Durable exact-amount orders/reservations and ownership matching; migration, idempotency, expiry, replay, atomic deposit accounting and exception records.
2. Five chain adapters with persisted scan progress, configured asset identities, canonical addresses, success/finality checks and retry behavior. No private keys or outgoing signing.
3. Admin configuration/order/exception controls and Mobile/PC amount entry, payment details, countdown and status/history.
4. Concurrent allocation, delayed scan, duplicate event, token identity, amount precision, expiry and review regression tests; frontend gates and rollout notes.

## Verification plan and limits

- Financial integration tests must use an isolated real MySQL database. Existing tests silently skip without DATABASE_URL; that is not database validation.
- Chain parsers should be tested against representative RPC/HTTP fixtures, including failed transactions, unrelated tokens, duplicate logs/output indices, pagination and node failures.
- Live receipt cannot be claimed without configured nodes and a controlled test transfer; no production payment or transaction is authorized by this research.
- Record exact executed checks and limits in `docs/superpowers/PROGRESS.md` for each delivered slice.
