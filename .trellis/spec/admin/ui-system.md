# Admin UI System Contract

The registration legal editor is a full-width page with exact country/language
selection, immutable publication history and read-only user receipts. Both
documents publish together, guarded by `system.legal.write`; read access uses
`system.legal.read`. See the backend
[Registration Legal Documents](../backend/registration-legal-documents.md)
contract. Never insert unreviewed boilerplate as published text.
The editor reuses `QuillRichTextEditor` without image upload and sends its
allowlisted block array, not HTML. A short nonempty document can publish;
show every missing title/body/reason or invalid date beside a disabled publish
action. In this page, `.quill-rich-text-container` needs an explicit height
because Quill's `height: 100%` otherwise extends past its field and intercepts
the publish button. History and read-only views render safe elements, not raw
HTML.

## Numeric Form Controls

- Monetary/rate drafts use text-valued Semi Input wrappers, not InputNumber or
  a `Number(value)` onChange adapter. Preserve exact source text through
  confirmation and JSON serialization; use shared decimal helpers for checks.
  Effective precision excludes trailing zeros. No silent rounding or clamping.
- Integer drafts that can exceed the safe range also retain raw text. Validate
  integer lexical form before conversion, then safe-integer and domain bounds.
  `9007199254740991.1`, `1e3` and `0x10` are not valid integer-form input.
- Optional restriction fields distinguish blank/null from invalid nonblank
  text. Invalid input must block saving, never silently remove a restriction.
  Revalidate at the payload/save boundary, not only the button disabled state.
- Authoritative amount input requires strings and actual storage/asset bounds.
  Display-only helpers may accept legacy finite Numbers; they cannot certify
  exact money or recover digits already lost by JSON parsing.
- Keep finite numeric geometry, chart coordinates, resize dimensions, bounded
  timing/reconnect jitter and explicit minute-grid scheduling as Numbers.
  Chart conversion never overwrites exact OHLCV source text or feeds execution.
- Test real form controls and real payload builders, not validators alone.
  Include narrow/desktop local mocks and assert both disabled invalid inputs
  and exact valid request bodies. Do not use production writes for verification.

## Platform Pair Default Market

- `strategy/internal` pair rows expose a separate 默认行情 SideSheet. Load only
  when opened; no per-row N+1. Display configuration and read-time runtime
  separately, including source/version, last price/time and error. Never imply
  the read snapshot proves the worker is currently healthy.
- Keep decimal inputs as strings, show ratio units and precision, invalidate
  preview on edits, and discard late responses from an old pair/editor request.
  No implicit enablement or automatic clearing of all-market pause.
- Save and independent pause-all need exact write permission, expected_version
  and reason. A 409 preserves draft/reason and requires explicit reload;
  discard/close, navigation and browser exit protect dirty state.
- Manual pause/disable returns to enabled defaults at a minute boundary, not a
  whole-market stop. Activation confirmation explains the saved start price,
  discontinuity and price-consumer effect. All-pause stops generators, not
  orders, positions or old cache. Source labels identify simulated activity.
- Optional `ConfirmAction.description` renders inside the reason confirmation
  and does not change existing callers, submission, errors or destructive style.

- Default mode controls distinguish independent motion from relative following.
  Follow reference options are fully paginated active external pairs; never pick
  the first result or guess BTC's ID. Incomplete/failed/stale directory refresh
  blocks follow writes but not independent-mode edits. Use local Semi Select
  `aria-labelledby` and visible unique labels: its `aria-label` can be overridden.
- Group always-applied enable/initial price/bounds/volume/depth separately from
  independent volatility/mean-reversion/wick fallback parameters. Distinguish
  saved configuration, latest runtime following/fallback reason/time and preview.
  Follow preview is historical-reference model replay, not a future prediction;
  display reference/range/sample count and degraded/overflow warnings. Preserve
  exact decimal, permission, dirty, reason and 409 guards for both modes.

## New-Coin Project Center and Work Queues

- The five navigation entries are project management, subscriptions/allocation,
  distribution/refund records, locks/unlocks and post-listing purchases. The old
  actions URL redirects `project_id` to `/admin/new-coins/projects/:projectId`;
  no ID returns the project list. Deep links use Router state, not location hash.
- Read the complete project by exact ID, not from the first 100 reference rows.
  Decimal fields must be strings; an incomplete/error response disables all
  configuration writes. Query keys include the current Admin session generation.
- Overview uses server supply counters and total/pending order counts. Do not
  infer project totals from a page of orders. Outstanding/refund amounts remain
  authoritative order-level fields; no unimplemented aggregate is implied.
- Hydrate issuance, unlock, fee and purchase forms from the original response.
  Only one configuration category may be dirty. Preserve local milliseconds and
  omit inactive unlock-rule fields. Identity and quote assets are immutable;
  issuance is editable only before orders/quota use in active preheat projects.
- Pin the original `configuration_version` for the life of a draft. Confirm
  changed values, impact and reason; a conflict preserves the draft and offers
  explicit discard/reload. Never attach a silently refreshed version to an old
  form. Dirty navigation is guarded, and lifecycle/grant actions do not discard it.
- Lifecycle contains only the authoritative next command. The backend checks
  again under lock, including pending settlement obligations before listing.
  Purchase enablement activates the selected pair; disablement stops this project
  only. Explain that side effect and fetch pair options only with read permission.
- Records use whitelisted URL filters. Locks and unlocks keep their old URLs,
  render only authorized tabs and mount only the selected list. Unlock-only roles
  still receive the unified navigation entry. Asset-scoped lock links explicitly
  include other projects/sources, never claim a project-level relationship.
- Audit links initialize only sanitized target type/ID in the shared audit page.
- Preserve stable React keys for stateful actions after conditional `Space`
  children: Semi flattens null children, so insertion can otherwise remount an
  open confirmation and lose its reason. ConfirmAction catches failed promises,
  retains the dialog/reason and announces the error rather than leaking a rejected
  promise into Semi Modal.

### Planned / Actual Listing and Lock Maturity

- Label `listed_at` as 计划上市时间, never as evidence of actual listing. Overview
  and project lists separately display `actual_listed_at`; null means 尚未确认上市,
  or 历史事件未记录 for a legacy listed project, never a guessed plan/event.
- Only 计划上市时间 is editable. Explain that plans do not automatically advance
  stages, that new 上市即解禁 holdings wait for the actual event, and that later
  config edits do not change existing gates. Fixed/relative rules stay independent.
- Lock resources require `listing_project_id`, `actual_listing_at`, and effective
  nullable `unlock_at`. A gated pending record reads 待实际上市, not an expired
  source timestamp. Detail/CSV use the same backend effective maturity projection.
- Read DTO validation requires the actual-time field even when null. Listing
  confirmation sends stage, original configuration snapshot and reason only.

## New-Coin Market Setup

### 1. Scope / Trigger

Apply to the project market tab and its independent draft/publication workflow.
Reuse the default generator forms, reference directory and preview chart.

### 2. Signatures

- `GET/PATCH /admin/api/v1/new-coins/:id/market-setup`
- `POST /admin/api/v1/new-coins/:id/market-setup/publish`
- Existing pair default-generator read, preview and pause endpoints remain
  authoritative for generator state.

### 3. Contracts

- Read/write UI requires both project and market-pair permissions. Candidate
  identity comes from server base/quote asset matches, not symbols or purchase
  linkage. External pairs are identified without a synthetic editor.
- Save writes only a server draft. Publication requires a saved unchanged
  revision, a current preview and explicit confirmation. Backend revision,
  permission and business checks still own authorization; the browser preview
  prerequisite is not a server-issued approval credential.
- Keep all financial inputs as decimal strings. Volatility presets change only
  volatility; advanced settings survive. Initial price is explicitly entered
  or copied from issuance, never silently used to reset existing prices.
- Published drafts are receipts. Reopening starts from current live settings;
  stale unpublished drafts require deliberate conflict recovery. Preview and
  mutation responses from a previous selection must not update a new editor.
- Display runtime as a snapshot at `checkedAt`, with price, tick time, source,
  version and errors. A browser timer must not age a healthy unrefreshed
  snapshot into an alleged outage. Refresh replaces the evidence.
- Pause is distinct from disabling trading. Synthetic volume/depth and relative
  following are never represented as real fills or guaranteed liquidity.
- At <=720px the Admin sidebar starts collapsed and collapses when entering
  the breakpoint, while explicit expansion remains possible. Its navigation
  scrolls within the viewport and the collapse control remains reachable.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Missing either read permission | No market request/editor |
| Missing write permission or inactive project | Read-only state |
| Dirty input | Guard navigation and invalidate preview |
| HTTP 409 | Preserve input; explicit discard/reload |
| Mutation outcome unknown | Block further writes until querying persisted state |
| Disabled pair | Allow draft/preview; block publish; no implicit activation |
| Published receipt and newer live version | Hydrate live settings, never replay old values |

### 5. Good / Base / Bad Cases

Good: save `1.000000000000000001`, preview, confirm one revision, then refresh
runtime independently. Base: inspect a paused pair without changing it.
Bad: save through the immediate-live PATCH or claim enabled means healthy.

### 6. Tests Required

Run `NewCoinMarketSetup.test.tsx`, `marketSetupModel.test.ts` and
`AdminLayout.test.tsx`, followed by the full Admin quality gate. Browser mocks
must exercise saved-draft publication, exact JSON, 60-candle preview and
1728/390/320px overflow. Mock checks do not certify real market delivery.

### 7. Wrong vs Correct

Wrong: refresh the expected version beneath an old draft and submit it.
Correct: keep the original version; on conflict retain input until explicit
discard/reload or an intentional new edit from live configuration.

## Manual New-Coin Settlement Actions

