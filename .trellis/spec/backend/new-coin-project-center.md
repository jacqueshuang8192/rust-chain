# Admin New-Coin Project Center

Version boundary: the legacy contracts below apply to NULL issuance policies.
Explicit version 2 projects follow [Agent Token Issuance V2](token-issuance-v2.md),
including realtime purchase, manual selected batches and atomic unlock-sale.

## Scope

Admin-only reads/configuration, preserving public and historical settlement
contracts. Routes remain transport-only; application coordinates transactions,
and project-center persistence helpers live in Admin infrastructure.

## Authoritative Read

`GET /admin/api/v1/new-coins/:id` returns the complete project and
`configuration_version`, `subscription_count`, `pending_manual_count`,
`issuance_editable`, `next_lifecycle_status`, `lifecycle_block_reason`.
Read project/counts from one transaction snapshot. Missing IDs return 404.
Project listing filters symbol (substring), lifecycle and operational status
use identical bound predicates for rows and total; pagination is unchanged.

## Issuance Mutation

`PATCH /admin/api/v1/new-coins/:id/issuance` requires a reason, positive
`total_supply`/`issue_price` and original `expected_total_supply`/
`expected_issue_price`. Lock the project before evaluating capability and
original values. Only active preheat projects with no subscriptions, zero
reserved/allocated supply and remaining equal to total qualify.

Check base-asset precision for supply and quote-asset precision for price,
matching creation. Acquire asset locks in stable ID order. Update total and
remaining together; preserve asset identities. Record before/after audit and
event in the same transaction. Late/stale writes return 409 without side effects.

## Configuration Concurrency

Lifecycle, unlock, fee and post-listing-purchase commands accept optional
`expected_config` for compatibility with older clients. New clients send the
opaque original `configuration_version`; compare it under the project lock.
It currently serializes the canonical project audit snapshot, including supply
and linked pair status, so intervening business changes may require refresh.
This is a compare-and-set guard, not a secret or an authorization credential.
Issuance uses its narrower mandatory original-value guard.

## Lifecycle and History

Only preheat -> subscription -> distribution -> listed is offered. Readiness
counts pending manual orders or outstanding manual freeze; listing checks the
same obligations again inside its write transaction. Overview hints do not
replace command validation. Config changes affect future lock snapshots only.
Planned/actual listing and new allocation gates follow the contract below.
No pause/cancellation semantics or retroactive position migration is introduced.

## Authorization and Verification

Both new endpoints use existing Admin authentication and `new_coin.projects`
resource read/write middleware. Frontend visibility is not authorization.
Regression: `admin_new_coin_project_center_guards_issuance_and_stale_configuration`
and project-summary assertions in
`admin_new_coin_manual_distribution_settles_and_refunds_exactly_once`.
Run real local MySQL Admin/user new-coin tests, architecture tests, format,
all-target checks and Clippy; absent-DB skips are not financial test evidence.


## Planned Time, Actual Event and Immutable Lock Gates

- `listed_at` remains the compatible configured **planned** time. Admin also
  returns nullable `actual_listed_at` in Unix milliseconds. Only project creation
  in `listed` and the distribution -> listed command set it, using server time.
  Lifecycle PATCH rejects an explicit non-null `listed_at`; edit plans through
  unlock configuration. Config writes never change the actual event timestamp.
- Migration 0123 recovers an actual timestamp only from an existing recorded
  lifecycle transition/create event. A legacy listed project with no event keeps
  null actual time; neither its plan nor migration execution time is evidence.
- Only NEW prelisting `immediate_on_listing` allocations write a non-null
  `asset_lock_positions.listing_project_id`. Their merge key includes project,
  user and asset. The stored `unlock_at` is source provenance, not maturity;
  release requires that pinned project to be listed with an actual event <= now.
  Changes to a planned date, unlock type or project operational status never
  detach that gate or rewrite amounts/fee snapshots.
- Historical locks keep a null gate and their original timestamps, including
  historical immediate-on-listing, fixed-time and relative-period contracts.
  New fixed/relative locks also use independent maturity, not a listing gate.
  No prior release is reversed. New listed purchases under immediate-on-listing
  are directly available even when their configured plan lies in the future.
  New Admin grants remain distribution-only, not an extra listed purchase path.
