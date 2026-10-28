# Timed On-chain Deposit Contract

## 1. Scope / Trigger

Applies to amount-first deposits on Ethereum (`eth`), Base (`base`), Tron,
Bitcoin (`btc`) and Solana. Only quick recharge and amount-first on-chain
recharge remain. Permanent address allocation and legacy observation credit are
retired in this test environment. No signing, custody or sweeps.

## 2. Signatures

- Migrations `0158_timed_chain_deposits.sql`, `0159_timed_deposit_text_identity.sql`,
  `0160_independent_recharge_addresses.sql` and
  `0161_timed_deposit_cancellation.sql`, `0162_deposit_scanner_providers.sql` and
  `0163_deposit_scanner_recovery.sql` and `0165_base_deposit_scan_streams.sql`. The shared table owns its IDs;
  existing order references are preserved without legacy inventory inserts.
- Tables: `wallet_deposit_scanners`, `wallet_deposit_chain_assets`,
  `wallet_deposit_shared_addresses`, `wallet_deposit_orders`, `wallet_deposit_receipts`.
- User `GET /api/v1/wallet/deposit-options`; `GET|POST /api/v1/wallet/deposit-orders`;
  `GET /api/v1/wallet/deposit-orders/:id`;
  `POST /api/v1/wallet/deposit-orders/:id/cancel` (no body, returns DepositOrder).
- Admin `GET /admin/api/v1/wallet/deposit-settings`, `PUT .../scanners/:network`,
  `PUT .../assets`, `GET|POST .../addresses`, `PUT .../addresses/:id`;
  `GET .../deposit-orders[/:id]`,
  `GET .../deposit-receipts`, `POST .../deposit-receipts/:id/review`.
- Lists use `limit` (1..100), `offset`, `status`, `network`, `asset_symbol`,
  exact `order_no`. Receipt order filters refer to its associated candidate/order.
- Worker: `workers::timed_deposits::run_loop`, 3-second scheduling, persisted
  60-second leases, revision fencing, 45-second rounds, up to 32 heights/round, reduced according to configured request spacing.

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
Full capacity returns 409 and never adds fractional suffixes.
Authenticated owners may cancel an unexpired pending unpaid order. Cancel uses
network -> order lock order, checks linked observed/review/credited receipts and
unassigned exact matching evidence, and rejects detected payment/review/expiry
with 409. Cross-user cancellation returns 404; missing/wrong-scope tokens return
401/403. Repeat cancellation returns the original cancelled order. Cancellation
writes only status and `cancelled_at` (nullable UTC milliseconds in DepositOrder),
never wallet/ledger, never deleting original created_at/expires_at or request key.
Allocation excludes rows with a cancellation timestamp even if later manually
credited. Any matching cancelled history prevents automatic credit; overlapping
windows stay unassigned review, and a unique candidate uses
`cancelled_combination_requires_review`. This applies even when a transfer's
block time precedes cancellation but observation arrives later. Manual audited
credit remains possible and preserves cancelled_at. Paid combinations
remain reserved until their original expiry. Cleanup labels do not own release.

The `(user_id, SHA256(idempotency_key))` binding is permanent. Replay must match
network, asset and exact amount; it returns the original deadline even after
expiry. New keys create new reservations. The clients preserve an uncertain
submission key in sessionStorage until successful resolution.

Chain identity and decimals are immutable. Admission requires an explicit native/token
identity, active platform asset/deposit flag, enabled shared address and scanner,
and a successful caught-up scan within 120 seconds with no recorded error.
Legacy `deposit_network_configs` and `deposit_address_pool` do not participate.
Scanner `enabled` means accepting new orders; every configured endpoint continues
scanning historical orders even when this flag is false. Receipt processing
rechecks endpoint presence and revision under the network lock, so stale in-flight
node tasks cannot commit financial changes after configuration changes.