- Settlement is a subscription row action. User, project and subscription IDs
  come from that row, not free-text inputs. Only pending `manual_distribution`
  orders and users with project read/write permission can open the action.
- Fetch the exact project on open; while loading/error or outside active
  distribution, confirmation stays disabled. Existing per-order backend checks
  remain authoritative against stale list rows and concurrent operators.
- Quantity is the final allocation. Preview payment and refund using exact
  decimal helpers and the order's issue-price snapshot. Zero means full refund;
  negative/excess amounts disable submission. Require a nonblank reason.
- Keep the same key and quantity on unknown-result retry; successful writes
  reload orders and invalidate only the affected project/new-coin reference cache.
- `额外赠币` lives separately in the project center, never sends a subscription ID
  and never implies fulfillment/refunds. A dirty settings draft disables its tab.
- Subscription tables distinguish original quote, outstanding freeze, actual
  payment and refund. Receipt tables distinguish unlinked grants and zero refunds;
  partial-refund amounts are inspected in the linked subscription records rather
  than inferred from receipt quantity alone.
- Regressions: `NewCoinProjectPage.test.tsx`, `NewCoinResourcePage.test.tsx`,
  `NewCoinManualDistribution.test.tsx`, legacy action/routes/layout tests and
  `ConfirmAction.test.tsx`. Backend contracts: `../backend/new-coin-project-center.md`
  and `../backend/new-coin-manual-distribution.md`.

## Scope

This contract applies to the operations console under `web/src/`. It covers
visual and interaction structure only. UI refactors must not change API paths,
request payloads, authorization scopes, pagination semantics, export behavior,
or destructive-action behavior.

## Brand and Shell

- Reuse tracked HIPPO assets from `web/src/assets/brand/`; do not restore the
  `RC / Rust Chain` placeholder brand.
- User-facing admin decoration is Chinese-only. Keep the HIPPO brand name and
  necessary business abbreviations such as KYC/API/PC, but do not render
  `HIPPO OPERATIONS`, `OPERATIONS`, English environment/security badges, or
  duplicate English section kickers above an existing Chinese heading.
- `AdminLayout` owns the dark graphite navigation rail, sticky operations
  header, current navigation domain/page, production badge, administrator
  identity, and logout action.
- `PageHeader` owns the page-level title, optional description, actions, and
  document title. Page code must not create a competing header layer.
- The content surface must have `min-width: 0`; document-level horizontal
  overflow is forbidden.
- At `max-width: 1400px`, the expanded sidebar is 208px. The collapsed sidebar
  remains 72px. At `max-width: 840px`, use the narrow layout defined by the
  shared stylesheet instead of adding page-local width overrides.

## Resource Page Composition

All standard admin list pages use this order:

```text
PageHeader
AdminResourcePage Card
  data operation section
  optional filter section
  table state or DataTable
DetailDrawer
```

- Keep create, batch, CSV, and refresh actions in the data operation section.
- Keep filters in `FilterBar`; do not build page-specific unlabeled filter rows.
- Filter labels remain visible. Placeholder text is supplementary, not the
  only field name.
- `FilterBar` keeps edits in a local draft until submit, synchronizes when the
  controlled `value` changes, prunes blank values on submit, and immediately
  emits `{}` on reset.
- While `loading` is true, all filter controls, submit, and reset are disabled.
- Empty, loading, and error states use `admin-table-state` and preserve a
  deliberate minimum content area.
- The density button describes the target state: `切换到自适应` or
  `切换到紧凑`.
- The `强平记录` resource exposes business-identifying `邮箱` and `交易对`
  columns before its financial snapshot. `记录ID`, `仓位ID`, and `用户ID`
  remain in the API row for detail/filter behavior but are not visible table
  leaves. `email` is a required response key whose value may be null and then
  uses the shared `-` empty-cell presentation; `symbol` is required and
  non-null. The page must not issue per-row user or market-directory requests.

## Financial Display Precision

- API Decimal fields, mutation payloads, comparisons, and CSV exports retain
  their original decimal strings. Visible formatting is terminal and must
  never overwrite the record or become a request value.
- The seconds-product cycle editor names `payout_rate` as net profit excluding
  principal and explains that `0.4` means 40% profit and 140 total return for a
  100-unit winning stake. It submits the exact decimal string; the list may show
  the same raw multiplier but must not relabel it as principal-inclusive payout.
- Generic Admin financial values render with two required fraction digits and
  at most six. Stablecoins and common fiat-like assets render at most two;
  other named assets render at most eight, tightened by a lower valid asset
  precision when present. Market-price-specific widgets may use an explicit
  price precision instead of the balance policy.
- Rounding is decimal half-up and operates on the decimal coefficient, never
  on a JavaScript `Number`. Rounded negative zero becomes zero. A non-zero
  value below the smallest visible unit renders a threshold (`<0.01`,
  `>-0.000001`) instead of a false zero.
- Generic resource amount cells may infer an asset symbol from the same API
  row only to choose the display cap. Inference must not add a duplicate asset
  label, issue a secondary request, weaken strict row validation, or affect CSV
  serialization. Rate, ratio, leverage, probability, and price columns stay on
  their explicit/generic formatting policy rather than inheriting a wallet
  asset cap accidentally.

## Semi Table Contract

### Column and Scroll Rules

Compact admin tables normalize columns without a numeric width to 160px.
Fixed or explicitly sized columns retain their configured width.

```ts
const tableColumns = normalizeTableColumns(columns, 'compact');
const scroll = containedTableScrollForColumns(
  tableColumns,
  rowSelection ? 48 : 0,
);
```

- Use a numeric `scroll.x` equal to the sum of normalized column widths plus
  the optional row-selection column.
- Raw Semi Table rendering is allowed only inside `shared/ResizableTable`.
  Never enable Semi Table's native `resizable` prop: its combination with
  horizontal scrolling causes fixed-column alignment and duplicate-column
  artifacts.
- Every application-declared leaf column uses the project-owned resize handle,
  including fixed action columns and dynamic columns. Numeric declared widths
  are the initial values; missing widths use the shared 160px default.
- `ResizableTable` owns clamped width state for its mounted instance, provides
  Pointer dragging plus separator keyboard controls, and recomputes numeric
  `scroll.x` from all current leaf widths plus visible row-selection and
  dedicated expand utility columns. Semi defaults `hideExpandedColumn` to
  `true`; only `false` creates the extra 48px expand column.
- Column identity prefers a unique `key`, then a unique `dataIndex`. Duplicate
  identities and anonymous leaves use their tree path so one leaf can never
  overwrite another leaf's width. Unchanged declarations keep mounted widths;
  added/replaced declarations start from their own configured width.
- The project handle must expose `role="separator"`, vertical orientation, a
  Chinese accessible name, current/min/max values, and Left/Right/Home/End
  behavior. It must stop header sorting/filtering activation and clean up
  document listeners and the body drag state after pointer up, cancel, or
  unmount.
- Columns with `key: 'actions'` are action-button columns. They receive the
  shared action-column class and a 120px resize minimum; ordinary data columns
  retain the 80px minimum. A business field titled `操作` is not an action-button
  column unless it uses that key.
- Action-column Semi buttons set both `height` and `min-height` to 24px so the
  shell-wide 36px minimum cannot win, use 8px horizontal padding, and keep
  nested button groups and text on one line even when a caller requests wrap.
- Standard resource action columns are fixed right and 288px wide, and their
  button groups stay on one line.
- Keep the fixed-right separator and shadow visible so the action column does
  not appear inserted between business columns.
- Compact and adaptive `DataTable` modes both route through `ResizableTable`;
  adaptive declarations may remain fluid, but the wrapper assigns their
  controlled initial width and numeric horizontal scroll width.
- Continue supplying a stable `rowKey`; pagination and row selection depend on
  it.

### Wrong vs Correct

```tsx
// Wrong: Semi native resizing conflicts with fixed columns and scroll.x.
<Table columns={columns} fixed resizable scroll={{ x: 'max-content' }} />

// Correct: the project wrapper owns widths, handles, and numeric scroll.x.
<ResizableTable columns={columns} rowSelection={rowSelection} />
```

## Forms, SideSheets, and Confirmation

- Shared admin forms use the global adaptive grid. A 720px medium SideSheet
  resolves to two columns; page-local three-column overrides are forbidden.
- Create and edit actions for the same resource reuse one form component;
  endpoint-only controls such as an initial status are explicit parameters,
  not separate page implementations that can drift in layout or validation.
- Asset create/edit forms expose `margin_transfer_enabled` as the Chinese switch
  `允许转入杠杆账户`; create defaults it off and edit initializes it from the
  returned record. The assets table shows the same boolean as `允许转入杠杆`,
  and every create/update payload sends the explicit value so an unchecked
  control can never be confused with an omitted legacy field.
- When a simplified edit form exposes only the primary item from an existing
  structured payload, preserve every unexposed legacy item unchanged in the
  update request. Never rebuild the payload from visible controls in a way
  that silently deletes hidden translations or migration metadata.
- Detail SideSheets use a bounded width rather than `80%` of a large desktop
  viewport.
- Reference-driven full-page editor SideSheets may use `100vw` when their
  composition combines media, metadata, summary, and a full-width document
  editor. Scope the width override to that resource, keep the header stable,
  make media fill its grid cell, collapse the primary grid by 1100px and the
  field grid by 840px, and verify zero horizontal overflow at each breakpoint.
- Keep the SideSheet header stable and let the body own scrolling.
- Ordinary save/submit actions use the primary HIPPO orange treatment.
  Destructive actions use danger styling.
- `ConfirmAction` requires a non-blank reason, trims it before submission,
  clears the draft after cancel/success, and must not close while submitting.
- Explicit `dangerous` overrides are allowed. Otherwise the shared irreversible
  action matcher determines danger styling.
