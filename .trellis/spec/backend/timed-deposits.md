# Timed On-chain Deposit Contract

## 1. Scope / Trigger

Applies to amount-first deposits on Ethereum (`eth`), Base (`base`), Tron,
Bitcoin (`btc`) and Solana. Existing permanently assigned addresses and quick
recharge providers retain their separate paths. No signing, custody or sweeps.

## 2. Signatures

- Migrations `0158_timed_chain_deposits.sql` and `0159_timed_deposit_text_identity.sql`.
- Tables: `wallet_deposit_scanners`, `wallet_deposit_chain_assets`,
  `wallet_deposit_shared_addresses`, `wallet_deposit_orders`, `wallet_deposit_receipts`.
- User `GET /api/v1/wallet/deposit-options`; `GET|POST /api/v1/wallet/deposit-orders`;
  `GET /api/v1/wallet/deposit-orders/:id`.
- Admin `GET /admin/api/v1/wallet/deposit-settings`, `PUT .../scanners/:network`,
  `PUT .../assets`, `PUT .../addresses/:id`; `GET .../deposit-orders[/:id]`,
  `GET .../deposit-receipts`, `POST .../deposit-receipts/:id/review`.
- Lists use `limit` (1..100), `offset`, `status`, `network`, `asset_symbol`,
  exact `order_no`. Receipt order filters refer to its associated candidate/order.
- Worker: `workers::timed_deposits::run_loop`, 3-second scheduling, persisted
  60-second leases, revision fencing, 45-second rounds, up to 32 heights/round.

## 3. Contracts

Creation takes `{asset_symbol,network,amount:string,idempotency_key}`. Amount is
coin quantity, never fiat. Reject extra fields, nonpositive amounts, unsupported
precision, amounts below the asset minimum, and fees consuming the whole payment.
Store within DECIMAL(38,18); admission precision is min(wallet precision, chain
precision). Responses preserve decimal strings and UTC millisecond timestamps.

Network row lock precedes user/order/inventory locks; read CURRENT_TIMESTAMP
**after** acquiring the lock. A reservation is `(network, chain identity,
canonical address, exact amount)` for `[created_at,expires_at)` with a 1,800-second
expiry. Different amounts share addresses; same amounts use another address.
Full capacity returns 409 and never adds fractional suffixes. Paid combinations
remain reserved until their original expiry. Cleanup labels do not own release.

The `(user_id, SHA256(idempotency_key))` binding is permanent. Replay must match
network, asset and exact amount; it returns the original deadline even after
expiry. New keys create new reservations. The clients preserve an uncertain
submission key in sessionStorage until successful resolution.

Chain identity and decimals are immutable. Assets require explicit native/token
identity, active platform asset/deposit flag, network whitelist, available
unassigned shared inventory and a successful caught-up scan within 120 seconds
with no recorded error. Mapping/address additions clear readiness and fence old
leases. Each receipt also rechecks enabled/revision under the network lock, so
a stale in-flight node task cannot commit financial changes after configuration
changes. Previously permanent/reclaimed inventory cannot enter the shared pool;
legacy allocation excludes enrolled rows and old inventory edits are blocked.

Canonical EVM addresses are lowercase; Base58 identities retain case. Text
columns use utf8mb4_unicode_ci for SQLx String metadata; SHA256 keys preserve
case-sensitive identity. Receipt uniqueness is `(network,SHA256(tx_hash),event_index)`.
Do not change applied migrations or introduce binary string decoding workarounds.

Only read-only scanners create receipts; no public observation endpoint exists
for this flow. Every receipt preserves network, contract/mint, recipient, exact
quantity, block hash/height/time, confirmations and event index. Re-observation
cannot change evidence. Use block time to match historical windows, not scan time.
A prior expired unpaid combination makes later matching transfers `review`.
Wrong amount, split payments, late payments, extra transfers and ambiguous
windows remain independent review evidence. Do not aggregate or guess owners.

Automatic and manual settlement use the existing deposit ledger/platform journal
transaction, with the explicit order owner. Fee snapshot, wallet, ledger, journal,
order, receipt and audit commit together. Review payload is
`{action:"credit"|"ignore",order_id:number|null,reason:string}`. Credit requires
same network/chain asset/address, payment no earlier than the creation second, sufficient
confirmations, supported wallet precision and an unpaid target. Second-resolution block timestamps within the creation second stay out of
automatic credit when ordering is uncertain, but allow audited manual assignment.
Manual review may assign a late or wrong-amount transfer; credit its actual amount less the frozen
fee, preserving the requested amount. Replays do not credit twice. Ignoring an
associated receipt clears the candidate review label only if no unresolved
receipt remains; it does not erase unpaid combination history.

