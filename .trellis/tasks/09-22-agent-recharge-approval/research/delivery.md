# Agent Recharge Delivery

## Business Behavior

- Agent team users expose recharge; own application history is in the sidebar.
- Default path requires administrator review in the agent-recharge queue.
- Agent management exposes independent direct-credit authorization. It applies
  only to that agent and never auto-credits existing pending applications.
- Funds enter the target user's Spot available wallet; the agent wallet is
  not charged. This is manual platform funding, not verified external custody.
- No real recharge, permission change, production migration or deployment was
  performed. Previous new-coin market work remains intact and uncommitted.

## Frontend Verification

- Final full Web suite: 101 files, 969/969 passed.
- Typecheck, lint and production build passed after final UI changes.
- Production-policy: 15/15. Existing coverage gate: 23/23; statements 85.61%,
  branches 81.78%, functions 85.33%, lines 92.87%.
- Tests exercise exact large/tiny Decimal strings, no double submission,
  immutable remount recovery, precision tightening, auth/scope failures,
  corrupt/missing/silently failing storage, strict terminal-state evidence,
  read-only permissions, review conflict reconciliation and policy CAS.
- Added narrow agent menu state/navigation regression; real responsive CSS
  is checked in browser DOM because jsdom does not emulate media layout.
- Mobile ledger/i18n tests: 19/19; PC transaction/date tests: 2/2.
  Both client typechecks passed. No complete Mobile/PC release gate rerun was
  needed for this additive ledger label/classification change.

## Browser Evidence And Limitations

- Local browser fixture: `browser-mock.js`; all `/api/` calls are intercepted
  and never forwarded. Credentials are placeholders confined to this test.
- Browser source fixtures and real React/Semi views were used, not fake HTML.
- Verified pending submission of `9007199254740993.000000000000000001`,
  one administrator approval and one explicit per-agent opt-in.
- Verified direct submission of `0.000000000000000001`, exact string POST,
  `direct/credited` result and no extra review POST.
- Verified explicit revocation returns the policy to review at version 2,
  without executing the remaining pending record. Narrow-menu navigation
  opens the sidebar, reaches own history and closes the sidebar.
- At 1728/390/320px, document horizontal overflow was zero for confirmation,
  review and policy editors. Modal widths were 560/358/288px; SideSheets
  640/358/288px. Narrow agent form now uses the content width, not the old
  fixed-sidebar remnant. Long amounts remain intact in the DOM and receipt.
- Browser screenshot APIs repeatedly timed out with `Page.captureScreenshot`;
  no successful screenshot is claimed. Some narrow pointer actions failed
  through the browser wrapper; keyboard activation of the same real controls
  succeeded. Complete visual/pointer acceptance still requires follow-up.
- The Vite browser log also contained generic `Script error` events without
  application stack detail, plus the existing hydration-fallback warning;
  this run does not claim a clean browser-console acceptance.
- These mocked interactions do not certify browser-to-real-backend funding
  end-to-end, real mobile devices, or external payment/custody integration.

## Backend Verification

See `backend-tests.md` for final real-MySQL commands and detailed concurrency
coverage. Final main-session unit suite: 503/503, numeric contract 1/1.
Real agent routes: 41/41 including 18 new recharge scenarios.
OpenAPI: 12/12; architecture: 11/11; documentation: 1/1.
The final added case validates the real financial-idempotency audit route:
valid credit is not orphaned; mismatched amount or pending/rejected references
are flagged, and the read route changes no money. Wallet Funding classifier
regression passed. Full-target/full-feature strict Clippy, full fmt, diff check,
source-integrity scan and its 16 unit tests passed.

## Environment Cleanup

The dedicated MySQL 13316 was shut down through its explicit task socket;
the listener exited and its PID file was removed. Both the current and
prior agent-owned recharge browser verification spaces were closed. Generated
Python bytecode was removed; existing `.vite/` and unrelated edits were kept.
Preview remains available on `http://127.0.0.1:5198/` (HTTP 200, detached
process 80706). The original 5178 service was not stopped or reconfigured.
No active verification command remains running. The preview does not include
mock credentials/data and still needs the configured backend and real login.

## Release Gate Remains Open

- Final bundle total JS: raw **2,608,723** bytes vs **2,600,000** budget;
  gzip **769,563** vs **750,000** budget.
- Previous task gzip baseline was **758,364**; this task adds **11,199** bytes
  to that recorded total. The raw budget now also fails.
- No budget/coverage threshold was raised and no test was skipped to conceal
  this. Feature behavior passing does not imply a releasable combined build.
- Keep A04/task open until budget and remaining browser acceptance are handled.

## Rollout Requirements

1. Rehearse migration 0144 on production-version MySQL 8.4 and backup/recovery.
   Local migration/concurrency verification used isolated MySQL 9.3.0 only.
2. Apply prerequisite migrations in normal order. The working tree also
   contains pending new-coin migrations 0142/0143 from the previous task.
3. Apply 0144 before deploying routes that read its tables/agent columns;
   there is no pre-migration compatibility fallback.
4. Review role grants: queue `agents.recharges.read`, review
   `agents.recharges.review`, policy read `agents.read`, policy change
   `agents.recharges.authorize`. Keep opt-in false until explicitly authorized.
5. Cross-workflow stress against legacy agent creation/reassignment lock
   ordering remains untested. No automatic deadlock retry or historical
   balance repair was introduced.
6. Resolve bundle/visual acceptance before release; do not auto-deploy this
   uncommitted work or mistake preview availability for production readiness.