- Modal cancel and confirm controls need stable Chinese accessible names.

## Tabs and Stateful Workbenches

- A visible tab must own exactly one visible `role="tabpanel"` with a matching
  `aria-labelledby`.
- When form sections are rendered outside Semi `Tabs`, keep one shared React
  state model so switching tabs does not discard edits.
- Security Policy displays one section at a time and keeps the save bar
  outside the tab panel so the action remains predictable.
- KYC and Market Feed workbenches preserve their existing API payloads while
  exposing the active panel semantically.
- Repeated row controls need record-specific accessible names.

## Shared Settings Editors and Audit Explorer

- Singleton settings pages use the shared settings editor shell and hooks rather
  than page-local `useEffect` request state. Query keys include the canonical
  API resource, mutations invalidate that resource after success, and retries
  never replay a write mutation automatically.
- The editor shell owns loading, retryable load failure, save failure, success,
  conflict, and dirty state. HTTP 409 is presented as a Chinese concurrent-edit
  conflict with an explicit latest-data reload action; it must never silently
  overwrite the administrator's draft.
- A dirty editor guards both React Router navigation and browser
  refresh/close. The route confirmation is Chinese and offers continue editing
  or discard-and-leave; `beforeunload` is installed only while dirty and is
  removed after save, reset, discard, or unmount.
- Save confirmation renders field-level changes with Chinese labels and
  human-readable values, a concise impact summary, and a required trimmed
  reason. Ordinary saves use the primary semantic treatment; high-risk changes
  use the danger semantic treatment and state their runtime impact explicitly.
- Secret inputs are write-only. Read responses and editor hydration expose only
  masks, `*_set` flags, last validation/rotation metadata, or equivalent safe
  status. Never place a stored secret, password, token, ciphertext, or a
  placeholder pretending to be that secret back into the input value.
- The audit explorer consumes `GET /admin/api/v1/audit-logs`. Its optional
  `created_from` and `created_to` query parameters are inclusive Unix
  millisecond bounds (including database microseconds within the selected end
  millisecond); the backend rejects an inverted range. The UI translates
  known action, target, and field codes to Chinese, shows structured before/after
  differences, and provides a canonical object link where the target type has a
  supported admin route.
- Audit export uses the currently loaded filters and rows, emits a UTF-8 BOM CSV
  with Chinese headers, and runs every serialized field through the same masking
  layer as the visible detail. Export must not bypass pagination silently: its
  label and filename describe that it contains the current result set.
- Unknown audit fields remain visible under a safe fallback label so new
  backend fields are not silently hidden. Recursive values whose key denotes a
  password, secret, token, credential, private key, or ciphertext are masked
  before rendering, even when historical rows predate backend redaction.
  String values under ordinary keys and all exported metadata fields also pass
  through free-text masking so named assignments, quoted JSON credentials, and
  Bearer tokens cannot escape through diagnostics, IDs, or request traces.
- Prediction configuration has one canonical entry at
  `/admin/prediction/settings`: its `assets` tab owns editable wager-asset
  configuration. The legacy `/admin/prediction/assets` URL redirects to
  `/admin/prediction/settings?tab=assets` and must not appear as a second
  sidebar or generic read-only resource page.

## Admin New-Coin Asset and Local-Time Workflow

### 1. Scope / Trigger

- Applies when changing the Admin new-coin create SideSheet, project-row
  actions, lifecycle action, unlock-rule action, asset selectors, or their
  request serialization.
- This is a cross-layer contract because Admin sends local wall-clock input and
  asset identifiers to Rust DTOs that deserialize absolute times as Unix
  milliseconds and enforce active-asset and unlock-rule invariants.

### 2. Signatures

```ts
POST /admin/api/v1/new-coins
PATCH /admin/api/v1/new-coins/:projectId/lifecycle
PATCH /admin/api/v1/new-coins/:projectId/unlock-rule

isNewCoinProjectCreatable(values: NewCoinProjectValues): boolean
requiredNewCoinLocalDateTimeMillis(value: string, label: string): number
optionalNewCoinLocalDateTimeMillis(value: string, label: string): number | undefined
newCoinProjectActionsPath(projectId: string): string
```

The create request includes positive integer `asset_id` and `quote_asset_id`,
positive Decimal-text `total_supply` and `issue_price`, one supported
`unlock_type`, and exactly the schedule field required by that type.

### 3. Contracts

- Both create asset selectors consume the shared `useAssetOptions` source,
  which requests `/admin/api/v1/assets` with `{ status: 'active', limit: 100 }`.
  `deposit_enabled` and
  `withdraw_enabled` do not affect eligibility; `status = disabled` remains
  ineligible in both the option query and backend asset lock.
- `quote_asset_id` is required, is serialized as a positive integer, and must
  differ from `asset_id`. Quote options exclude the current project asset.
  Selecting a project asset or symbol that conflicts with the selected quote
  clears the quote immediately. Coupled state changes use functional React
  updates so a queued change cannot restore a stale conflicting value.
- `计划上市时间` and `固定解禁时间` use `datetime-local`. Parse their numeric local
  calendar components, reject normalization such as February 30 or a DST gap,
  then send the resulting Unix milliseconds. Never interpret the value as UTC
  text or expose a raw timestamp input to an operator.
- Create serializes only `listed_at` for `immediate_on_listing`, only
  `fixed_unlock_at` for `fixed_time`, or only positive integer
  `relative_unlock_seconds` for `relative_period`. The create button stays
  disabled until the currently selected schedule is valid.
- Lifecycle commands contain no editable timestamp; the server records actual
  listing. Unlock configuration sends the exact active schedule only. Invalid
  time values raise a Chinese field-specific validation error before sending.
- An active row whose `lifecycle_status` is `preheat` exposes `开始申购`.
  Confirmation sends only `{ lifecycle_status: 'subscription', reason }` to
  the lifecycle PATCH and reloads the authoritative list after success. Other
  lifecycle stages omit the shortcut; an inactive preheat row keeps it
  disabled. The backend active-project and transition checks remain
  authoritative under races.
- The project-row `配置与操作` entry navigates to
  `/admin/new-coins/actions?project_id=:id` through React Router. The admin app
  uses `createBrowserRouter`, so row actions must not write that path into
  `window.location.hash`.
- Repeated project-row controls expose record-specific Chinese accessible
  names while retaining concise visible action text. The generic
  `subscription` lifecycle label is `申购中` in resource status cells,
  details, and reference selectors.
- Backend precision, active-status, mutually exclusive unlock fields, audit
  transaction, and unlock-fee rules remain authoritative. In particular, an
  enabled unlock-fee asset must equal the project's `quote_asset_id`.

### 4. Validation & Error Matrix

| Condition | Required behavior |
|---|---|
| Quote asset is blank, zero, non-integer, or equals project asset | Keep create disabled and send no request |
| Project asset changes to the selected quote | Clear quote immediately; filter that asset from quote options |
| Active asset has both transfer switches off | Keep it selectable and allow backend creation |
| Asset status is `disabled` | Exclude it from options; backend creation returns validation error |
| Required schedule is blank, malformed, normalized, or non-positive relative seconds | Keep create disabled; boundary helper reports a Chinese field error |
| Optional lifecycle/action time is blank | Return `undefined` and omit the JSON key |
| Optional lifecycle/action time is nonblank but invalid | Report a Chinese field error and make no request |
| Active project is in `preheat` | Offer `开始申购`; PATCH exactly to `subscription` after a reason is confirmed |
| Project is already `subscription`, `distribution`, or `listed` | Omit the preheat shortcut; never offer a skip or replay |
| Preheat project is inactive | Keep the shortcut disabled and send no request |

### 5. Good / Base / Bad Cases

- Good: select active project asset 11 and active quote asset 12 even when both
  have deposit and withdrawal disabled; a fixed local time submits
  `quote_asset_id: 12` and the exact local-time Unix milliseconds.
- Base: confirm listing without a time input; the PATCH body has no
  `listed_at` or `actual_listed_at` key.
- Base: open a preheat project from the list, confirm `开始申购`, and
  render the reloaded row as `申购中`.
- Bad: retain quote 12 after switching the project asset to 12, convert
  `datetime-local` by appending `Z`, or keep all three schedule fields in the
  create payload. Each violates a backend invariant or changes the operator's
  intended instant.

### 6. Tests Required

- Create UI coverage must use active asset response rows whose deposit and
  withdrawal flags are false, select those rows through the rendered controls,
  switch the project asset into a quote conflict, assert the quote clears, and
  inspect the exact POST body. Asserting only the fixture object is insufficient.
- Button coverage exercises all three `unlock_type` values with missing,
  invalid, zero/fractional, and valid current schedule inputs.
- Unlock UI coverage asserts `datetime-local`, exact planned PATCH milliseconds
  and invalid-date rejection. Lifecycle UI sends no timestamps.
- Project-row coverage asserts the exact `preheat -> subscription` PATCH body,
  required reason, post-success reload, inactive/other-stage guards,
  record-specific accessible names, and BrowserRouter pathname/query
  navigation without a hash.
- The focused Rust route test sets both transfer flags false and proves create
  succeeds, while a disabled asset remains rejected. If `DATABASE_URL` is
  absent and the test skips by contract, report that real-database assertions
  did not execute.
- Run the Admin quality gate plus `cargo fmt --all -- --check`,
  `cargo check --all-targets`, the focused Rust route test, Trellis task
  validation, and `git diff --check`.

### 7. Wrong vs Correct

```tsx
// Wrong: stale state can restore a conflicting quote and the local value is
// incorrectly reinterpreted as UTC.
setProject({ ...project, assetId });
body.fixed_unlock_at = Date.parse(`${fixedUnlockAt}Z`);

// Correct: coupled state derives from the latest snapshot and the helper reads
// datetime-local as browser-local calendar components.
setProject((current) => ({
  ...current,
  assetId,
  quoteAssetId: current.quoteAssetId === assetId ? '' : current.quoteAssetId
}));
body.fixed_unlock_at = requiredNewCoinLocalDateTimeMillis(fixedUnlockAt, '固定解禁时间');
```

