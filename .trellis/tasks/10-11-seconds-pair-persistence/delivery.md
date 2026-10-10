# Delivery

## Root Cause And Fix

SecondsView stored selection only in a component ref. Remounting initialized it
to null and selected the first API product. A request also captured the current
ID before awaiting products, allowing late responses to undo a newer choice.

The navigation store now persists a dedicated seconds product ID. SecondsView
resolves the current/saved ID against the latest API products after the request,
then remembers the valid result. Empty and failed responses retain preference;
missing products fall back; unavailable storage keeps in-memory navigation usable.
No order fields or quote data are persisted. Spot/contract preferences are separate.

The confirmed extension adds `/seconds/:symbol?` with canonical `BASE_QUOTE`
addresses. URL identity takes priority; bare and legacy routes restore memory.
Picker changes and canonicalization replace history. A dedicated selection
composable handles route changes and pending-selection/request races.
The shell reuses SecondsView across symbol changes, and accessibility completes
on nextTick. `/seconds/history` retains its own static route.

## Verification

- Seven behavioral tests execute the real store, selection composable, router
  and extracted SFC functions. Cover route priority, canonicalization, legacy
  entries, history separation, replacement, pending choice and invalid products.
- Final `npm --prefix mobile run release:gate`: passed, 824/824 tests, no skips,
  production/test type checks, PWA/Tauri builds, artifacts and all budgets.
- Initial gate caught a three-line source budget excess; redundant sparkline
  branches were unified and the final full gate passed without increasing budgets.
- Ego TaskSpace 9: local app at port 5186, anonymous product list initially ETH,
  selected BTC via picker, verified persisted ID 1, performed cache-bypassing
  browser reload, confirmed BTC remained selected at `#/seconds`. Space closed.
- Ego TaskSpace 10 after symbol-route extension: direct ETH URL wins over stored
  BTC; picker changes to BTC URL with unchanged browser history length and exact
  same Seconds DOM root, one main landmark and main-content ID. Cache-bypassing
  reload keeps BTC. History route and Back return to BTC; bare route reload
  canonicalizes to remembered BTC. Space closed. No login or financial mutation.
- Four existing source-contract tests were updated from the old literal route
  to the new optional-symbol declaration; behavior remains covered separately.
- No production deployment, installed-device PWA or native package test performed.

## Proposed Commit

`fix(mobile): preserve seconds selection in symbol routes`

Include navigation store, selection composable, SecondsView, router, shell and
accessibility lifecycle, corresponding regression tests and this task.
For shared navigation spec and PROGRESS, include only this task's additions.
Exclude prior wallet localization, provenance labels, PC changes and .vite cache.
Local commit only; no remote push or deployment.
