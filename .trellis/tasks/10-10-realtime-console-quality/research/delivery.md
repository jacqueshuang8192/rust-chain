# Local Delivery: Realtime Settlement And Console Quality

## Diagnosis And Changes

The previous Redis-price change is preserved: opened orders use the current
cached positive price without rejecting an old observation. This change does
not replace that pricing policy, rewrite historical evidence, or change payout,
manual-review or refund rules.

The inspected code had several latency amplifiers:

- One settlement batch ran sequentially. The joined locking projection could
  lock the shared pair/user rows as well as each order.
- Failed earliest-expiry orders could repeatedly occupy the batch head.
- Each full batch waited for the next interval even with a backlog. A local
  untracked `.env` still overrode the one-second default with five seconds.
- Mobile lacked a seconds private-stream consumer. PC had a private consumer,
  but settlement display still used per-order three-second polling.
- PC's result adapter also fetched the entire order list for each queried
  order. Final review replaced that N+1 path with one snapshot per round.

Automatic settlement now uses four bounded concurrent attempts, order-only
locking, candidate and per-order `SKIP LOCKED`, retry-time ordering, two-second
Redis GET deadlines, and up to ten consecutive full progressing batches.
Completed transactions and hints are drained even if a retry write fails.
Only local `.env` was changed to one second; no deployment configuration was
changed. Logs expose effective interval and batch elapsed milliseconds.

Mobile reuses the shared authenticated seconds topic; PC adds open/reconnect
callbacks, heartbeat and stale-socket repair. Events trigger coalesced REST
reconciliation, not event-derived wallet credits. In-flight event bursts retain
one follow-up. Token refresh preserves Mobile order tracking; true identity
changes reset it. PC reads reject old symbols/tokens/unmounted consumers.
One PC snapshot updates active/history rows and returns newly completed tracked
orders for the existing result dialog. Refund/review rows remain neutral.
Countdowns are local; low-frequency recovery remains because the hub is lossy
and process-local, not a cross-instance Redis bus.

API and migration programs now share readable Chinese single-line logging:
timestamp, level, message, named fields, technical module. Common fields are
localized; known sensitive fields are hidden, URI passwords sanitized, and
request spans omitted. Idle workers no longer flood INFO. Technical identifiers
and underlying third-party errors remain intact; this is not a promise that
every character emitted by dependencies is Chinese.

Agent pages use bounded form widths, visible field labels, 36px desktop/44px
mobile control wrappers, sensible table widths and local horizontal scrolling.
Agent overview/convert counters display integers in compact four-column desktop
and two-column narrow grids without the inherited decorative circles.
Shared administrator details use semantic two-column fields, narrow single
columns, nested item boundaries, wrapping evidence and raw-value titles.
Known evidence/refund/retry/agent/wallet keys and enums have Chinese labels.
Unknown values and free text are not guessed or rewritten.

## Owned Files

- Seconds backend: `src/modules/seconds_contract/infrastructure.rs`,
  `src/workers/seconds_contract_settlement.rs`,
  `tests/seconds_contract_settlement_worker.rs`.
- Mobile: `src/core/privateUserStreamManager.ts`,
  `src/composables/useSecondsPrivateReconciliation.ts`, `src/views/SecondsView.vue`,
  `tests/private-user-stream.test.ts`, `tests/seconds-live-multi-orders.test.ts`,
  `tests/seconds-private-reconciliation.test.ts` under `mobile/`.
- PC: `src/api/stomp.ts`, `src/api/second.ts`, `src/stores/second.ts`, `src/views/SecondOptions.vue`,
  `tests/stomp.test.ts`, `tests/seconds-reconciliation.test.ts`,
  `tests/guest-auth-states.test.ts` under `pc/`.
- Logging: `src/infra/logging.rs`, `src/infra/mod.rs`, `src/main.rs`,
  `src/bin/exchange-migrate.rs`, `src/migration.rs`,
  `src/modules/prediction/application.rs`,
  `src/modules/quick_recharge/application.rs`,
  `tests/unit_src/src_infra_logging_tests.rs`, and log-only changes in workers
  `agent_commission_settlement`, `earn_auto_redemption`, `loan_overdue`,
  `margin_interest`, `margin_liquidation`, `synthetic_market`, `unlock_scanner`,
  `wallet_chain`.
- Web: `src/agent/pages.tsx/.test.tsx`, `src/agent/UserPortfolioPage.tsx/.css`,
  `src/layouts/AgentLayout.css`, `src/shared/DetailDrawer.tsx/.css/.test.tsx`,
  `src/shared/adminResponseFieldLabels.ts`, `src/shared/adminEnumLabels.ts`,
  consumer tests `src/admin/resources/AdminResourcePage.test.tsx` and
  `src/admin/actions/AgentManagementPage.test.tsx`.
- Specs: backend logging/index/seconds/realtime and admin Chinese presentation,
  this task directory and `docs/superpowers/PROGRESS.md`.

Other chart, default-theme, Pencil, PWA, lockfile and cache working-tree changes
are pre-existing and excluded. Nothing is staged, committed, pushed or deployed.

## Verification