```tsx
// Wrong: createBrowserRouter does not route a path stored in the fragment.
window.location.hash = newCoinProjectActionsPath(projectId);

// Correct: use the active React Router navigation contract.
navigate(newCoinProjectActionsPath(projectId));
```

## Margin Product Configuration Workflow

### 1. Scope / Trigger

- Applies to create and edit SideSheets for the `marginProducts` resource.
- The workflow follows the operational order `basic -> leverage -> risk -> review`.
  Create and edit reuse one state model and one field component so a product
  cannot be validated or serialized differently between the two actions.

### 2. Signatures

- `支持保证金模式` is an `AdminMultiSelect` over `isolated` (`逐仓`) and
  `cross` (`全仓`).
- `默认保证金模式` is an `AdminSelect` whose options are restricted to the
  currently supported modes.
- Create and update requests send both `margin_mode` and `margin_modes`.
  `margin_mode` is the visible default, and the same value must be the first
  item in the ordered `margin_modes` array.
- The resource table displays separate `默认保证金模式` and
  `支持保证金模式` columns; neither column may substitute for the other.

### 3. Contracts

- Removing the selected default mode immediately falls back to the first
  remaining supported mode. Removing every mode clears the default and blocks
  publication.
- Edit hydration preserves both implemented modes from `margin_modes` and
  reads the default from `margin_mode`. A legacy row without `margin_modes`
  falls back to its `margin_mode`, then to `isolated` only when both fields are
  absent.
- The leverage step combines preset and comma-separated custom levels,
  normalizes equivalent numeric values, removes duplicates, sorts ascending,
  and derives `max_leverage` from the last level.
- The review step summarizes the selected pair and asset labels, supported and
  default modes, leverage levels, risk values, and initial/updated status. It
  states that active publication opens new positions immediately and product
  changes do not rewrite existing positions.
- Tabs remain directly navigable for review, but only one matching
  `role="tabpanel"` is visible. Every panel uses Semi's `semiTab*` /
  `semiTabPanel*` ID relationship. Direct navigation never bypasses final
  validation; the review submit action remains disabled until all prior steps
  pass.

### 4. Validation & Error Matrix

- Missing pair or margin asset -> block next/review submit with a Chinese
  inline workflow error.
- Empty supported modes, or a default outside the supported set -> block.
- Empty leverage set, an empty custom CSV item, a non-decimal level, or a
  level less than or equal to one -> block; never silently discard it.
- `min_margin` missing, non-decimal, or non-positive -> block.
- Non-empty `max_margin` missing decimal syntax, non-positive, or less than
  `min_margin` -> block; an empty value means no upper limit.
- `maintenance_margin_rate` must be a non-negative decimal.
- A non-empty `hourly_interest_rate` must be a non-negative decimal; empty
  means the backend default.

### 5. Good / Base / Bad Cases

- Good: a product supports `[isolated, cross]`, defaults to `cross`, and sends
  `margin_mode: "cross"` plus `margin_modes: ["cross", "isolated"]`.
- Base: a legacy isolated-only product hydrates as supported/default isolated
  and round-trips without introducing cross.
- Bad: filtering edit hydration to isolated or hard-coding
  `margin_modes: ["isolated"]`; editing a cross-capable row would silently
  remove a live business capability.

### 6. Tests Required

- Create coverage selects both modes, changes the default, exercises previous
  and next navigation, reaches the review summary, and asserts the exact
  default-first request body.
- Edit coverage starts with default cross and both supported modes, then
  asserts the same ordered values survive the update request.
- Validation coverage removes the current/default modes, enters invalid custom
  leverage CSV values, inverted margin bounds, and invalid rates; assert the
  Chinese error and disabled navigation/submit state.
- Accessibility coverage asserts one visible tabpanel and matching
  `aria-controls` / `aria-labelledby` IDs. Responsive browser QA asserts no
  document overflow and usable footer controls at desktop and narrow widths.

### 7. Wrong vs Correct

```tsx
// Wrong: hides backend capability and silently downgrades an edited product.
<AdminTextInput readOnly value="逐仓" />
const body = { margin_modes: ['isolated'] };

// Correct: default mode is explicit and owns the first ordered array slot.
<AdminMultiSelect value={values.marginModes} optionList={marginModeOptions} />
<AdminSelect value={values.defaultMarginMode} optionList={supportedOptions} />
const body = {
  margin_mode: values.defaultMarginMode,
  margin_modes: [
    values.defaultMarginMode,
    ...values.marginModes.filter((mode) => mode !== values.defaultMarginMode)
  ]
};
```

## Market Strategy Settings, Versions, Nodes, and Recovery

- The strategy row is a scoped exception to the generic single-line action group:
  constrain `.admin-market-strategy-row-actions` to the existing 288px action
  cell and wrap whole buttons. Do not widen all resources or allow its edit,
  pause and status controls to be clipped outside the fixed column. Verify
  bounding boxes and actual hit targets at 1728px and 1280px.
- One Chinese validation result owns preview/save gating and payload validation.
  Check strict local calendar dates and UTC-minute alignment without silently
  rounding hydrated seconds, ordered interior node times, target/mode enums,
  absolute targets greater than zero, percentage targets strictly greater than
  -100, paired volume bounds and generator ranges. Compare decimal text exactly;
  negative percentages above -100 remain valid. Show the first actionable error
  inline rather than an unexplained disabled button.
- An active detail response disables configuration submission even if the list
  was stale. Opening the editor never changes status. Provide an explicit
  reason-confirmed `paused` status command in active rows, keep disable available,
  and reload the list after status changes. Explain that saving a new version
  does not automatically enable a paused/disabled strategy. Backend transaction
  guards remain authoritative for concurrent status changes.
- Preset application is all-or-nothing. If minute rounding collides or reverses
  nodes, reject the application with a message to expand the range and preserve
  the draft. Never silently drop or reorder a relative-to-previous target.
- Create and edit reuse one market-strategy form and one ordered node editor.
  Each node exposes Chinese labels for target time/type/value, execution mode,
  tolerance, local volatility, and optional paired volume bounds. Use
  `datetime-local` inputs, convert to Unix milliseconds at the API boundary,
  and preserve array order. Add/delete buttons and every repeated field need a
  record-specific Chinese accessible name such as `节点1目标时间` or `删除节点1`.
- The create form loads active trading pairs from the shared admin pair option
  source and renders `交易对ID` as a searchable select whose labels include
  symbol and ID. Only `internal` and `strategy` market types are eligible for a
  synthetic strategy. Render `策略类型` as a select backed by implemented
  strategy types; the current implementation exposes `price_path` as
  `价格路径（OHLCV）`. Do not restore free-text inputs that can submit an
  unsupported pair or inert strategy type. Existing edit records with a legacy
  type may retain that value as an explicitly labelled historical option.
- `/admin/market/strategies` is the single settings entry. It owns list,
  create/edit, presets, OHLCV preview, version history/rollback, node editing,
  status actions, and manual recovery. Do not restore a duplicate navigation
  item or a second `marketStrategyActions` resource config. The legacy
  `/admin/market/strategies/actions` URL may only redirect to the canonical
  route.
- The generator section uses Chinese controls for `行情场景`, `随机种子模式`,
  `固定随机种子`/`当前实际随机种子`, `均值回归强度（0～2）`, `噪声强度（0～5）`,
  `影线强度（0～5）`, and `成交量形态`. Create defaults must match backend
  legacy defaults. Edit initializes every value from detail `generator`, never
  from the list row.
- Presets are loaded once after a create/edit SideSheet opens from
  `GET /admin/api/v1/market-strategies/presets`. An empty or failed response
  must not create a retry render loop; failure stays inline with an explicit
  reload action. Selecting a scenario alone does not mutate price/nodes. The
  administrator must click `应用场景预设` and confirm the replacement summary.
  Applying requires a valid start price and time range; only shape parameters
  (mean reversion/noise/wicks/volume shape) and relative nodes are replaced.
  Preserve exact start/target prices, time range, global ratio/volume boundaries,
  seed mode/value and regenerate command. Cancellation leaves the draft intact;
  preset reference target percentage is not an implicit price command.
- `生成 OHLCV 预览` is enabled only when the complete form is submittable.
  Create preview omits `strategy_id`; edit preview includes the row strategy
  ID so the backend can use the next version and inherited seed. Display total
  minutes, returned sample count, preview version, actual seed, OHLC candles
  scaled over the complete high/low range, and a scroll-contained OHLCV sample
  grid. Do not use a close-only sparkline: it hides abnormal wicks. If sample
  count is smaller than total minutes, label the samples as non-continuous;
  preserve exact decimal values in the grid and extrema labels. Flat/tiny
  candles have finite geometry; invalid rows stay visible in the raw grid with
  an explicit preview warning rather than invented prices. State clearly that the
  regenerate-seed preview seed is ephemeral. Preview never submits a reason or
  creates/updates a strategy.
- Global and node volatility inputs use the existing **decimal ratio** contract
  (`0.01 = 1%`, `1 = 100%`), unlike tolerance/relative targets which use percent
  values. Reuse `MarketStrategyVolatilityField` for the Chinese ratio label,
  example and exact live percent interpretation. Never silently divide a saved
  value by 100 or normalize away a typing draft. `marketStrategyShapeWarnings`
  checks each global/local ratio times wick strength using exact decimal helpers:
  at or above 1, warn about price-floor/large-wick risk without changing the
  payload or disallowing a historically valid configuration. Noise and wick
  strength control independent parts of OHLC. Tests assert precise drafts,
  unchanged payloads, local overrides, warning boundaries, and real SVG wicks.
