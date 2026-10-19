# Delivery: Mobile new-coin phase alignment

## Changes

- Distribution is named consistently in list filters and detail milestones;
  filters follow subscription → distribution → live purchase → listing and
  remain one scrollable row at narrow widths.
- Detail explains the actual phase and offers project refresh plus distribution
  records. Scheduled rule dates and the card listing time are labelled planned.
  Removed timezone/automatic/manual explanatory copy stays hidden.
- Public project state refreshes while visible every 10 seconds and on return.
  Older requests cannot overwrite a newer symbol, manual refresh or unmounted
  view. Cached stages survive errors. Legacy confirmations retain their snapshot.
- Subscription forms retain unknown receipts across phase changes while refusing
  new subscriptions after closure. Backend idempotency and settlement remain unchanged.
- v2 cards display the contract's six lock variants instead of legacy fallback
  fields; the progress label now says subscription, and records cover all new-coin activity.

## Verification

- `npm --prefix mobile run release:gate`: exit 0, 860/860 tests, both type checks,
  PWA/Tauri builds and artifact checks, bundle/source-size/test-quality budgets.
- Six added tests execute actual composable/SFC setup for stage refresh, hidden
  and paused pages, latest-only reads, errors, unmount, uncertain subscription
  retries, closed phases, and all v2 card lock variants.
- Browser in-memory fixtures: distribution → purchase reveals chart/buy entry;
  subsequent automatic refresh to listed hides buy and reveals spot entry.
- 12 detail combinations: Chinese/English × light/dark × 320/390/448px, no page
  overflow, four planned labels, hidden copy absent, action targets >=44px.
- Final 320px list: six correctly ordered filters in one row, working distribution
  filter, fixed-time lock from v2, no page overflow. Legacy detail: four stages,
  no synthetic purchase/chart, blocked purchase controls, no page overflow.
- Inspected final screenshots under `/tmp/new-coin-*.png`. Browser fixtures made
  zero writes; production inspection used public reads only. No server deployment,
  allocation, stage mutation, migration or money movement was performed.
- `git diff --check` passed. New CSS initially exceeded the existing budget;
  reusing the existing token surface removed the duplication, and final budgets
  pass without threshold changes. A development HMR overlay required reloading
  the fixture; final validation used a fresh local server with no overlay.

## Delivery state

Implementation and validation complete. Business commit `d467cf4` created; remote push pending. No deployment.
Existing `.vite/` cache is excluded. Task space 26 and the task-owned 5187 Vite
server were closed; pre-existing development servers were preserved. Ego Lite
reported an available update; it was not installed.

For KHB, frontend deployment improves the explanation and freshness only.
Actual purchase still requires settlement of pending subscription obligations;
the configured listing node remains manual.

## Commit result

Business commit `d467cf4eca9d8c76be731b2d7801e151a3b1c598` is recorded. Task archive and journal follow before a normal remote push. Latest combined Mobile gate passed 871/871; no business source changed during submission.