Direct import takes `{network,addresses:string[],reason}` with 1..500 input rows.
Validate and canonicalize the full batch, deduplicate, then lock the network and
insert new shared rows plus audit in one transaction. Duplicate rows are skipped;
reimport never re-enables disabled addresses. Audit failure rolls back the batch.
Additions clear readiness and fence old leases. A network owns its address
identities independently; importing an EVM address on ETH does not enable Base.
`PUT .../addresses/:id` only changes `enabled` on an existing shared row (404 if
absent); it never enrolls an old inventory ID or edits chain identity. Disabling
an address/asset affects new orders only; historical scanner identities remain.

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
Full settings additionally return `platform_assets: [{symbol,status,deposit_enabled,precision_scale}]`.
This minimal catalog is available under `wallet.networks.read` without
`assets.read`; it contains no management-only asset fields. The full frontend
contract requires the catalog, while address-only reads stay address-only.

Each settings `ScannerView` also requires `scan_fresh: boolean`,
`cursor_height: u64 | null` and `checked_at: UTC milliseconds`. Freshness uses
`last_scan_at > CURRENT_TIMESTAMP(6) - INTERVAL 120 SECOND` in MySQL, matching
order admission; null scan time is false. Read height from persisted
`cursor_json.height` (Solana uses slots). The snapshot read never probes nodes,
advances cursors or reveals credentials. A fresh timestamp alone does not prove
admission: endpoint, null error, network switch, asset flags and addresses still
matter. Ship backend and admin together; the strict client rejects missing or
invalid snapshot fields rather than inventing health from its local clock.

Permissions: `wallet.networks` for settings/mappings, `wallet.address_pool` for
shared-address reads/writes, `wallet.deposits` for orders/evidence, `.review` for review.

Supported adapter boundaries:

| Network | Assets | Node requirements |
|---|---|---|
| Ethereum | ETH, configured ERC20 | Mainnet chainId 1; blockHash logs; decimals; callTracer for native external/internal successful calls |
| Base | Direct ETH transfers, configured ERC20 | Mainnet chainId 8453; blockHash logs; decimals; block transaction hashes, batched transaction lookups and individual receipts; no trace or fullTransactions=true requirement |
| Tron | Configured TRC20 | Mainnet FullNode, complete block transaction receipts and executed Transfer logs; native TRX rejected |
| Bitcoin | BTC | Synchronized mainnet Bitcoin Core, getblock verbosity 2; each vout independent; exclude coinbase |
| Solana | SOL, classic SPL | Mainnet-beta, finalized full jsonParsed blocks and inner instructions; verify mint program/decimals; Token2022 excluded |

Initialization at confirmed head is allowed only without historical orders.
Missing historical cursor, discontinuous hashes, stale/inconsistent node data,
unsupported parsing or confirmed reorganization stops progress and closes new
order admission. RPC errors do not advance the cursor; replayed completed events
are idempotent. Reorganization recovery requires reconciliation and a reviewed
checkpoint repair. Ordinary restart/configuration saves never skip blocks; the
explicit audited recovery action below is the only supported head-jump path.

Base native scanning requests `eth_getBlockByNumber(height, false)`, then batches
`eth_getTransactionByHash(hash)` in groups of at most 25 when a native asset is
included (including mappings retained for historical orders). Each HTTP batch
uses the existing durable gate; restore response order by numeric JSON-RPC IDs,
reject missing/duplicate/out-of-range IDs or any failed subcall, and require each
returned transaction hash to equal the requested block hash-list entry. Every
transaction must have a unique hash, matching block hash/height, its exact array
index, valid sender, explicit recipient (null only for creation), and a hex value.
Only positive, non-self transfers to watched recipients request
`eth_getTransactionReceipt(txHash)`. Require matching transaction/block identity,
index, sender and recipient; status 1 credits the outer transfer, status 0 skips
it, and missing/unknown status or incomplete/mismatched evidence fails the block.
No debug/trace call, contract-creation attribution or internal native transfer
inference is allowed on Base. ERC20 logs and Ethereum callTracer remain unchanged.