| Check | Result |
| --- | --- |
| Isolated MySQL 9.3 + Redis settlement worker | 11/11; includes limit-one locked head, retry fairness, rollback, replay |
| Isolated seconds routes + principal refund | 28/28 with `--test-threads=1`; refund uses its dedicated database |
| Rust library + architecture + documentation | 513/513 + 11/11 + 1/1 |
| Backend WebSocket routing/isolation | `cargo test --test events_ws --quiet`, 16/16 |
| Rust all-target/all-feature strict Clippy, fmt | Passed |
| Mobile `release:gate` | Passed: 804 tests, production/test types, PWA/Tauri builds, artifact/bundle/source/test-quality budgets |
| PC all tests, type-check and production build | Passed: 118/118, including snapshot request count, event-burst merge, stale reads and neutral refunds |
| Web all tests | Final rerun: 106 files, 1000/1000 |
| Web production policy / coverage | 15/15 and 23/23, thresholds passed |
| Web typecheck / lint / build | Passed |
| Web bundle budget | Failed: raw JavaScript 2,630,506 > 2,600,000 bytes; existing baseline was already over budget; threshold unchanged |
| Task context / diff whitespace | Passed |

The first fully parallel route run returned 26/28: one cache-age opening got
HTTP 500, and the capacity test observed MySQL 1213. Serial fixture execution
passed all 28, including its own 32-concurrent-open scenario. This is not proof
that every production deadlock is eliminated; broad concurrent fixture
interference remains a validation limitation. No new test skip or timeout
increase was introduced.

The first Mobile gate caught the enlarged SecondsView source budget. Moving the
new event lifecycle into a tested composable fixed it without raising limits.
Web consumer tests were updated from retired field/content table headers to
semantic labeled fields; PC source assertions now require lifecycle guards.
The final integer-counter change required updating the existing `15.00` test
expectation to `15`. Unsupported `exact` options on role queries were removed
after type checking caught them; the final Web production build passed.

## Browser Evidence

Ego TaskSpace 2, local Vite at `http://127.0.0.1:5198`, with the read-only
`browser-mock.js` fixture. Mocks intercept application GETs and reject writes.
These checks are not live-API, real-login, native-device or financial E2E tests.

- Agent users: 1728/1280/390/320px, zero document overflow, eight resize handles.
  Actual keyboard and pointer resize changed the first header from 108px to
  124.4375px to 171.421875px.
- Empty invite page: same four widths, zero overflow; wrapper height 36px on
  desktop, 44px on narrow screens, input draft retained after resizing.
- Agent portfolio: wallet empty state and populated seconds tab. Status selector
  is 240px wide on desktop and fills 366px/296px on 390px/320px; no overflow.
- Administrator seconds detail: 780px desktop sheet with two columns, full-width
  390px/320px sheet with one column. Document and detail overflow both zero.
  Raw decimal evidence remains available; unknown keys remain visible.
- Agent dashboard and convert stats: 1728/1280/390/320px, zero root and layout
  content overflow. Compact counters retain exact integer presentation while
  financial amounts retain existing formatting.

Final overflow measurements use `scrollWidth - clientWidth` on both the document
and actual content/detail containers. Earlier `scrollWidth - innerWidth` readings
could mask scrollbar-width overflow and are not the final evidence. Rechecking
users, invite empty state, portfolio wallet empty state, dashboard, convert stats
and administrator detail with the corrected metric returned zero. The populated
portfolio seconds tab was also rechecked: zero root/content overflow, 14 column
handles and a 240px desktop / 366px at 390 / 296px at 320 status selector.

Inspected final screenshots include `/tmp/console-quality-dashboard-final-1728.png`,
`/tmp/console-quality-dashboard-final-320.png`,
`/tmp/console-quality-agent-convert-320.png`, and
`/tmp/console-quality-admin-detail-final-320.png`.

Representative screenshots use `/tmp/console-quality-*.png`; transient screenshots
are not build inputs. An early browser attempt used the API-style plural page
path and another waited for the wrong button label; neither is counted as a
successful check. The successful checks used the registered singular route and
the observed `查看详情` control. A final portfolio wait targeted a business order
number that this table does not render; after inspecting the visible ID/pair row,
the correct populated-table check passed. No mock write or unhandled request
occurred in the successful verification runs.

TaskSpace 2 was finished with no retained pages. The existing Vite preview remains
available at `http://127.0.0.1:5198/` (HTTP 200 confirmed). Database/Redis services
that existed before the task remain untouched.

## Release Boundaries

1. No production timing measurement or deployment was performed. Confirm
   `SECONDS_CONTRACT_SETTLEMENT_INTERVAL_SECONDS=1`, worker enabled, Redis/MySQL
   connectivity and actual order timings on the deployed instance.
2. Private hints are still process-local. Separate workers/API instances can
   need the bounded REST fallback. Durable/cross-instance delivery is not added.
3. Web bundle budget is still a release blocker. No unrelated bundle rewrite
   or budget relaxation was made.
4. The broad live Admin login/KYC/security-policy browser matrix was not rerun.
   Shared component tests passed; browser scope was the affected representative
   agent/detail surfaces. MySQL 8.4 itself was not executed locally.
5. Independent seconds history routes retain their existing load-on-entry
   behavior; this change covers Mobile trading and PC active/history tabs.