- Every row exposes `版本历史`. The version SideSheet loads newest first,
  marks the active version in text as well as color, and displays Chinese
  scenario/seed mode plus actual seed, effective time, creation time, and
  creator. A non-active version uses `ConfirmAction` with a trimmed reason to
  call the copy-restore endpoint. Never relabel this as direct activation or
  remove/overwrite the old card after success; reload history and the resource
  list instead.
- Distinguish **current configuration** from **running status**. Version restore
  is disabled for active/unknown status with an explanation. Restore confirmation
  and success say it copies a new configuration without automatically enabling;
  backend locks remain authoritative if status changes concurrently.
- Editing tracks an authoritative baseline: reset/dirty close require explicit
  discard confirmation, cancellation keeps the draft, and reopening reads fresh
  detail. Block close during an in-flight save and protect refresh while dirty.
  Creation retains its draft on same-page close and offers explicit clearing.
- Expired active creation is blocked with an actionable Chinese explanation;
  historical draft creation and preview remain allowed. Recheck expiry on final
  submission, not just when rendering the button.
- Recovery invalidates prior previews when loading a new range, ignores responses
  arriving after close/unmount and checks preview expiry both on screen and at
  execution. No expired token or in-flight range can be submitted. None of these
  UI guards weakens backend verification or authorizes automatic history writes.
- The strategy list response does not contain nodes. Opening edit must first
  load `GET /admin/api/v1/market-strategies/:id`, sort by `sequence_no`, and
  populate the shared form. Never submit the list row's implicit empty array,
  because that would delete configured nodes. The empty editor must explicitly
  explain that the legacy compatibility endpoint remains in use.
- Keep the row action `检测缺口/补偿K线` small, single-line, and named independently of
  nearby view/edit/status actions. It opens one bounded, body-scrolling
  SideSheet headed `检测缺口与补偿K线`; the sheet must not automatically execute a
  recovery when opened.
- The SideSheet interaction is strictly `detect -> select one gap -> preview ->
  enter reason -> execute`. Show half-open range times, missing 1m count,
  config version, affected aggregate intervals, first/last price, token expiry,
  bounded OHLCV samples, and task history with status, actual/expected 1m
  progress, aggregate count, reason/error, and creation time. A no-gap result
  remains a successful explicit empty state.
- Clear any prior preview when gaps are re-detected or the sheet closes.
  Execute sends only the returned `preview_token` and a trimmed, non-blank
  reason. Disable confirmation without both, lock reason/action controls while
  submitting, set `maskClosable={false}`, and prevent Escape/cancel from
  closing during submission. After success, clear the reason and preview, then
  refresh gaps and history.
- Detect/preview/execute failures remain in the sheet context via the shared
  error/Toast treatment; loading and empty states use `aria-live`, and an
  inline persistent failure state, when present, uses `role="alert"`. Every gap
  preview button includes the range identity in its accessible name. Status
  must not be conveyed by Tag color alone; render the Chinese text label for
  `pending`, `running`, `completed`, or `failed`.
- The wide recovery sheet must fit within the viewport, keep its header stable,
  allow its tables to scroll inside the body, and introduce no document-level
  horizontal overflow. On narrow screens, toolbar and node-editor headings
  stack without hiding focus indicators or reducing action target semantics.
- This UI contract does not duplicate backend recovery semantics: API fields,
  preview-token validity, version/gap conflicts, and the no-Redis/no-WebSocket/
  no-checkpoint boundary are owned by
  [Synthetic Market and K-line Recovery Contracts](../backend/synthetic-market-kline.md).

## Agent And Admin Online Support Workbench

### 1. Scope / Trigger

- Apply when changing `/agent/support`, `/admin/support`, their shared API
  client/workbench, navigation, RBAC, queue pagination, or chat history.

### 2. Signatures

```ts
createStaffSupportApi('agent' | 'admin'): {
  listConversations({ status, unread_only, assigned_agent_id, unassigned, limit, offset })
  getConversation(conversationId)
  getMessages(conversationId, { limit, before_id })
  sendMessage(conversationId, { body, client_message_id })
  markRead(conversationId, messageId)
  setStatus(conversationId, 'open' | 'closed')
}
```

The scope binds both the API prefix and request-client authentication scope.
Pages never concatenate an alternate staff identity or agent ID into a route.

### 3. Contracts

- The agent page explains and presents exact-owner visibility only. It has no
  global/unassigned controls and cannot request another agent's queue.
- The admin route/nav requires `support.conversations.read`; mutation controls
  require `support.conversations.write`. Read-only roles can inspect history
  while reply, read, and status actions remain visibly disabled.
- Queue status and admin-unassigned filters execute server queries. The inbox
  uses controlled Semi Pagination: page/page-size map to
  `offset=(page-1)*pageSize` and `limit=pageSize`; changing a server filter
  resets to page 1. Never page only the first fixed 100 rows against a larger
  backend `total`.
- Admin may expose an `仅未读` queue filter backed by `unread_only=true`;
  preserve it across status, assignment, pagination, refresh, and polling.
  Queue summaries distinguish the server `total` from current-page rows and
  may show current-page unread/unassigned counts without presenting them as
  global totals.
- Any keyword filter that is not implemented by the backend must be labelled as
  a current-page filter and must not claim global result coverage.
- Both scopes share an inbox/chat split with neutral surfaces and scoped styles
  in `support/onlineSupport.css`. Inbox rows expose complete customer context
  without a horizontally scrolling resource table; the selected row uses
  `aria-pressed`. Queue and message history scroll independently.
- At <=900px, selecting a conversation shows the chat and a return-to-list
  action. Returning preserves selection, reply draft and failed retry intent;
  opening another conversation retains the existing draft-reset behavior.
  Keep the return action available even if the selected detail is unavailable.
  The composer must remain reachable in short viewports and error states;
  do not clip stacked errors, and avoid document-level horizontal overflow.
- Disable switching conversations while a reply or read/status mutation is
  pending so its response cannot update another conversation's visible state.
- Opening a conversation loads its current metadata and newest message page.
  “Load older” uses `next_before_id`, keeps existing messages on failure,
  deduplicates immutable IDs, and preserves chronological order.
- REST reconciliation runs on a bounded interval and ignores stale overlapping
  queue/detail responses. Unmount clears every timer. A failed silent refresh
  keeps cached queue/history visible with an inline retry.
- A failed reply retains one body/client-message-ID attempt. Retry reuses that
  ID; success clears the composer, merges/reloads persisted state, and does not
  fabricate optimistic backend IDs.
- The queue always shows customer identity, current exact agent or `未分配`,
  status, staff unread count, last-message preview, and time. Message sender
  labels distinguish customer, agent, and administrator.

### 4. Validation & Error Matrix

| Condition | Required behavior |
| --- | --- |
| Admin has read but no write permission | Keep workbench readable; disable all mutations with explanatory copy |
| Server reports total beyond current page | Render controlled pagination and request the matching offset |
| Older-message request fails | Keep loaded history and show an older-page retry |
| Silent poll fails with cached content | Keep content and show a non-destructive warning |
| Reply is blank or over 2,000 scalars | Make no request and show Chinese validation |
| Reply request fails | Preserve the immutable attempt for same-ID retry |
| Selected conversation disappears after reassignment/filter change | Clear or reload selection without showing another agent's stale details |

### 5. Good / Base / Bad Cases

- Good: Agent 8 pages to queue page 2, opens one exact-owned conversation,
  loads older history, replies once, and reconciles the committed record.
- Base: an admin opens an unassigned conversation and replies as administrator.
- Bad: an agent UI sends `assigned_agent_id` to choose another owner's queue.
- Bad: a local table shows page buttons backed only by the first 100 rows while
  its summary displays a larger server total.

### 6. Tests Required

- API tests assert scope-specific prefixes/auth scopes, normalized user contact
  fields, exact request bodies, and message/queue pagination parameters.
- Workbench tests cover agent/admin control differences, runtime write
  permission, page-to-offset mapping, filter page reset, current-page keyword
  semantics, admin unread filtering, older-history merge/retry, unread/status
  actions, same-ID reply retry, stale request suppression, and polling cleanup.
- Route/navigation/access tests prove the admin read permission and agent route
  registration; browser checks cover desktop/narrow layouts, empty/error/long-message
  states, list/chat navigation, draft preservation, reachable actions and zero
  document overflow.

### 7. Wrong vs Correct

```tsx
// Wrong: local pagination can never reach rows after the first fetch.
<DataTable data={firstHundred} />

// Correct: render currentPageRows and drive backend offset/limit with pagination.
<Pagination currentPage={currentPage} pageSize={pageSize} total={total}
  onPageChange={onPageChange} onPageSizeChange={onPageSizeChange} />
```

Backend ownership, persistence, and exact-agent authorization remain defined by
[Agent-Routed Online Support Contracts](../backend/online-support.md).

## Visual Tokens and Accessibility

- Use the shared warm-white, graphite, HIPPO orange, information blue, success,
  warning, and danger roles in `styles.css`.
- Do not use the bright decorative orange for small body text when contrast is
  insufficient. `--admin-color-primary` and its hover token are the
  contrast-safe action/text roles.
- Keep keyboard focus visible on inputs, selects, buttons, navigation, tabs,
  and switches.
- Loading and empty states use `aria-live`; failures use `role="alert"`.
- Honor `prefers-reduced-motion`.

## Administrator Wallet Recharge Command

### 1. Scope / Trigger

- Trigger: confirming an administrator-initiated user wallet recharge or retrying an uncertain response.

### 2. Contract