Native Base receipt `event_index` stays `0x80000000` for each transaction, matching
the previous callTracer root event; the uniqueness key also includes network and
transaction hash. Do not substitute transactionIndex or a new index namespace:
replaying an old direct payment must reach the existing receipt/credit. Existing
internal receipts remain stored; new or unobserved historical internal payments
are outside direct-only support. Operator help must disclose this boundary.
This change has no HTTP DTO, database or migration change and never clears
durable cooldowns or checkpoints automatically.
Base rounds with native assets cap the existing adaptive batch at four blocks
because transaction lookup batches add requests. Provider per-method usage still
applies to every subcall; batching is not a quota multiplier. HTTP 429 continues
to cool the selected key, and sustained catch-up depends on actual plan capacity.

### Provider failover and durable rate control

`SaveScanner.providers?: SaveScannerProvider[]` atomically replaces the network's
provider/key configuration under the existing network revision/lock and audit.
Each provider has stable optional ID, name, enabled flag, priority 0..1000,
HTTP endpoint (blank keeps encrypted value), request_interval_ms 100..2000,
and 1..10 key slots. Each key has stable optional ID, name, enabled, priority,
request_interval_ms, optional secret and explicit clear_api_key. One network
supports 1..10 providers. IDs must belong to the edited network/provider and
must not repeat. An enabled provider needs an enabled slot. A keyless public
endpoint still has one enabled slot with no secret.

`ScannerView.providers` is required in settings responses. Provider/key snapshots
expose cooldown_until, last_error and last_success_at, but never endpoint or
secret values. Both frontend and backend must ship together. Migration 0162
copies existing ciphertexts into a default provider/key, leaving checkpoints
and historical orders unchanged. Legacy single-node writes cannot overwrite a
multi-provider/key pool. The scanner's existing encrypted endpoint column remains
an admission/settlement presence projection; runtime reads the normalized tables.

Templates may contain one `{API_KEY}` in URL path/query, never authority. Runtime
URL-encodes the selected key and omits the header; non-template endpoints retain
TRON-PRO-API-KEY/Bearer behavior. Every new provider attempt verifies chain identity,
asset decimals and the persisted block hash. Only transport/provider protocol
failures permit automatic retry: chain-identity, continuity and financial errors
remain network-level stops rather than being hidden by a different source.

Primary/backup order is provider priority/ID then key priority/ID. A round makes
at most four attempts within its existing 45-second deadline (35 seconds per
attempt). 401/429 cool down the selected key; connection, 403, server, malformed
JSON and application/RPC failures cool down the provider. 429 accepts numeric or
HTTP-date Retry-After, with 60 seconds minimum and seven days maximum; other
minimums are 300 seconds for access rejection and 30 seconds for connection/server
errors. Repeated failures back off exponentially. Success resets only the selected
key/provider; other cooling keys retain their state. Configuration saves without
new credentials preserve cooldowns; explicit credential replacement resets the
corresponding stored failure state.

Before every HTTP request, the gate verifies network revision and live lease,
locks provider then key, reserves the later of their persisted next-request times,
and updates both deadlines. Sleep occurs outside the transaction; eligibility is
checked again before sending. Response completion extends both persisted deadlines
by the configured interval, preventing scheduling/DB latency from consuming the
reserved gap before the request actually arrives. Different workers cannot reset request pacing or
cooldowns on restart. Failed rounds do not advance checkpoints; receipts already
observed remain idempotent. All candidates cooling/unavailable closes admission;
there is no unauthenticated fallback or automatic cursor skip.

### Explicit head recovery and dual progress

`POST /admin/api/v1/wallet/deposit-settings/scanners/:network/recovery` takes
`{mode:"backfill"|"skip",expected_revision:u64,acknowledge_skip:boolean,reason:string}`
and returns 202 with JSON `{queued:true}` (the Admin client requires JSON for non-204 responses). Both backend and Admin map this exact action to
`wallet.networks.write` before the generic `/recovery` operate classifier.
A reason is mandatory; skip requires acknowledgement. Under the network lock,
reject missing endpoint/cursor, a stale revision or an already queued request with
409. Persist the request, actor and reason, increment revision, invalidate lease
and freshness, and audit `deposit_scanner.recovery_requested`. Never call the node
or advance the cursor in the HTTP handler.