`CREDENTIAL_ENCRYPTION_KEY` is the existing 32-byte key. Node URL/API key are
encrypted and never returned, audited or logged. Blank updates preserve them.
Permissions: `wallet.networks` for settings/mappings, `wallet.address_pool` for
shared-address writes, `wallet.deposits` for orders/evidence, `.review` for review.

Supported adapter boundaries:

| Network | Assets | Node requirements |
|---|---|---|
| Ethereum/Base | ETH, configured ERC20 | Mainnet chainId 1/8453; blockHash logs; decimals; callTracer for native external/internal successful calls |
| Tron | Configured TRC20 | Mainnet FullNode, complete block transaction receipts and executed Transfer logs; native TRX rejected |
| Bitcoin | BTC | Synchronized mainnet Bitcoin Core, getblock verbosity 2; each vout independent; exclude coinbase |
| Solana | SOL, classic SPL | Mainnet-beta, finalized full jsonParsed blocks and inner instructions; verify mint program/decimals; Token2022 excluded |

Initialization at confirmed head is allowed only without historical orders.
Missing historical cursor, discontinuous hashes, stale/inconsistent node data,
unsupported parsing or confirmed reorganization stops progress and closes new
order admission. RPC errors do not advance the cursor; replayed completed events
are idempotent. Reorganization recovery requires reconciliation and a reviewed
checkpoint repair; there is no UI button silently skipping blocks.

## 4. Validation & Error Matrix

| Condition | Required result |
|---|---|
| Numeric JSON amount | 422 transport rejection |
| Invalid amount/key/identity/precision | 400 validation |
| No ready scanner/mapping/inventory | No selectable option; creation rejected |
| Same amount occupies all addresses | 409; unchanged amount |
| Same key, changed request | 409 |
| Other user's order | 404; list omits it |
| Missing token / wrong scope or permission | 401 / 403 |
| Stale configuration revision | 409; retain editor draft/reason |
| Duplicate receipt/review | Original result; no additional credit |
| Evidence identity changes | Stop and reconcile; retain original evidence |
| Review lacks matching identity/confirmation | Reject; rollback all financial writes |
| Cursor missing with historical orders | Pause; require explicit recovery |

## 5. Good / Base / Bad Cases

- Good: twelve concurrent 100-token requests and ten addresses yield ten exact
  reservations and two conflicts; a 200-token request may reuse an address.
- Base: a transfer included at minute 29 but observed after minute 30 still
  belongs to the original order when its combination has no unresolved history.
- Bad: automatically credit user B after user A's unpaid 100-token order expires
  and the same combination is reused, without proving who paid.

## 6. Tests Required

- Isolated MySQL: migrations twice, concurrent allocation, full pool, expiry,
  permanent idempotency, scope filtering, delayed scan, ambiguity/review,
  duplicate evidence, fee/journal balance, audit rollback and cursor recovery.
- RPC fixtures: wrong chain, successful/failed/internal calls, complete Tron
  receipts, independent BTC outputs, genuine Solana program/mint identities,
  temporary token accounts, ordered bounded slot lists and reorganization.
- Actual HTTP routes: string-only amount, safe ownership, order-number search,
  unauthenticated/cross-scope requests and separate Admin read/review privileges.
- Mobile and PC: actual SFC behavior, uncertain retry key, exact large numbers,
  session-change stale response rejection, restored orders and expired QR removal.
- Admin: exact preview, immutable revision, secret omission, explicit target/reason,
  failed drafts retained and assigned inventory rejected. No mock proves live-chain receipt.

## 7. Wrong vs Correct

Wrong: `amount = Number(input)`; allocate per user forever; match the current
owner at scan time; reset a missing cursor to latest; read a MySQL COALESCE/MAX
signed BIGINT as u32; use binary-collated VARCHAR as SQLx String.

Correct: keep decimal text, create an explicit half-open order, match immutable
chain evidence against history, quarantine reused unpaid combinations, retain
checkpoints, and read the unsigned confirmation column with descending LIMIT 1
plus an application default when no rows exist.