- Opening a fresh confirmation creates one client idempotency key and freezes user, asset, normalized amount, and trimmed reason with that key.
- Loading state, timeout, dropped response, and explicit retry reuse the same key. Editing any frozen field creates a new logical confirmation and key.
- The request always submits `idempotency_key`; there is no legacy no-key fallback.
- Same command replay shows the original successful result. HTTP 409 means the key was reused with different parameters and must be shown as a conflict, not retried automatically.
- Success closes the confirmation and refreshes authoritative wallet/user data once. A client never optimistically adds the recharge amount.

### 3. Tests Required

- Resource action tests prove one key per confirmation, reuse after an uncertain response, a new key after intent editing, and 409 conflict presentation.
- The UI tests assert amount is transmitted as a decimal string and reason trimming matches the backend fingerprint contract.

For precision metadata, new-intent validation and replay after configuration
changes, follow [Admin Financial Validation](../backend/admin-financial-validation.md).
This also defines convert ratio/storage bounds and the legacy pure-disable path.

## Agent Recharge And Approval UI

### 1. Scope / Trigger

Agent team-user recharge, own-account application history, administrator review
queue, and explicit per-agent direct-credit authorization.

### 2. Signatures

- `/agent/users/:userId/recharge`, `/agent/recharges`
- `/admin/agent-recharges`, agent-management row `RechargePolicyAction`
- API contracts and lock/money rules: [Agent Hierarchy](../backend/agent-hierarchy.md).

### 3. Contracts

- Pin user ID to the chosen team user. New requests require authoritative
  asset precision and exact positive Decimal text, with explicit confirmation.
  Display actual returned state, not the pre-submit opt-in snapshot.
- Freeze user/asset/amount/reason and persist the pending draft plus the shared
  recoverable key before sending. Unknown results, 409 and later 401/403/404
  retain the original request. A surviving draft with a missing/corrupt key
  fails closed; never regenerate a new credit identity. Storage failure blocks
  sending, and successful tombstone persistence precedes key release.
- Review identity is application ID + administrator + decision + reason, not
  a new API idempotency key. Preserve it across remounts. A read-only detail
  query may resolve another administrator's final decision, display its actual
  reviewer/reason, and unlock the queue. Pending/failed queries retain the draft.
- Policy editor loads on demand, not per row. Keep the original CAS version
  on conflict; discard/reload is explicit. Read, review and authorize are
  independently gated. `agents.write` never grants direct-credit authority.
- Own history is account-owned and server-paged, not a local first-page slice.
  Render exact amounts and Chinese mode/status/ledger labels.
- Narrow agent layouts hide the fixed sidebar behind an accessible menu
  toggle; choosing a destination closes it. Desktop navigation stays unchanged.

### 4. Validation & Error Matrix

| Condition | UI result |
| --- | --- |
| Lost response / auth or scope changed after unknown result | Preserve frozen input and key |
| Pending draft but key absent/corrupt | No new POST; require record reconciliation |
| Another administrator already reviewed | Query authoritative detail, no second credit |
| Read-only role | Read data, no mutation control |
| Policy version conflict | Keep old draft/version; explicit reload |
| Invalid record terminal timestamps or review evidence | Contract error, no success acknowledgement |

### 5. Good / Base / Bad Cases

Good: retry a large exact amount after precision tightening using the original
key. Base: agent has no direct-credit permission and sees pending after submit.
Bad: infer credited from a flag, silently reset version, or release identity
solely because a later request is forbidden.

### 6. Tests Required

Real Semi tests cover confirmation accessibility, repeat clicks, remount,
precision, corrupt/unavailable storage, authority changes, independent
permissions, concurrent-review reconciliation and policy version conflicts.
Browser mocks cover 1728/390/320px forms, confirmations and SideSheets. Record
any screenshot-tool failure separately from DOM/interaction evidence.

### 7. Wrong vs Correct

Wrong: change the key after timeout, or clear an unresolved review on 409.
Correct: keep immutable identity, query authoritative state where available,
and only start a genuinely new recharge after the prior outcome is confirmed.

## Required Tests

- `ResizableTable`: a custom handle for every declared leaf, no Semi native
  `.react-resizable-handle`, Pointer and keyboard resizing, min/max bounds,
  fixed-column coverage, `key: 'actions'` classification, row-selection width,
  prop forwarding, and cleanup.
- Standard resource and custom action tables: 24px button height/min-height,
  8px horizontal padding, and computed no-wrap behavior including callers that
  still render a wrapping `Space`.
- `DataTable`: local/server pagination, compact/adaptive modes, selection,
  stable row keys, project resize handles, and numeric scroll-width updates.
- `FilterBar`: controlled-value synchronization, draft submit, blank pruning,
  reset, and loading disablement.
- `ConfirmAction`: trimmed reason, draft reset, ordinary/danger semantics.
- `AdminLayout`: active child group, collapse and restore, navigation.
- KYC / Market Feed / Security Policy: tabpanel semantics and existing API
  payload behavior.
- Market strategy actions: node add/edit/delete and repeated-field names,
  detail-before-edit generator/node preservation, backend preset application,
  generator/seed payloads, create/edit preview context and rendered
  version/seed/OHLCV, immutable history copy-restore with trimmed reason, exact
  millisecond payloads, recovery detect/preview/execute order, submit lock,
  no-gap/error/live states, and status/progress task history.

### Resource-Action Test Runtime

- Keep independent create, detail, edit, and status operations in separate
  cases with reset response fixtures and a fresh QueryClient/render. A list
  mock that always returns the original row is not a persisted lifecycle;
  serializing all four operations into one test only accumulates runtime.
- For tests of completed form values and API serialization, set ordinary
  controlled-input fixtures with `fireEvent.change(input, { target: { value } })`.
  Retain `userEvent` when keyboard/focus/typing behavior is the assertion.
  Large Semi forms containing Quill must not rerender once per fixture
  character merely to populate an unrelated field.
- Keep real Semi/Quill rendering, StrictMode, selection and confirmation
  interactions, exact decimal/multilingual payload assertions, and mutation
  counts. Await the expected list invalidation/refetch instead of sampling
  asynchronous counters immediately or sleeping for an arbitrary duration.
- Fix timeout regressions by removing unnecessary work and isolating behavior,
  not by skipping assertions, mocking away production forms, enabling retries,
  or raising the existing 20000ms timeout. Compare focused timings, repeat the
  affected cases, then run `npm --prefix web test` with the default concurrency
  used by the P0 CI script. `web/vite.config.ts` owns `maxWorkers: 2` for both
  local and CI runs: defaulting to CPU-count-minus-one oversubscribes these
  CPU/memory-heavy jsdom suites. A one-off local worker override is not an
  equivalent shared gate. The runtime-configuration test guards this budget
  and the unchanged timeout.

## Browser Assertions

At 1728px:

```text
document horizontal overflow = 0
every named asset-table leaf column has one project resize handle
Semi native .react-resizable-handle count = 0
Pointer and keyboard resizing update the column and numeric scroll width
resource fixed action column = 288px
fixed action column remains aligned after resizing and horizontal scrolling
medium asset SideSheet = 720px and two form columns
Security Policy visible cards = active tab only
```

At 1280px:

```text
document horizontal overflow = 0
expanded sidebar = 208px
resource filters align as a stable labeled grid
empty state remains deliberate and readable
project resize handles remain keyboard focusable without clipping
```

## Admin Financial Intent, DTO, Permission, Ticker, and Release Contract

### 1. Scope / Trigger

- Trigger: changing an Admin financial command, resource DTO, action control,
  reference selector, live ticker consumer, table, or production bundle policy.

### 2. Signatures

```ts
canonicalRequestIntent(values: Record<string, unknown>): string
runRecoverableFinancialCommand<T>(options: RecoverableFinancialCommandOptions<T>): Promise<T>
apiRequest<T>(path: string, init?: ApiRequestInit): Promise<T>
adminPermissionForRequest(endpoint: string, method: AdminHttpMethod): string | null
subscribeMarketTicker(symbol: string, listener: TickerListener): () => void
useSharedAdminOptionQuery<T>(options): SharedOptionQueryState<T>
```

### 3. Contracts

- Financial command identity includes auth scope, subject, session generation,
  command, user, asset, and canonical intent. Decimal-like fields canonicalize
  as decimal text. The pending/uncertain lease remains in `sessionStorage`
  through timeout, cancellation, response-body loss, and component remount; only
  success or an explicit definitive non-execution failure releases it.
- API requests have one deadline covering fetch and body parsing. Errors are
  typed as timeout, abort, network, HTTP, or contract failures. List DTOs must
  be objects with the declared response key, object rows, non-negative safe
  integer totals, required fields, and decimal text for amount columns.
- Admin read permission is exact per route. Mutation controls resolve the real
  endpoint and HTTP method to exactly one of `write | review | settle | operate`
  and are absent without permission; the backend remains authoritative.
- Shared reference options are cancellable, cached by stable query identity,
  and reused across drawers/pages. Mutations invalidate the owned key rather
  than causing every selector to refetch on mount.
- Ticker consumers share one symbol-normalized connection manager with
  ref-counting, session generation, watchdog, jittered reconnect, and
  `connecting | fresh | stale | offline` diagnostics.
- Every populated table leaf has the project resize handle, pointer/keyboard
  resize, contained horizontal scrolling, and nowrap compact action controls.
  Selected navigation uses an opaque contrast-safe background; a Semi light
  theme background color must not sit behind white selected text.
- Lint, typecheck, all tests, production-policy tests, coverage thresholds,
  production build, and bundle budget all pass before release.

### 4. Validation & Error Matrix