The worker reads the checkpoint again after claiming the lease. Recovery validates
network/assets, the old checkpoint and the latest confirmed head (finalized slots
for Solana); retaining history also validates an existing target. Failure leaves
all progress/request data intact for retry. Apply only under the network lock with
matching revision, token and unexpired lease; audit `recovery_applied` atomically
with old/new progress, actor and reason. Both actions require a subsequent successful
live scan before admission can resume. Confirmed reorganization and missing cursors
still require reconciliation; these actions cannot bypass those checks.

Migration 0163 persists `recovery_request_json`, `backfill_cursor_json`,
`backfill_target_json`, `backfill_last_error`, `backfill_blocked`, `backfill_next_at`.
Dual progress retains the oldest unfinished cursor and pins the new target hash.
Repeated recovery extends one continuous range; overlapping evidence is idempotent.
Skip discards the intervening range AND any existing backfill without deleting
orders, receipts or ledger entries. Neither mode runs automatically on restart.

Live work has priority. Schedule one bounded historical batch when live success
is within 30 seconds and error-free, or to recheck a blocked history anchor; wait
at least 15 seconds between historical rounds. Both use the same network lease
and durable provider/key pacing. Validate the fixed history target before scanning,
then the full batch's continuity and final join hash before observing its receipts.
Transport errors retain historical progress without replacing live status. Chain,
continuity or accounting errors persist a history block and close admission; live
success or another dual recovery cannot clear this block. A successful historical
retry clears it and invalidates live freshness, requiring another live success.

ScannerView adds nullable `recovery_mode`, `backfill_cursor_height`,
`backfill_target_height`, `backfill_last_error`, and boolean `backfill_blocked`.
Cast JSON mode extraction explicitly to utf8mb4 text: MySQL otherwise exposes
binary metadata that SQLx cannot decode as String. Snapshot reads remain passive.

`ScanFailure` stores `Box<AppError>` plus its retryable flag. Do not inline
AppError in this wrapper: its layout plus metadata can reach Clippy's 128-byte
large-error threshold on CI even when local compilation passes. Preserve original
error variants/messages and retry classification; do not silence `result_large_err`.
The scanner regression test checks the wrapper size and both classifications.

### Base native / ERC20 progress isolation

#### 1. Scope / Trigger

Base native transaction lookups can fail while ERC20 logs are available. Neither
asset class may block the other class's cursor, evidence settlement or admission.
Other networks retain the single-network scanner contract above.

#### 2. Signatures

Migration `0165_base_deposit_scan_streams.sql` adds
`wallet_deposit_scanner_streams(network,stream)` for `base/native` and
`base/token`, plus `wallet_deposit_scanner_stream_failures(provider_id,stream)`.
`ScannerView.streams` contains two `ScannerStreamView` entries for Base and an
empty array for other networks. Each has live/backfill/recovery health fields.
Recovery accepts optional `stream: "native" | "token"`; omitted/null on Base
queues both streams atomically, while other networks reject a non-null stream.

#### 3. Contracts

- Migration copies the exact live cursor, pending request, historical range and
  history error/block into both streams. It clears freshness/leases and increments
  network revision, never moving a height. Old Base progress remains historical
  rollout evidence and is never consumed by the new runtime or admission.
- The worker runs separate three-second loops for Base native, Base token and
  the existing non-Base group. A long native attempt cannot delay the next token
  round. Order expiry maintenance stays in the non-Base loop.
- Each stream owns its 60-second lease, 45-second round, 35-second attempts,
  realtime/backfill schedule and error state. Queries for assets, historical
  confirmation depth and missing-checkpoint protection filter by asset_kind.
  Disabled identities with historical orders remain monitored.
- Claim/error/complete/gate operations take the network lock before stream and
  provider/key rows. Configuration revision remains network-wide. Base receipt
  settlement checks both network revision and the current stream's unexpired
  token inside the financial transaction; receipt IDs and event indexes do not
  gain a stream component.
