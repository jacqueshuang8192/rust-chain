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


## Administrator Direct Issuance and Records

### 1. Scope / Trigger

Administrator creation now publishes the same v2 contract as agent approval.
Historical `POST /admin/api/v1/new-coins` remains a v1 compatibility endpoint;
the Admin UI creates v2 through the distinct endpoint below. Migration `0154`
adds the immutable admin creation receipt; do not modify it after application.

### 2. Signatures

```text
POST /admin/api/v1/new-coins/issuance
GET  /admin/api/v1/new-coins/holdings
GET  /admin/api/v1/new-coins/buybacks
GET  /admin/api/v1/new-coins/operations
admin_token_issuance_receipts PK(admin_id, idempotency_key VARBINARY(128))
```

### 3. Contracts

- Creation requires `new_coin.projects.write`. Body: existing active `asset_id`,
  distinct `quote_asset_id`, Decimal-text `total_supply` and `issue_price`,
  `issuance_policy`, nonblank `reason`, `idempotency_key`. No caller-selected
  issuer, starting stage or legacy lock fields. Prices/supply respect storage
  and authoritative asset precision. Existing assets are selected, not minted.
- Lock the active administrator row, read canonical intent/receipt, then lock
  assets by increasing ID. Amount formatting and trimmed reason/key canonicalize
  before comparison. Same admin/key/intent returns the original project's current
  snapshot even after a stage change. Changed intent conflicts. A new project,
  its fixed pair, disabled generator, liquidity identity, zero base wallets,
  lifecycle/operation/admin audits and receipt commit atomically.
- `issuer_agent_id=null` means platform direct issuance. Agent approval calls
  the same `publish_policy` helper with its original agent ID. Both paths retain
  the contract seed and never create available funds.
- Holdings require `new_coin.locks.read`; buybacks require
  `new_coin.unlocks.read`; operations require `audit.logs.read`. Their counts and
  rows share predicates and a read transaction. Limit 1..100, offset <=100000.
  IDs must be positive; unknown query fields fail. No read endpoint mutates funds.