- Admin lock list adds `listing_project_id` and `actual_listing_at`. Its
  `unlock_at` is effective maturity (null while gated awaiting listing), so table,
  detail and CSV never advertise a source timestamp as a due unlock time.
  This is a read projection; persisted snapshots remain untouched.
- An audit/event failure during the listing command rolls back both stage and
  actual time. A plan edit or timer alone never advances a lifecycle stage.

## Shared Manual and Worker Unlock Transaction

`unlock_eligibility.rs` owns identity, maturity and fee evidence predicates;
`unlock_scan.rs` uses them for bounded candidates and a separate blocked-fee
count. The worker invokes the same locked release transaction as the user API,
with an explicit clock only for worker/test scheduling.

Release requires pending record, matching owner/asset, positive quantity, active
position and sufficient remaining amount. Fee-disabled historical records pass;
fee-enabled zero amounts require `not_required` plus an asset; positive fees
require `paid`, paid time, the exact negative available wallet ledger and both
matching platform journal legs. A status label alone never proves payment.

Lock order is asset -> wallet -> unlock/position. Precision, wallet locked amount
and predicates are rechecked under lock. Moving locked to available, preserving
frozen, position quantities, terminal unlock status and paired release ledgers
commit together. A repeated terminal record produces no second movement/event.
The worker does not collect fees; it broadcasts only after a successful commit.

Required real tests: `tests/unlock_scanner.rs` (fee evidence, races, rollback),
`tests/new_coin_listing_migration.rs` (exact migration and legacy preservation),
`admin_new_coin_actual_listing_gates_new_locks_without_rewriting_history`
(partial refund, merged gates, rule edits, listing rollback, fixed-time history,
actual event, user/worker race and listed purchase with a future plan).


## Coordinated Rollout

Old release APIs/workers ignore `listing_project_id`. Suspend new-coin writes and
old scanners while applying 0123 and switching API/worker/Admin; never mix an old
release path with newly gated holdings. Rollback versions must retain the gate.
Historical invalid paid evidence is an audit/reconciliation issue, not permission
to fabricate settlement legs, automatically charge again or release early.

## Project Market Drafts

### Scope and Signatures

Migration 0142 adds `new_coin_market_drafts`, one row per project. It links a
pair independently of `post_listing_pair_id`; no migration enables a generator
or pair. The Admin project page exposes a market tab.

```text
GET   /admin/api/v1/new-coins/:id/market-setup
PATCH /admin/api/v1/new-coins/:id/market-setup
POST  /admin/api/v1/new-coins/:id/market-setup/publish
```

### Contracts

Read requires both `new_coin.projects.read` and `market.pairs.read`; writes
require both corresponding `.write` permissions. Candidate pairs use exact
project base/quote asset IDs, capped at 100 with an explicit overflow error.
External pairs can be identified but never configured as synthetic.

Save accepts `{pair_id, expected_revision, expected_market_version,
initial_price, config, reason}`. Amounts use bounded decimal input. Complete
parameters are validated against pair precision; a draft can omit an initial
price, but publication still needs authoritative history or an explicit price.
Project lock then pair row serializes saves. Only the draft and its audit change.

Publish accepts `{pair_id, expected_revision, reason}`. Lock order is shared
generation named lock -> project -> pair -> generator. Reuse the default-market
save transaction and append the publication receipt/audit in the same commit.
Compare live version to draft `base_version`. Identical revision replay returns
`{revision, published_version, replayed:true}` without reapplying parameters or
reenabling paused/disabled settings. Project lifecycle/purchase linkage, pair
status, historical candles and user orders remain untouched.

Client saves explicitly, previews the unchanged draft with the existing
read-only generator preview, then confirms publication. Editing invalidates
preview. Amounts stay strings; preset selection changes only volatility.
Unpublished old-base drafts are conflicts; published drafts are receipts, not
authority over current live parameters. A lost mutation response locks writes
until persisted state is queried; 409 retains local input and offers explicit
discard/reload. Do not silently rebase a stale edit to the newest version.

### Validation and Error Matrix