- Provider/key pacing and HTTP 401/429 or transport/server cooldowns remain
  shared. RPC application failures, invalid batch responses and complete-attempt
  timeouts use a persisted provider/stream cooldown; sibling success cannot clear
  it. Sibling success also cannot clear an active shared quota cooldown.
  Credential/endpoint replacement resets scoped failures; blank saves preserve them.
- Options and new-order validation read the corresponding Base stream's null
  error and 120-second freshness. Network switches, asset flags, addresses and
  all other admission requirements still apply. Base aggregate snapshot is
  conservative; it must never be used to reject a healthy token payment method.
- Recovery increments the network revision when queued and fences all old work,
  but clears only the selected stream's freshness. Applying Base recovery does
  not increment the network revision again or invalidate sibling progress.
  Both requested/applied audit entries identify the stream. The Admin freezes
  stream, revision and checkpoint in the recovery drawer and strictly requires
  both distinct Base snapshots in settings responses.
- Stop old workers before migration, then deploy backend and Admin together.
  Do not run old and new workers simultaneously. A rollback requires reviewing
  both streams and reconstructing an audited checkpoint; the frozen legacy Base
  row is not a valid automatic rollback checkpoint after independent progress.

#### 4. Validation & Error Matrix

| Condition | Required result |
|---|---|
| Native transaction RPC rejected | Native checkpoint retained; ERC20 continues, credits and admits |
| ERC20 log RPC rejected | Token checkpoint retained; native continues independently |
| Shared HTTP 429 | Selected key cooled for both streams; no quota bypass |
| Configuration changed / expired stream lease | No stale Base receipt or progress commit |
| Token-only backfill/skip | Native history unchanged; token changes audited |
| Missing/duplicate Base stream snapshot | Admin fails closed rather than using aggregate health |

#### 5. Good / Base / Bad Cases

Good: native remains at 100 while tokens reach 101 and credit one USDC receipt.
Base: Tron continues using its original lease, recovery and provider behavior.
Bad: two cursors with one error/readiness gate, or a serial outer loop that waits
for native completion before scheduling the next token scan.

#### 6. Tests Required

Real isolated MySQL with mock RPC: both failure directions; options and create;
real wallet credit; history replay without duplicate ledger entries; scoped and
legacy recovery; shared quota; stale configuration and expired-lease rejection;
repeated token rounds while native lease is held. Migration preserves pending
history and fences old workers; SQLx rerun is clean. Admin tests cover separate
status, missing/duplicate snapshots and selected recovery payload; OpenAPI keeps
stream optional and includes the required response array. Browser check both
statuses, token recovery and narrow layouts. Real-chain throughput remains an
explicit deployment check, not implied by fixture success.

#### 7. Wrong vs Correct

Wrong: `base.ready = network.last_error == null` for every asset, or clear a
provider's current HTTP 429 when a sibling finishes a successful request.
Correct: admit USDC using `base/token` state; keep active shared quota cooldowns
and clear only the successful stream's scoped failure after a full valid scan.

### Scanner HTTP diagnostics

`Node::post_operation(path: &'static str, operation: &'static str, body: Value)`
retains code-defined operation names for REST and JSON-RPC calls. Non-2xx
failures persist the network, operation, numeric HTTP status and a fixed Chinese
operator hint in `wallet_deposit_scanners.last_error` (within 255 characters).
Only the API-key presence flag is shown. Never echo node URLs, request params,
API-key values, response bodies, redirect locations or raw reqwest errors.
Redirects remain disabled. A 403 is not proof of an invalid key: the exact
`api.trongrid.io` host receives a checklist covering permission, IP/method
restrictions, request frequency and quota; other hosts use a generic hint.
Transport failures identify timeout/connect/other without echoing the URL.
No HTTP failure advances the cursor or marks the scanner ready.

## 4. Validation & Error Matrix

