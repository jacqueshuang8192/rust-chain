# Agent Token Issuance Version 2

## 1. Scope / Trigger

Applies only to projects with `issuance_policy.version=2`. Historical NULL
policies, pending legacy applications, old subscription receipts and old
lock/unlock records retain their version 1 contracts. No backfill reinterprets
those holdings. Immutable migrations are `0152_agent_token_lifecycle.sql` and
`0153_new_coin_holdings_and_receipts.sql`.

## 2. Signatures

```text
POST /agent/api/v1/coin-applications (existing input + issuance_policy)
POST /admin/api/v1/agent-coin-applications/:id/review
GET  /agent/api/v1/token-projects?before_id=
POST /agent/api/v1/token-projects/:id/stage
POST /agent/api/v1/new-coin-allocations
GET  /api/v1/new-coins/holdings?project_id=&before_id=
POST /api/v1/new-coins/:symbol/quotes
POST /api/v1/new-coins/quotes/:id/confirm
```

The policy contains `version`, configuring person's IANA `timezone`,
`purchase_initial_price`, `listing_reference_price`, four phase nodes
(`subscription`, `distribution`, `purchase`, `listing`) each with an independent
`automatic` boolean and Unix-millisecond `at`, plus tagged `lock` and `fee`.
`issue_price` remains the fixed subscription price. Prices, percentages, fees
and quantities remain decimal text. New quote quantity rejects JSON numbers.

Stage input: `{expected_stage,target_stage,reason}`. Allocation input:
`{subscription_ids:[...],percent:"50",reason,idempotency_key}` (1..100 unique
positive IDs). Buy quote input: `{side:"buy",quantity:"10",idempotency_key}`;
sell quote input: `{side:"sell",holding_id,idempotency_key}`. Confirmation has
no client-selected price/fee/recipient. Project and holding lists use exclusive
descending ID cursors, 100-row pages and nullable `next_before_id`.

Public project list/detail include nullable `post_listing_pair_symbol` from the
exact configured pair with matching base/quote IDs. Clients use this identity
for charts and spot navigation; never infer it by concatenating asset symbols.
Subscription POST still returns the compact
`{idempotency_key,status,lock_position_id}` receipt, not the history-row DTO.

## 3. Contracts

### Approval and scheduling

Approval atomically creates the asset, zero-balance user wallets, project,
fixed pair and disabled default generator, immutable contract and audit.
No balances are minted by approval. Version 2 configuration is immutable after
review; legacy Admin issuance/unlock/fee/pair mutations are rejected.

Stages are `preheat -> subscription -> distribution -> purchase -> listed`.
Manual execution follows the same ordered transaction as the worker; each
automatic node is independently enabled. Auto distribution only opens the
phase: it never selects orders, allocates tokens, refunds or sells. Pending
manual subscription obligations block purchase and listing.

The worker polls every 10 seconds, at most 100 durable due candidates ordered
by next attempt and ID. Failed nodes persist a one-minute backoff. Shared pair
generation locks and project row locks serialize API/worker races; actual
purchase/listing events, generator enablement, audit and next due time commit
atomically. Planned time is never evidence of an actual event. Restart resumes
the queue without bypassing a manual node.

All instants are UTC; the IANA zone preserves the configuring person's calendar
semantics. Clients enter local times and reject missing/repeated DST wall-clock
times. Viewers display the same instant locally. Months are calendar months,
clamped at month end, with ambiguous maturity choosing the later instant and
nonexistent maturity advancing to the next valid local instant.

### Selected allocation and ownership

Any user may subscribe to any published project. Agent visibility/allocation
scope follows the buyer's agent subtree, independently of issuer. The issuer
may advance its own project but cannot allocate another team's buyers.

User, hierarchy and portal locks revalidate scope/session before a batch. Lock
projects and the union of all asset/wallet identities in stable order. A batch
uses one explicit 0..100 percentage. Quantity rounds down to base precision;
actual payment is quantity times immutable subscription price, representable
at quote precision. Refund is original frozen quote minus actual payment.
Zero refunds everything. Each selected order is terminal after this allocation;
other selected batches may have different percentages. Mixed unauthorized or
invalid orders roll back the entire batch.

Same portal/key and canonical IDs/percentage/reason returns the original
receipt; changed intent or a new key for a terminal order conflicts. User/asset
ledger legs, treasury proceeds, quota release, holding, receipt and audit share
one transaction. Original subscription `created_at` is the acquisition time,
not the later distribution click.

### Market and holdings

Purchase stage starts the platform generator from its seed and buys at a fresh
server price, never fixed `issue_price` or seed. Listing keeps the same pair,
generator, current price and historical candles; reference listing price is
only descriptive. Remaining unissued supply transfers once to the existing
system spot liquidity account. Normal spot is forbidden before actual listing;
version 2 direct purchase is forbidden after it.

