# Remove Client K-line Source Display

## Goal

Remove the chart header row labelled "K线来源 / 外部参考" from Mobile and PC.

## Scope

- Remove the source-summary row, its dedicated styles and unused aggregation
  from `MobileMarketChart.vue` and `MarketChart.vue`.
- Keep the Mobile optional ticker input compatible with existing callers.
- Preserve candle metadata, chart providers, 1m defaults, live updates,
  history pagination and the generated-market risk notice.
- Preserve source labels outside the chart, including order books and trades.
- No backend, database, dependency or trading behavior changes.

## Acceptance Criteria

- [x] Neither chart wrapper renders a source-summary row in Chinese or English.
- [x] Mixed-source and unknown candles still reach the Mobile renderer intact.
- [x] Historical loading/retry and generated-market notices remain functional.
- [x] PC keeps provider selection, chart inputs and default interval.
- [x] Relevant tests and type checks pass; progress and specifications updated.

## Verification Plan

- Mobile release gate and PC type check plus chart/provenance regressions.
- Browser smoke check was attempted but timed out while mounting an isolated
  component, so it is not used as a pass criterion.
- `git diff --check` and Trellis task validation.

## Known Workspace State

Leave `.vite/`, `mobile/pnpm-lock.yaml` and `mobile/pnpm-workspace.yaml`
untracked and unchanged. Do not push or operate a production service.

## Delivery

- Mobile release gate passed with 793 tests, both build modes and all budgets.
- PC type check and 8 chart/provenance regression tests passed.
- Diff and task validation passed. Browser mounting timed out; visual checks
  are not claimed. The isolated browser space was closed after navigation
  cancelled its pending evaluation.
- Implementation complete. Await user confirmation before committing; archive
  only after the work commit. Development previews use ports 5186 and 5187.