| Condition | Required result |
|---|---|
| Numeric JSON amount | 422 transport rejection |
| Invalid amount/key/identity/precision | 400 validation |
| No ready scanner/mapping/inventory | No selectable option; creation rejected |
| Empty/oversize/invalid import batch | 400; no rows or audit committed |
| Valid duplicate import | 204; existing state unchanged |
| Legacy allocation/inventory/observation route | 404; no financial side effect |
| Same amount occupies all addresses | 409; unchanged amount |
| Same key, changed request | 409 |
| Other user's order | 404; list omits it |
| Cancel unexpired unpaid order | cancelled; same deadline, timestamp recorded, combination released |
| Cancel detected/review/credited/expired order | 409; no financial changes |
| Receive after cancellation/reuse | review; no automatic credit |
| Missing token / wrong scope or permission | 401 / 403 |
| Recovery without network-write / skip acknowledgement | 403 / 400; no queued request or progress change |
| Recovery queued successfully | 202; cursor unchanged until worker validates |
| Recovery node failure or obsolete lease/revision | Preserve live and historical checkpoints |
| Historical transport failure | Separate error; retain history and live status |
| Historical anchor/continuity failure | No batch settlement; block admission until validated retry |
| Stale configuration revision | 409; retain editor draft/reason |
| Duplicate receipt/review | Original result; no additional credit |
| Evidence identity changes | Stop and reconcile; retain original evidence |
| Review lacks matching identity/confirmation | Reject; rollback all financial writes |
| Cursor missing with historical orders | Pause; require explicit recovery |
| Settings timestamp is null/stale | `scan_fresh=false`; no healthy UI label |
| Settings read | Return persisted height and DB check time; no cursor mutation |
| Node HTTP 401 / 403 | Preserve status/operation; authentication / access-check hint |
| Node HTTP 404 / 405 | Preserve status/operation; base URL and method-check hint |
| Node HTTP 429 / 5xx / 3xx | Rate/quota / upstream availability / final API URL hint; no redirect |
| Transport failure | Safe timeout/connect/send category; no raw URL or error |
| Main key 429, backup key 401, next key healthy | Persist isolated cooldowns; validated backup initializes/continues same checkpoint |
| All providers cooling/failing | No additional node requests; preserve cursor; network error closes admission |
| Existing provider/key saved with blank secrets | Retain encrypted values and cooldown state |
| Base native direct transfer with matching successful receipt | Produce one native root event at `0x80000000`; no trace calls |
| Base reverted/zero/self/creation/internal-only ETH transfer | No native deposit evidence |
| Base receipt absent, mismatched, malformed or unknown status | Fail entire block; do not advance progress or partially settle that batch |
| Cross-network provider ID / cross-provider key ID | Reject and roll back entire configuration/audit |
| Provider URL template and enabled empty key | Reject; no anonymous fallback |

## 5. Good / Base / Bad Cases

- Good: dual recovery reaches the new head while retaining the old range; replaying that range credits each receipt only once.
- Bad: restart clears the old cursor, or live success hides a blocked historical anchor.
- Good: cancellation wins a race with scanning, so the receipt goes to review; if scanning wins, cancellation fails.
- Good: twelve concurrent 100-token requests and ten addresses yield ten exact
  reservations and two conflicts; a 200-token request may reuse an address.
- Base: a transfer included at minute 29 but observed after minute 30 still
  belongs to the original order when its combination has no unresolved history.
- Bad: automatically credit user B after user A's unpaid 100-token order expires
  and the same combination is reused, without proving who paid.
- Base: a configured endpoint with no completed scan shows waiting; a fresh
  successful scanner with admission disabled stays caught up but cannot accept orders.

## 6. Tests Required

- Isolated MySQL: migrations twice, concurrent allocation, full pool, expiry,
  permanent idempotency, scope filtering, delayed scan, ambiguity/review,
  duplicate evidence, fee/journal balance, audit rollback and cursor recovery;
  direct import atomicity/duplicates, cross-network independence, legacy allowlist
  independence, and disabled-admission scanning/settlement with revision fencing.