Each allocation/purchase writes `new_coin_holdings`, immutable quantity/cost/
policy/acquisition/maturity, and credits `wallet_accounts.locked`. It creates
no legacy lock/unlock record. Thus legacy scanner/fee/release paths cannot
release or double-spend these lots. `none` is immediately eligible once market
trading is available; it still exits through the atomic sale flow. Stage locks
require actual purchase/listing events; weeks/months start at acquisition;
fixed time uses the original instant. Maturity never automatically sells.

Holdings expose cost, valid market valuation, profit and eligibility. Valuation
rounds down at quote-asset precision; unavailable/paused/stale/unrepresentable
valuations remain null. Eligibility means lock maturity, not guaranteed funds
or a valid engine. Sold lots remain visible as historical records.

### Atomic platform buyback

Quotes are user-bound, whole-lot sells, valid for 15 seconds, created only from
positive platform default/strategy prices observed in the previous 60 seconds
(not in the future). Check active pair and current live generator lease,
pause state, source and version; quote records preserve source and generation
identity. A departed generation invalidates an unexecuted quote. Replay of an
executed quote returns its original receipt even after expiration or pause.

Confirmation locks user, project, original quote, assets/wallets and holding.
Fees are none, max(gross-cost,0) times ratio, gross times ratio, or a fixed quote
asset amount. Quantize at quote precision; fees cannot exceed gross. Remove the
user's locked tokens, credit platform inventory, debit funded platform quote
liquidity and credit user net proceeds, record fee revenue and terminal lot/
quote/audit in one transaction. Never synthesize quote liquidity. Insufficient
platform funds or any final audit failure leaves all balances and lot status
unchanged. Existing lots retain this exit contract after listing.

Clients save account-scoped intents before sending; storage failure blocks a
write. Unknown confirmation results retain the original quote/key and block a
new trade. Retrying a committed-but-lost response returns the same receipt;
expiry does not prohibit querying that terminal result. Known unexecuted
failure permits explicit cancellation and a new quote, never silent repricing.

## 4. Validation & Error Matrix

| Condition | Result |
|---|---|
| Invalid decimal text, node order, timezone, unknown tagged rule | Reject before publication |
| Wrong issuer or buyer subtree | Not found / forbidden, no money writes |
| Pending subscription at purchase/listing | Conflict; no generator/event side effects |
| Stale/missing/future/paused/departed-generation quote | Conflict; no fixed-price fallback |
| Lot not mature, sold, other owner; normal spot before listing | Reject without changing lot or wallets |
| Fee exceeds proceeds or platform lacks quote funds | Reject atomically |
| Final audit/receipt/ledger failure | Full rollback |
| Same executed quote retried concurrently | One settlement, identical original receipts |

## 5. Good / Base / Bad Cases

- 5,000 tokens at 0.2 freezes 1,000; 50% allocation yields 2,500 and refunds 500.
- Another selected batch at 80% yields 4,000 and refunds 200 per identical order.
- Market 0.4 buys 10 for 4, despite seed 0.3 and subscription price 0.2.
- Sell a 2,500-token lot costing 500 at 0.4: gross 1,000; positive-profit fee
  20% is 100; net 900. Listing reference 9.99 never resets the 0.4 market.
- Bad: allow issuer C to allocate buyer B under agent A, automatically sell at
  maturity, or move tracked locked tokens into ordinary spot available balance.

## 6. Tests Required

`agent_routes::token_lifecycle` uses real isolated MySQL and dedicated Redis:
atomic approval, mixed/cross-issuer scope, 50/80 refunds, changed-intent retry,
concurrent idempotency, rollback, all six tagged locks and original acquisition,
valid/invalid quotes, insufficient treasury, continuous handoff, cursor scopes,
worker concurrency/restart/manual gates/backoff. Run legacy `new_coin_routes`
and Admin new-coin tests against real MySQL (serialize global trigger fixtures).
Also run domain/OpenAPI/architecture/documentation/strict Clippy checks.
Web batch tests execute real controls and unknown-result recovery. Mobile/PC
contract tests cover exact arithmetic and quote/receipt identity; browser checks
cover narrow/desktop locales/themes, pagination and original-quote retry.

## 7. Wrong vs Correct

Wrong: interpret a reference issue price as a guaranteed future buyback price.
Correct: show reference/seed separately, quote from the live platform engine,
and settle only the user-approved bounded quote against funded liquidity.

## Rollout

Apply 0152 then 0153 before coordinated API/worker/Agent/Admin/Mobile/PC rollout.
Old clients cannot operate the version 2 branch. Seed/fund the existing system
spot liquidity account through the platform's audited operational process;
approval creates identity and wallets, not funds. Keep approved application
snapshots, receipts and migrations immutable. This task does not deploy, fund
production wallets, or migrate production data.
