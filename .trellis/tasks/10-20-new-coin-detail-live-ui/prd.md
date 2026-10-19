# Simplify new coin detail while preserving live updates

## Goal

Remove the requested mobile detail explanations and manual-refresh chrome while preserving automatic project, holding, and chart updates.

## Requirements

- Remove the complete `new-coin-detail-stage-note` block, including stage guidance, manual refresh, and its allocation-record shortcut.
- Remove the direct header of `TokenTradingPanel` (`token-surface token-trading`), including the title and refresh button; retain individual holding headers.
- Remove the platform-generated/chart-reference paragraph above the chart.
- Preserve the existing project refresh (10 seconds while visible, plus focus/visibility return), holdings refresh (10 seconds outside pending transactions), WebSocket chart feed, and post-trade reconciliation.
- Preserve phase indicators, trading/confirmation flows, error/retry behavior, and the existing records navigation elsewhere.
- Limit implementation to mobile. Do not change backend, PC, prices, fees, or settlement rules.

## Inspection and Decisions

The existing polling and stream lifecycles already meet the no-manual-refresh requirement. This is a removal-only UI change, not a new refresh implementation. Delete unused stageGuidance and its dedicated trading-header selector; retain shared locale resources and chart styles.

## Acceptance Criteria

- [x] Requested elements are absent in Chinese/English and light/dark themes.
- [x] Project phase changes and holdings update without a refresh button; chart renders and live updates remain wired.
- [x] Mobile release gate passes without budget changes; no horizontal overflow at 320/390/448px.
- [x] PROGRESS and relevant presentation spec updated; local commit with no push/deployment.

## Plan and Validation

1. Remove the requested markup and unused local presentation code.
2. Run the existing phase-refresh/chart-stream tests through the full mobile release gate.
3. Use an isolated local browser fixture to verify hidden copy, phase/holdings refresh, chart rendering, and localized/theme layouts. No production account or money operations.
4. Record results and complete task bookkeeping. Preserve existing `.vite/` cache and unrelated services.

## Delivery and Verification

- `npm --prefix mobile run release:gate` exited 0: 871/871 tests; production/test type checks; PWA/Tauri builds, artifacts, bundle and governance checks. Log: `/tmp/new-coin-detail-live-ui-gate.log`.
- Existing lifecycle and chart tests cover timed/focus/visibility refresh, stale results, pending subscription receipts, live candles overriding old history, and stream teardown. No refresh or settlement code changed.
- Ego Browser space 29 on a task-only 5187 server used an in-memory HTTP adapter and WebSocket fixture; all non-GET adapter requests were rejected. No persistent auth writes or production financial operations.
- Without clicking refresh, distribution changed to purchase and holding value/profit changed from 25/5 to 30/10 within the next 10-second tick (observed 9202ms). Listing then replaced the purchase form with the spot action and enabled the holding on its next tick (observed 4719ms).
- Actual chart socket subscribed to KHBUSDT/1m; a simulated WebSocket candle advanced the rendered chart close to 0.4. Chart stayed mounted through listing. Fixture write count: 0.
- Twelve Chinese/English × light/dark × 320/390/448px checks: removed regions absent, holding headers retained, no horizontal overflow or runtime compile overlay. Screenshots reviewed. Results: `/tmp/new-coin-detail-clean-layouts.json`; screenshots `/tmp/new-coin-detail-clean-*.png`.
- Browser task and task-only server closed; existing services and `.vite/` retained. No push or deployment.