- Holdings/buybacks independently filter `project_id`, `user_id`, `email`,
  `agent_id` (buyer's direct referral agent), `issuer_agent_id`, and `status`.
  Holdings preserve immutable policy/cost/acquisition and held/sold status;
  effective stage unlock uses actual purchase/listing, never planned dates.
- Buybacks always require `side=sell AND status=executed`; gross, fee and net
  come from that receipt with explicit quote symbol and observed/executed times.
  Quoted sells and executed buys cannot affect the result or total.
- Operations filter exact project/actor type/actor ID/action and preserve real
  admin/agent/scheduler/user identity (scheduler ID may be null).
- Project DTO adds issuer code, quote/pair symbols, actual purchase,
  next-transition/next-attempt timestamps and transition_error. All timestamps
  are UTC Unix milliseconds, all monetary amounts are strings.
- Project market setup returns only the published pair for v2. Saving a draft
  requires its original `purchase_initial_price`; first publication requires
  actual purchase evidence and purchase/listed stage. Published revision replay
  still returns its receipt without restarting a paused market.

### 4. Validation & Error Matrix

| Input/state | Result |
|---|---|
| Missing permission | 403, no side effects |
| Numeric JSON amount or unknown create field | 422 |
| Invalid policy, inactive/same asset, precision, paging | 400 |
| Same key with changed canonical intent | 409, retain original intent |
| Project/pair unique collision | 409 `TOKEN_ISSUANCE_EXISTS`, entire attempt rolled back |
| Final audit/storage failure | 500, no partial project/pair/wallet/receipt |
| V2 different pair/seed draft | 400 |
| V2 publication before actual purchase | 409 |

### 5. Good / Base / Bad Cases

Good: concurrent identical creation produces one project and one receipt;
`0.20` and `0.2` replay the same intent. A B-owned buyer in an A-issued project
appears under buyer agent B and issuer A filters. A 1,000 gross / 100 fee / 900
net sale is visible only after execution. Base: a preheat project has a disabled
pair and null actual purchase. Bad: use an unexecuted quote as settlement proof.

### 6. Tests Required

`admin_routes::token_issuance` uses isolated MySQL for canonical/concurrent
replay, permission denial, precision, audit rollback, zero wallets, ownership,
pagination/count/empty pages, readonly ledgers, actual-event unlocks, executed
sell filtering and seed/stage protection. Regress existing Admin new-coin and
`agent_routes::token_lifecycle` tests; update OpenAPI schemas and both document
aliases. UI tests cover durable retries, session replacement, response-contract
identity, malformed amounts, original rules and post-failure authoritative reads.

### 7. Wrong vs Correct

Wrong: clear the key after an unknown creation result, silently change the
contract seed, or read v2 holdings from legacy `asset_lock_positions`.
Correct: persist the full session-scoped intent before sending, compare returned
policy/amounts/asset IDs, retry that intent, and read `new_coin_holdings`.

### Rollout

Apply migration 0154 after 0152/0153 before exposing Admin v2 creation. It creates
only receipt storage. Funding the buyback account remains a separate audited
operational procedure; the creation API never fabricates treasury balances.

## Coin Logo Upload and Spot Handoff

### 1. Scope / Trigger
New coin creation, agent review, and listing share `assets.logo_url` as the
canonical image. Public Mobile copy is simplified independently of timestamps
and automatic stage policy; PC presentation is unchanged.

### 2. Signatures
- `POST /agent/api/v1/coin-applications/logo`: authenticated multipart `file`,
  returning the existing image-upload response (`download_url`, object metadata).
- Agent application input/record: optional nullable `logo_url`.
- `POST /admin/api/v1/new-coins/issuance`: optional `logo_url`; absent/null keeps
  the selected asset image. Admin uploads use `/admin/api/v1/uploads/images`.
- Migration 0155 adds nullable application `logo_url` and upload object
  `uploaded_by_agent`, with its FK to `agent_admin_users`.

### 3. Contracts
Uploads reuse configured storage, byte/MIME validation and size limits. Owner
columns distinguish admin, user and agent portal accounts. Uploading alone
publishes no asset/project. Approval copies the immutable application Logo into
its new asset. Admin creation changes the asset Logo in the same transaction as
project/pair creation, audit and receipt. Logo is part of immutable replay input;
missing admin Logo is omitted from serialized intent to preserve old receipts.
Publishing copies the asset Logo into the dedicated pair. Listing refreshes a
configured asset Logo onto that same pair without changing price, pair identity,
history or money. If no asset Logo exists, preserve an existing pair image.
The legacy pair activation path follows the same rule.

### 4. Validation & Error Matrix
| Condition | Result |
| --- | --- |
| Invalid agent session/scope or inactive hierarchy | Existing auth denial; no upload |
| Invalid bytes/MIME/size or disabled storage | Existing upload validation error |
| Optional Logo empty/whitespace | Normalize to no override |
| Logo longer than 2048 characters | Validation error before persistence |
| Same idempotency key with changed Logo | Conflict, no new project/application |
| Approval/creation audit failure | Roll back asset Logo and publication |
| Frontend upload/session replacement failure | Keep previous Logo; no late assignment |

### 5. Good / Base / Bad Cases
Good: uploaded Logo survives confirmation, unknown-result retry, review and spot
handoff. Base: old applications with no Logo remain valid, and admin creation
without an override inherits its selected asset image. Bad: assigning an agent
portal ID to `uploaded_by` or hard-coding `logo_url: None` at review.

### 6. Tests Required
Run agent upload/ownership/publication routes, token lifecycle (including public
project image and listing refresh), admin creation rollback/replay/inheritance,
frontend upload failure/session isolation and receipt mismatch tests. Use an
isolated MySQL database and disposable Redis for lifecycle tests. Run Mobile
release gate and Web quality gates; report existing bundle-budget failures.

### 7. Wrong vs Correct
Wrong: recreate the pair at listing to install a Logo, or silently accept changed
Logo metadata on an existing idempotency key. Correct: update image metadata on
the existing pair in the listing transaction and compare the full original
application/creation intent on retry.

## Spot Visibility and Admission

### 1. Scope / Trigger
Purchase-stage v2 charts must not publish or authorize an ordinary spot market.

### 2. Signatures
`GET /api/v1/markets`, `GET/PUT /api/v1/user/market-favorites[/:symbol]`,
`POST /api/v1/spot/orders`; DB `post_listing_pair_id`, `issuance_policy`,
`lifecycle_status`, `actual_listed_at`; no payload or schema changes.

### 3. Contracts
- A v2 project's pair can be active during purchase solely to run the market
  generator. Active is not evidence of spot listing. Keep symbol-specific ticker,
  depth, trade and K-line reads available to the token purchase workspace.
- `GET /markets`, market-favorite reads/additions and spot pair rules exclude any
  associated v2 project unless `lifecycle_status = listed` and
  `actual_listed_at IS NOT NULL`. Planned dates cannot open spot trading.
  Ordinary pairs and legacy (`issuance_policy IS NULL`) behavior stay unchanged.
- Both financial and compatibility spot order insertion recheck listing under a
  shared project-row lock before writing. Successful financial idempotent replay
  remains before current-rule admission. Numeric pair IDs and symbols obey the
  same rule for buys/sells and market/limit/stop-limit orders.
- Listing keeps the same pair/generator/price/history. No migration, disabling of
  purchase feeds, automatic sale or release of locked holdings is implied.

### 4. Validation & Error Matrix
| State / request | Outcome |
| --- | --- |
| Active v2 purchase pair, or listed label without actual timestamp | Omit from market/favorites; favorite PUT returns 400 |
| Unlisted pair by numeric ID or symbol, any spot order type | 400 TradingPairDisabled; no financial writes |
| Unlisted compatibility writer / transaction race | Listing conflict before insertion |
| Listed plus actual timestamp, active pair | Discoverable and eligible for ordinary spot rules |
| Successful order retry | Original response, no duplicate settlement |

### 5. Good / Base / Bad Cases
Good: live purchase ticker is readable while spot discovery is empty; listing
publishes that same pair. Base: ordinary and legacy active markets stay visible.
Bad: a planned date, active feed, or timestamp without listed stage opens spot.

### 6. Tests Required
Real MySQL/Redis lifecycle tests must prove hidden directories and saved
  favorites, rejection of favorite creation, funded order denial with unchanged
  wallets/ledger/order counts, and actual-listing visibility and spot settlement.
  Include a listed label lacking actual evidence and a compatibility-writer
  attempt, plus ordinary/legacy route regressions.

### 7. Wrong vs Correct
Wrong: disable the purchase pair or treat every active ticker as a spot listing.
Correct: retain active feed reads, gate spot discovery/admission on both actual
listing fields, and let REST catalogs own client market membership.

## Direct purchase and wallet valuation

### 1. Scope / Trigger

V2 purchase-stage clients buy immediately; wallet value must be available before
spot listing. Listing eligibility remains unchanged.

### 2. Signatures

```text
POST /api/v1/new-coins/:symbol/market-purchases
{quantity:"2",idempotency_key:"..."}
=> {idempotency_key:"...",receipt:TradeReceipt}
GET /api/v1/wallet/accounts
accounts[].issuance_valuation?: {price_usdt:string|null,stage:string,expires_at:number}
```

### 3. Contracts

- Direct buys accept no client price, side, fee or recipient. Lock the active
  user, replay canonical `operation=market_purchase` intent, then lock project,
  pair and ordered asset/wallet rows. Load the fresh engine price and verify its
  live lease/version. Persist and execute the internal quote in that same
  transaction; no pending quote or partial funding survives rollback.
- Same user/key and canonical quantity returns the original receipt before
  checking current phase or price. Different quantities or legacy quote intents
  conflict. Old quote/confirm clients and selling retain their existing contracts.
- Wallet valuation reads only nonzero booked balances of the authenticated user.
  Pending subscription quantities are not additional balances: their quote funds
  are already frozen. Subscription/distribution uses fixed `issue_price`;
  purchase/listed requires a fresh platform ticker and live engine.
- Convert the project's actual quote asset through a fresh matching USDT ticker;
  only USDT itself has unit value. Unknown/stale/missing prices stay null.
  `expires_at` is no later than either underlying price's 60-second lifetime.
  Missing metadata means ordinary market valuation, while present/null metadata
  forbids fallback to stale spot prices. Wallet reads never widen spot discovery.

### 4. Validation & Error Matrix

| Condition | Result |
|---|---|
| Numeric quantity / unknown client price | 422 |
| Nonpositive quantity, precision, supply or balance failure | 400, atomic rollback |
| Wrong phase, stale/future price, paused engine, reused changed intent | 409 |
| Final audit fails | 500; no quote, holding, purchase, ledger or balance mutation |
| Lost response, same canonical request after listing | Original receipt |
| Missing valuation or quote conversion ticker | Null valuation, balances retained |

### 5. Good / Base / Bad Cases

2 tokens at the current 0.5 price debit 1 quote unit once, even when retried at
0.8. A booked 2,500-token balance at 0.2 in a quote currency worth 2 USDT values
to 1,000 USDT. Missing conversion cannot be treated as quote=USDT.

### 6. Tests Required

Run isolated MySQL/Redis `agent_routes::token_lifecycle` including the
`direct_purchase` helper: concurrency, canonical replay, audit rollback,
changed/legacy intent collisions, freshness, precision, supply/balance rejection,
listing replay and new-order rejection, fixed/live/missing/FX valuation. Regress
wallet routes, legacy new-coin routes, architecture, documentation and OpenAPI.
Client tests must exercise durable unknown-result retry and account replacement.

### 7. Wrong vs Correct

Wrong: restore purchase tokens to `/markets` to repair their asset value.
Correct: carry wallet-only valuation metadata and keep actual-listing spot gates.
Wrong: fetch a buy quote, then hide its confirmation behind a second request.
Correct: take and settle the live server price in one atomic purchase request.