| Condition | Required behavior |
|-----------|-------------------|
| Same financial intent retries after uncertainty | Reuse the exact idempotency key |
| Same key is paired with changed intent | Never send; create a different lease/key |
| Response body drops after server success | Mark uncertain; preserve key for replay |
| DTO row misses a required/decimal field | Raise `ContractError`; do not coerce |
| User has read but lacks mutation action | Render data; omit that action control |
| Reference request unmounts | Abort consumer without committing stale options |
| Last ticker consumer releases | Close socket/watchdog and retain no reconnect timer |
| Selected sidebar item is white on a light inherited background | Release-blocking visual defect |

### 5. Good / Base / Bad Cases

- Good: a recharge times out after the server commits; reload and retry replay
  the same key and display the original result without a second credit.
- Good: nine visible table leaves expose nine resize handles and a pointer drag
  changes the first column while document overflow remains zero.
- Base: a read-only operator can inspect a page but sees no review/settle button.
- Bad: rotate an idempotency key by TTL, coerce `"0.000000000000000001"` through
  `Number`, infer permission from a page label, or open one ticker socket per row.

### 6. Tests Required

- Financial command tests cover canonical decimal identity, timeout, abort,
  body loss, reload recovery, definitive failure, success, and changed intent.
- API contract tests cover required fields, decimal fields, malformed totals,
  error classification, request deadline, and stale-session 401 behavior.
- Access tests cover every route/API mapping and independent action permission.
- Ticker tests cover ref-counting, stale watchdog, backoff, symbol/session change,
  listener isolation, and cleanup.
- Browser checks cover login, selected sidebar contrast, populated table handles,
  real pointer resize, no wrapped action buttons, and 768/1024/1440 overflow.

### 7. Wrong vs Correct

```ts
// Wrong: each retry creates a new command and silently coerces the amount.
await recharge({ amount: Number(amount), idempotency_key: crypto.randomUUID() })

// Correct: canonical intent owns a recoverable key until the outcome is known.
await runRecoverableFinancialCommand({
  scope: financialCommandScopeFromSession(session, 'wallet.recharge', userId, assetId),
  values: { amount: canonicalDecimalText(amount), reason: reason.trim() },
  store: financialCommandIntents,
  request: (key) => recharge({ amount, idempotency_key: key }),
})
```


## Strategy Activation and Runtime Diagnostics (2026-09-06)

### 1. Scope / Trigger

Apply when changing strategy activation, strategy list/detail DTOs or operational
health UI. Configuration `active` is intent, not proof of a successful market round.

### 2. Signatures

- Create `/admin/api/v1/market-strategies` with `status=active` and
  PATCH `/admin/api/v1/market-strategies/:id/status` share activation guards.
- List and detail expose nullable `error_message` and Unix-millisecond `last_tick_at`
  alongside `run_status`, `recovery_status` and start/end timestamps.
- `marketStrategyRuntime(record, now)` derives health; `runtime_health` is a
  `source: derived` column, never a mandatory API key.

### 3. Contracts

- Activating an ended interval (`end_time <= now`) fails before mutation. Historical
  drafts, configuration editing, preview and manual recovery remain supported.
- Serialize activation by locking the eligible active strategy/internal trading pair
  before the strategy. Creation follows the same order. The immutable pair ID may be
  pre-read for locating the lock, but current reads own all validation evidence.
- Reject another active strategy whose half-open interval overlaps the new future
  effective interval `[max(start_time, now), end_time)`. Use a locking/current read
  after acquiring the pair lock; a repeatable-read snapshot must not miss a concurrently
  committed activation. Adjacent schedules and reactivation of the same row are allowed.
- Activation revalidates the trading pair's enabled/source state. Failure leaves
  strategy/run status, events and audit unchanged.
- UI exposes pending start, ended, missing run, actual worker error, inconsistent run,
  pending first successful round, delayed (>15 seconds) and successful health. Never
  use initialized `last_generated_at` or `active` alone to claim a live feed.
- Health is explicitly labelled `加载时`: it describes the loaded API row, not an
  ongoing poll. Refresh replaces the row; do not age an unrefreshed healthy snapshot
  into a false outage. Put diagnostics before secondary price/type columns.
- Expired inactive rows show why activation is disabled and keep editing available.
  A stale browser clock/row cannot bypass the authoritative backend guard.
- Explain that configured volume is per-minute cumulative volume; zero creates no
  synthetic print. Synthetic book/trades are display data, not user fills.

### 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| Expired active create/status request | Chinese validation error; no partial writes |
| Concurrent overlapping activation | Exactly one success, other conflict |
| Adjacent half-open schedules | Both accepted |
| Disabled/external pair at activation | Reject even if the old draft exists |
| Active without last_tick_at | Pending first market round, never healthy |
| Real worker error | Show backend error and last real push time |
| Unrefreshed page | Retain explicitly labelled loaded-snapshot diagnosis |
| Lazy action module still loading | Tests await the actual action, not just page title |

### 5. Good / Base / Bad Cases

Good: a draft can preserve last month's configuration but must extend its interval
before activation. Two concurrent operators cannot turn on conflicting strategies.
Base: a future active strategy shows `待开始` until its configured time.
Bad: list says `启用` therefore claims normal market delivery, hides run errors,
or obtains overlap evidence before waiting for the pair lock.

### 6. Tests Required

- `admin_market_strategy_activation_guards_expiry_overlap_and_exposes_runtime_errors`
  uses real MySQL to verify expiry, concurrent overlap, adjacency, disabled pair,
  error/tick DTO fields and rollback.
- `runtime.test.tsx` covers all diagnostic branches, exact expiry/15-second boundaries,
  activation guards and rendered error/time evidence.
- Resource/action tests await lazy-loaded controls. Run the full Admin quality gate
  and inspect 1728/1280px actual hit targets, expired controls and the edit SideSheet.

### 7. Wrong vs Correct

```text
Wrong: SELECT overlap before lock; active => healthy; hide expired action without explanation.
Correct: pair lock + current read; error/last_tick evidence; actionable expiry message.
```

## Agent New-Coin Applications

- Agent `/agent/coin-applications` combines issuance form and own-account
  application history in `新币发行申请` and `我的申请记录` tabs, defaulting to
  the form. Keep visited panels mounted to preserve drafts and history state;
  lazy-render history so its table is first measured while visible. Submission
  feedback points to the records tab and success refreshes its data.
  Admin `/admin/agent-coin-applications` is a separately
  permissioned review queue; keep exact `.read` / `.review` mappings aligned.
- Use shared DataTable, status filters, server pagination, TimestampText and
  detail SideSheet. Show complete input, submission time, decision/rejection
  reason, review time and published project ID. Approved means preheat published,
  not actually listed; planned time never promises an automatic transition.
- Persist unknown submission intent (full immutable payload and idempotency key)
  in sessionStorage scoped to subject/session generation. Remount reuses the key;
  storage failure blocks sending. Successful response must match submitted fields
  and exact Decimal values before releasing the intent. No Number money adapter.
- Review requires a reason, confirmation and the independent review permission.
  After an uncertain review, block decisions until authoritative detail is read.
  Scope/session changes remount state; abort and ignore old list/detail responses.
- Keep 1728/1280 and 390/320 layouts free of document overflow. Tables retain
  column resizing and contained horizontal scrolling; narrow forms use one column.
- Tests: `CoinApplicationsPage.test.tsx`, access/route tests and full web gates.
  Browser fixtures verify presentation/interaction only; real DB tests own the
  publication, audit rollback, authorization and concurrency guarantees.


## Agent Header Identity

- Header identity comes from `GET /agent/api/v1/me`: preserve `username` and
  `agent_code`, show levels 1–3 as Chinese agent labels, and label the agent code.
  Authentication subjects (including `agent:unknown` for opaque tokens) are
  internal identifiers and must never be used as display-name fallbacks.
- Validate nonblank username/code and supported levels at the API boundary.
  Use neutral loading/error copy with explicit retry; do not fabricate identity.
- Key identity state by agent subject and session generation, cancel on unmount,
  and ignore aborted responses. Token refresh within a generation retains identity.
- Constrain long header text with ellipsis and full-value titles. Keep menu, retry,
  and logout accessible at 320px without horizontal overflow.
- Verify malformed payloads, retry, same-subject session replacement, late
  responses, and narrow/wide layouts. Header changes require no auth-store rewrite.


## Agent New-Coin Team Records

- `/agent/orders/new-coin-subscriptions` (新币申购) and
  `/agent/orders/new-coin-purchases` (新币认购) reuse the read-only team order page.
  Purchase records include v2 realtime recognition and historical post-listing purchases; Admin labels them `认购订单`.
- Project ID replaces product ID; coin symbol replaces trading pair; direction
  and result filters are absent. Preserve server paging, query/session stale
  response guards, resizable contained tables and read-only detail drawers.
- Subscription amounts show quote units and requested/allocated quantities show
  coin units. Legacy settled/refunded nulls display unknown, never synthetic zero.
  Historical available status remains accepted and labeled 已到账（历史）.
- Only row user ownership defines agent visibility. No approval, allocation,
  refund or lock release controls are added to these reporting pages.

## Version 2 Issuance Controls

New applications expose fixed subscription price, market seed/reference prices,
four independent local-time node switches and immutable lock/exit-fee policy.
Review shows the full contract. Approved version 2 projects show five stages
and no legacy configuration/grant forms. Own issued projects use ID cursors;
selected-batch allocation uses buyer-subtree orders independent of issuer.
Each batch has an exact percent/payment/refund preview, reason, saved retry key
and explicit confirmation. A lost response restores the original batch even
when the table selection has changed. No automatic allocation option exists.
See [the backend version boundary](../backend/token-issuance-v2.md).


## Administrator V2 Issuance Management

- Project creation uses `CreateTokenProject` and
  `POST /admin/api/v1/new-coins/issuance`, selecting existing active assets.
  Reuse shared issuance node/lock/fee controls and summaries. Start in preheat;
  forbid manual initial stage selection. The historical creation component/API
  remains separately tested for compatibility and is not the default UI entry.