- Cancellation: owner isolation, repeat/replay, unchanged windows, immediate reuse, late/reused ambiguity, observed/review/expired rejection and concurrent scan/cancel; verify ledger unchanged on cancel and one entry after repeated manual credit.
- Recovery DB/HTTP fixtures: dual progress/extension/skip, acknowledgement and revision
  rejection, transient/fatal history errors, audit, obsolete in-flight result fencing,
  confirmed target, historical actual settlement and ledger idempotency on replay.
- Provider DB fixtures: upgrade from 0161 with ciphertext preservation, migrations
  twice, 429/Retry-After and 401 key failover, pacing, cross-round cooldown,
  all-unavailable cursor preservation, blank-secret preservation, ownership rollback
  and secret-free settings/audit. Browser exercises actual API saves and responsive
  provider/key editors. Independent providers do not guarantee account quota.
- HTTP fixtures: preserve status/operation for 401/403/404/405/429/5xx/3xx;
  assert TRON-PRO-API-KEY header and successful JSON remain intact, diagnostic
  length fits persistence, secrets/body/Location/parameters stay absent, and
  TronGrid hints require exact host identity.
- RPC fixtures: wrong chain, successful/failed/internal calls, complete Tron
  receipts, independent BTC outputs, genuine Solana program/mint identities,
  temporary token accounts, ordered bounded slot lists and reorganization.
- Actual HTTP routes: string-only amount, safe ownership, order-number search,
  unauthenticated/cross-scope requests and separate Admin read/review privileges.
- Mobile and PC: actual SFC behavior, uncertain retry key, exact large numbers,
  session-change stale response rejection, restored orders and expired QR removal.
- Admin: exact preview, immutable revision, secret omission, explicit target/reason,
  failed drafts retained, direct import preview, address-only permissions and
  selected-network retention after save. No mock proves live-chain receipt.
- Settings snapshot: real MySQL null/fresh/stale times, unsigned height and DB
  check time; serialized response excludes both secrets and encrypted fields,
  and a settings read preserves the cursor hash.

## 7. Wrong vs Correct

Wrong: `amount = Number(input)`; allocate per user forever; match the current
owner at scan time; reset a missing cursor to latest; read a MySQL COALESCE/MAX
signed BIGINT as u32; use binary-collated VARCHAR as SQLx String.

Correct: keep decimal text, create an explicit half-open order, match immutable
chain evidence against history, quarantine reused unpaid combinations, retain
checkpoints, and read the unsigned confirmation column with descending LIMIT 1
plus an application default when no rows exist.

Wrong: show a green receiving status because `enabled=true` or a node URL was
saved. Correct: display snapshot freshness separately from admission and list
every unsatisfied network/asset/address prerequisite.

Wrong: delete a cancelled order or rewrite its expiry to release its slot.
Correct: retain its original window and cancellation timestamp, release allocation
explicitly, and require manual attribution of later matching evidence.

Wrong: collapse all provider rejections into "non-success HTTP status", or
append upstream JSON/HTML to make it more useful. Correct: report the numeric
status and code-defined operation with safe fixed guidance; obtain the actual
provider account evidence before claiming a specific operational root cause.

Wrong: direct SQL head reset, or classify every `/recovery` action as operate.
Correct: use the reasoned, revision-pinned network-write request and validate its
confirmed target in the worker; never erase retained history without explicit skip.

Wrong: replace Base traces with transaction values without checking receipt
success, or assign transactionIndex as the native event index. Correct: verify
successful receipt identity and retain the old trace-root event index. Required
fixtures cover mixed ERC20/native blocks, 18-decimal amounts, reverted and excluded
transfers, malformed/duplicate transactions, all receipt identity mismatches,
missing receipts after an earlier valid transfer, anchor/token-only request
shapes, and equality with the retained Ethereum trace-root evidence. Keep the
Ethereum internal-transfer regression; Base direct-only support must not narrow it.
Batch fixtures must include shuffled replies, multiple 25-call chunks, empty
requests, missing results, individual errors and invalid response IDs/versions.