| Condition | Result |
| --- | --- |
| Either required permission absent | 403 |
| Project inactive, wrong assets, external pair | 400, no write |
| Missing history/initial price or pair disabled on publish | 400, no live write |
| Draft revision/live version changed | 409, preserve draft |
| Decimal exceeds 38,18 or unknown request field | Axum 422 before use case |
| Final audit fails | Entire publication/receipt rolled back |
| Same published revision retried after pause | Original receipt, pause preserved |

### Good/Base/Bad Cases

- Good: save exact `1.000000000000000001`, preview, publish once, replay once.
- Base: unconfigured pair remains disabled until an explicit publish.
- Bad: use the purchase pair as an implicit market link, silently activate a
  disabled pair, use a browser Number for price, or reapply a published draft.

### Tests Required

`admin_routes::new_coin_market` checks permission intersection, CAS, exact
precision, no draft price effects, invalid-pair rejection, final-audit rollback,
concurrent publication and paused replay. `openapi_routes` checks both aliases.
Web model/component tests cover exact strings, stale preview, unknown writes,
conflict recovery, read-only controls and current-live hydration.

### Wrong vs Correct

Wrong: label the existing immediate generator PATCH as "Save draft".
Correct: persist the project draft separately, then publish under the shared
writer lock and existing generator transaction. A fresh displayed price still
does not prove financial price eligibility or worker lease health.

## Agent Issuance Applications

Migration `0151_agent_new_coin_applications.sql` adds immutable applications,
separate from public assets/projects. Agent endpoints are
`/agent/api/v1/coin-applications` (GET/POST), `/options` (GET) and `/:id` (GET).
Admin endpoints use `/admin/api/v1/agent-coin-applications` (GET), `/:id` (GET)
and `/:id/review` (POST). Admin read and review independently require
`new_coin.applications.read` and `.review`; project write does not substitute.

- Owner identity is the authenticated portal account plus its original agent.
  Both list/count and detail constrain those IDs. There is no team-subtree read;
  parents, siblings and another portal on the same node cannot see applications.
  Unknown owner/status fields in input are rejected. Status filters and paging
  are bounded (limit 1..100, offset <=100000), ordered by descending ID.
- Inputs pin symbol, name, precision, positive supply/price, quote asset,
  planned listing time and reason. Money is Decimal text, timestamps milliseconds.
  Symbol/name/precision reuse asset validation; amounts use DECIMAL(38,18) and
  authoritative precision. Pending/rejected records never create assets/projects.
- `(agent_admin_id, idempotency_key VARBINARY(128))` is unique. Lock canonical
  ancestor primary keys, then portal account before the first key read. Validate
  active state, unchanged path/owner and session version; compare all immutable
  inputs on replay. The same key cannot change intent or current owner.
- Approval locks ancestors/portal then application, quote asset and new asset.
  Quote uses an exclusive lock to avoid shared-to-exclusive upgrade deadlocks
  when the project helper later rechecks precision. Recheck original ownership
  and active hierarchy; rejection can close a disabled/reassigned application.
- Review only transitions pending -> approved/rejected. Approval atomically
  creates a coin asset with funding/withdrawal/margin-transfer disabled, zero
  wallet accounts, and a preheat project with immediate-on-actual-listing unlock
  and no unlock fee. No mint, balance credit, trading pair, subscription or
  actual listing is implied. The existing Admin lifecycle manages later stages.
- Project creation is shared via `create_admin_new_coin_project_in_tx`; project,
  lifecycle event, approval receipt and review audit commit together. Audit or
  duplicate-symbol failure rolls everything back and leaves pending unchanged.
  Identical reviewer/decision/trimmed-reason replay returns the original receipt;
  other terminal writes conflict. Detail GET recovers an uncertain outcome.
- Deploy migration before API/Admin. The legacy direct-creation API and historical
  settlement remain unchanged. The default Admin creation UI now uses the v2
  endpoint described in [token-issuance-v2.md](./token-issuance-v2.md).

Validation: `coin_applications_enforce_ownership_and_atomic_publication` uses
real MySQL for portal isolation, immutable replay, pending invisibility, terminal
races, independent RBAC, numeric fields, rejection visibility, inactive-agent
approval and final-audit rollback. Also run existing Admin project-create tests,
OpenAPI, architecture/documentation guards, all-target Clippy and frontend gates.
