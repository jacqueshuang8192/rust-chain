# Agent Team Orders Delivery

## Implementation

- Four lazy portal entries at `/agent/orders/seconds|margin|earn|spot`.
- One authenticated read-only backend route `/agent/api/v1/orders/:kind`.
- Rows and COUNT use the same builder, independently constrained by the
  authenticated materialized path. Selected branch scope can only narrow it.
- Shared financial DTO validation preserves Decimal text, required nullable
  fields, safe identities and millisecond timestamps.
- Common and expandable advanced filters, server pagination, reload, localized
  detail snapshots and local-only seconds countdowns.
- Session/kind remount plus request ownership prevent stale financial display.
- No migrations, dependencies, financial commands or production changes.

## Verification

- `cargo check --all-targets`: passed.
- `DATABASE_URL=mysql://root@127.0.0.1:13316/agent_team_orders_test cargo test
  --test agent_routes team_orders -- --nocapture --test-threads=1`: 3/3 passed.
  Actual MySQL9.3, not a skipped/no-DB run or production/MySQL8.4 validation.
- Full agent routes on that database: 42/44 passed, all three new tests passed.
  Existing dashboard fixture inserts two agent-owned invite codes, violating
  `uk_invite_codes_agent_owner`; existing invite test expects a second create
  to change limit 10 to 25, contrary to the fixed-code contract. These old
  test bodies and invite implementations are unchanged by this task.
- `cargo test --lib --test backend_architecture --test backend_documentation
  --quiet`: 513/513 + 11/11 + 1/1 passed.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- Web API/model/navigation/portfolio/FilterBar tests passed except first-round
  new component assertions, since corrected. Final new page tests: 16/16.
- `npm --prefix web run typecheck`, `lint`, `build`: passed.
- First Web full run: 1049/1050, one new filter-button accessible-name assertion.
  Fixed by hiding the decorative icon from the accessible name; rerun 1050/1050.
  Final browser review found untranslated spot directions; reused the existing
  field-scoped label helper and added six direction regressions. Final full
  rerun: 109 files, 1056/1056 tests passed.
- `npm --prefix web run test:production-policy`: 15/15 passed.
- `npm --prefix web run test:coverage`: 23/23 passed; statements 85.61%,
  branches 81.78%, functions 85.33%, lines 92.87%. This is the repository's
  existing four-file coverage gate, not coverage of all new order code.
- Final typecheck, lint and build after direction localization: passed.
- Final build budget: 2,641,661 raw JS bytes > 2,600,000. Baseline before this
  task was already 2,630,506. No threshold increases or release bypasses.
- Task context validation and `git diff --check`: passed. Codex inline mode
  loads specs directly; no delegated context entries are required.

## Browser

Ego TaskSpace 3, Vite `127.0.0.1:5198`. All API-shaped requests are intercepted
by `browser-mock.js`, unknown requests and writes refused. Logged production
origin URLs are client request strings only, not requests sent to production.

- Seconds: all 14 project resize handles; real pointer drag 200 -> 248px;
  user filter sends `user_id=42`, shows 12 mock rows; no polling from countdown.
- 1728px: document/content overflow zero, populated rows and four menu entries.
- 390px: document/content overflow zero; common filters show initially and
  table top is approximately 567px rather than the initial 1191px.
- Earn: 8.00% display from 0.08 string; localized type-specific detail names.
  320px drawer width equals viewport, no raw keys or document overflow.
- Margin: pending (null entry) is labelled correctly in list and detail;
  no raw detail keys at 1280px.
- Spot: populated and empty states at 1280px, Chinese details, and all 12
  filtered buy-direction tags localized. No raw `buy` tags remain.
- Advanced filters: `agent_id=3` returns 12 fixture rows; collapse retains
  `more filters (1)`, and submitting again still sends the branch ID.
- Final 320px spot: document and content overflow both zero, table scroll
  contained, common filters and first rows visible. Writes/unhandled requests
  remain empty. No production API requests or financial mutations.
- TaskSpace 3 finished once with no retained pages. Existing Vite server remains
  available at `http://127.0.0.1:5198` (HTTP 200). The preview needs a backend
  running the new route for real data; mocks do not certify live integration.

Screenshots inspected:
`/tmp/agent-orders-desktop.png`, `/tmp/agent-orders-mobile.png`,
`/tmp/agent-orders-detail-320.png`.
Additional inspected screenshots: `/tmp/agent-orders-earn-detail-final.png`,
`/tmp/agent-orders-empty-1280.png`, `/tmp/agent-orders-spot-320.png`,
`/tmp/agent-orders-spot-desktop-final.png`.

## Change Boundary

This task owns `team_orders` modules/route wiring, the new OpenAPI module,
agent portal refunded schema correction, agent route tests, team order Web
files and fixture, agent financial contract extraction, navigation, datetime
and labelled FilterBar controls, agent hierarchy/order-number specs and this
task's progress additions.

Earlier realtime/logging/console work, Mobile/PC changes, shared detail
redesign, and all other existing dirty files are excluded. PROGRESS contains
prior changes; do not stage its whole file for a task-only commit.
No commit, push or deployment has been performed in this continuation.
See `commit-plan.md` for the proposed single-task commit and excluded work.

## Remaining Release Work

- Resolve the pre-existing Web bundle-budget excess without raising the limit.
- Reconcile the two pre-existing invite tests with the fixed-code contract.
- Confirm the proposed task-only commit before staging or committing.
- Validate real end-to-end behavior with a backend running this route; browser
  fixtures and MySQL9.3 tests do not certify production/MySQL8.4 deployment.