- Persist the complete intent/key under administrator subject/session generation
  before POST. Unknown failures and contract mismatch keep it locked across
  remounts. Validate asset IDs, canonical supply/price and `policyIdentity` before
  reporting success. Storage failure before sending performs zero requests;
  late results from another generation cannot call completion callbacks.
  Definitive 400/422 or `409 TOKEN_ISSUANCE_EXISTS` permits correction; generic
  409 and transport failures require original-intent recovery.
- Project center shows issuer, actual purchase, next automatic node and persisted
  blocked/retry status. Manual phase actions lock while in flight and after an
  uncertain result; only a successful authoritative reload clears that state.
  V2 uses the read-only `发行合同` tab and project-scoped holdings/buyback/operation
  links. V1 retains the original settings/grant and asset-scoped lock links.
- `持仓与锁仓规则` and `解锁回购记录` reuse strict resource response validation,
  server pagination/filtering, contained resizable tables, exact CSV and detail
  drawers. Show buyer ownership independently from issuer. Holdings explain that
  maturity never automatically sells; original policy remains inspectable.
  Buybacks show cost, gross, deducted fee, net and quote unit, no estimated profits.
  Register holdings/buybacks/operations in `SERVER_PAGED_ENDPOINTS`; otherwise
  `ResourcePage` silently uses local paging. Regression tests must cross the
  50-row boundary and retain `project_id` with `limit=50&offset=50`.
  Set `csvFileName` and export only the loaded page; empty/error states disable
  export. Put explanatory copy in `ResourceConfig.notice`, rendered inside the
  resource page below its header. Import `coinApplications.css` when rendering
  shared issuance summaries outside application pages, including lazy modals.
  Buyback detail metadata labels the quote source/version without changing the
  underlying evidence or relabeling unrelated resources.
- `发行操作记录` requires audit read; preserve actual executor roles and raw JSON
  evidence. Legacy release pages remain under `历史锁仓与解禁`.
- V2 market setup pins pair and seed. An older saved draft with a different seed
  hydrates to the contract seed as an unsaved correction, retaining revision/CAS.
  Read-only users cannot save it. Actual purchase is required before publishing;
  listing preserves the same market history. Initial-price input is disabled.
- Test `CreateTokenProject`, `TokenRecordsPage`, `NewCoinProjectPage` and
  `NewCoinMarketSetup`, plus resource/nav/permission contracts. Browser checks
  include full creation confirmation/retry, record empty/error/rules, fixed seed,
  and desktop/390/320 layouts. See backend v2 spec for transaction and rollout.

## Unified Recharge Operations

Follow [the timed deposit contract](../backend/timed-deposits.md). The wallet
sidebar exposes only `快速充值` and `链上充值` for recharge. Quick recharge hosts
configuration and orders; chain recharge hosts network/receiving configuration,
orders/receipt review and ledger tabs. Old recharge URLs redirect; withdrawal
network configuration remains a separate entry.

Tabs are routed and independently authorized; mount only the selected page.
`wallet.networks` controls node/asset settings, `wallet.address_pool` controls
address reads/writes, and `wallet.deposits` controls orders/evidence. Address-only
operators enter `/admin/wallet/chain-recharge/addresses` and request only the
address endpoint, never network secrets/settings. Navigation falls back to an
authorized related path; read permission alone never exposes write actions.

Select a network once, then configure its node, explicit chain asset mappings
and directly paste 1..500 receiving addresses. Keep the selected network after
save/refetch. No permanent assignment, inventory ID preview, address-group
fallback or separate enrollment form remains. Preview complete address/network
inputs, require a reason, and explain atomic rejection and duplicate preservation.

Never echo node secrets; blank means preserve. Pin revision through failed saves;
retain draft/reason on 409. Use explicit network, multi-asset and address lists
with independent SideSheet editors, not a global implicit dirty lock. Opening
an editor pins its network, record and revision; cancel/discard protects local
changes and navigation. Refresh is unavailable while an editor/save is active.
Failed refresh disables new writes without erasing a pinned draft. New asset
selection uses the minimal platform catalog, excludes already mapped symbols
and explains platform recharge disabled status. ETH/18 (Ethereum/Base), BTC/8
and SOL/9 are fixed native presets; Tron offers only TRC20. Manual token entry
never guesses contracts or decimals. Explicit, source-verified mainnet presets
may populate new asset drafts for Ethereum, Tron and Solana; exclude mapped
symbols and disable choices absent from the platform catalog. Changing the
platform symbol clears the previous token identity. Node presets only populate
endpoint/confirmation drafts, preserving admission state and secret semantics.
Block API_KEY placeholders even when admission is disabled because configured
nodes still scan history. Presets never save automatically; preserve revision,
reason and discard guards. The two preset selects opt out of popup motion to
avoid a leave/reopen race that strands an expanded select with an empty portal;
other shared selects retain their default. Browser checks must repeatedly switch
Solana node and asset presets within the same editor. Existing identity is copyable read-only text with a reason;
only its enabled state is editable. Native identity is not an empty disabled
contract input. Semi Modal action buttons need explicit Chinese aria-labels;
Select visible labels use aria-labelledby. `接受新充值订单` controls admission, not
historical scanning; disabling addresses/assets preserves in-flight processing.

The configuration guide orders node access, explicit assets, receiving addresses
and scan/admission readiness. Each network/asset method lists all blockers:
disabled mapping, absent/inactive/non-depositable platform asset, paused network,
no enabled address, or missing/error/waiting/stale scanner. Read `scan_fresh`,
`cursor_height` and `checked_at` from the strict backend snapshot; never infer
freshness from browser time or equate the admission switch with scan success.
Loading/error snapshots show unknown readiness, not stale green status.
Use `slot` for Solana progress. The setup guide is absent for address-only roles.

Alchemy/Helius presets expose a separate masked provider key input. Trim and
URL-encode it into the selected template, then save through the existing encrypted
endpoint API. Do not copy it into the Bearer field. Switching presets clears this
draft key; manual endpoint edits clear preset selection/key. Placeholder guards,
explicit save, revision and discard semantics remain mandatory.

Refresh every 15 seconds only for the full settings view while visible and idle.
Pause during editors, address confirmation dialogs, reads and writes.
`ConfirmAction.onOpenChange?: (open: boolean) => void` notifies open/cancel/success;
failed confirmation retains the dialog and paused refresh. Its optional nature
preserves other call sites. Tests cover pause/resume and failed confirmation.
Keep the guide at four/two/one columns for desktop/tablet/narrow layouts. Scope
summary flex rules to its text block; Semi Tag is also a `div` and must not stretch.

Search actual `order_no`; display internal IDs only as labelled action references.
Review never preselects a candidate as proof of ownership: read an explicit target,
show actual amount/fee and chain identity, and require a reason. Backend matching
and confirmation checks remain final. Preserve desktop/390/320 layouts, contained
horizontal table scrolling and accessible labels. Tests cover permission-specific
tabs, direct import payloads, revisions, drafts, secrets and unchanged quick recharge.
At narrow widths, stack the Card header title above its action. Semi Card places
the action inside `semi-card-header-wrapper`, so wrapping the outer header alone
does not prevent title truncation; pass a non-ellipsis Typography title explicitly.


### Receipt attribution lookup

Deposit review accepts internal positive safe IDs and exact public CD order
numbers. Order-number lookup must return exactly one exact match. Clear the
previous target before every read and invalidate late responses when input
changes or the panel unmounts. Candidate IDs are explicit read actions, never
automatically selected payment owners. Show identity/address/time/confirmation
blockers. A SideSheet contains evidence and target preview; a separate
ConfirmAction requires the reason before credit/ignore. Freeze target controls
and closing during confirmation/submission, prevent duplicate writes, and
preserve backend atomic revalidation. Cancelled orders are eligible for explicit
manual attribution only, with the cancellation history retained by the backend.

### Deposit provider pools

Once a network has providers, the scanner SideSheet edits the full pinned-revision
provider/key snapshot. Names, priorities, enabled flags and request intervals remain
explicit. Secrets start blank; replacement and clearing differ. URL placeholders
are allowed only in provider endpoint templates and require credentials. Initial
single-node setup creates the default channel; future edits use the pool editor.
Add/remove changes remain drafts until the reason confirmation succeeds. Errors
and revision conflicts preserve every draft. Runtime provider/key cooldown times
come from the DB snapshot, not client inference, and never replace network-level
readiness. Disabled/unknown/all-key-cooling states must not say healthy. Reuse Semi
controls, responsive fieldsets, masked inputs and the existing navigation guard.

### Scanner recovery controls

Network cards offer a network-write-only “恢复到最新区块” SideSheet when a saved
endpoint/checkpoint exists and no request is pending. Default to dual progress;
skip requires a separate risk switch acknowledging that both the skipped interval
and existing backfill will no longer be detected automatically. Both modes use
ConfirmAction with a mandatory reason, pinned network/revision, draft protection
and error retention. Switching modes resets the skip acknowledgement.

Show queued mode and separate live/history heights, fixed historical target and
errors from the strict ScannerView. Explain that history transport retries differ
from blocked history requiring reconciliation. Solana uses slot labels. Do not
claim reception is ready solely because recovery succeeded. Refresh is passive
and pauses while editing; recovery is never triggered by opening or refreshing.
Tests cover reason/payload, revision conflict retention, skip gating, queued state,
reader permission and missing checkpoint. Browser checks use real local HTTP at
1728/390/320 widths with isolated data and a simulated node.

Recovery SideSheet titles must allow shrinking (`flex: 1 1 0; min-width: 0`) and
wrapping while the close action keeps its width. Verify the actual close-button
bounds at 320px; document-level overflow alone misses clipped overlay controls.
