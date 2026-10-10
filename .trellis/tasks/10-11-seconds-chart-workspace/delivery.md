# Delivery

## Implemented

- In-flow Seconds chart workspace, hiding the compact sparkline only while open.
- Real candles with wicks, MA5/10/20, volume, native crosshair/price line,
  timestamp/OHLC/MA/volume inspection, zoom and return-to-latest controls.
- Shared supported periods, localized controls and history failure/retry.
- Independent expanded-chart stream owner, live-over-REST reconciliation,
  stale callback rejection and teardown. Order logic remains unchanged.
- Auto-sized parent track prevents order controls overlaying the chart.

## Verification

- Final `npm --prefix mobile run release:gate`: 831/831 passed, no skips;
  source/test type checks, PWA/Tauri builds, artifact and all budgets passed.
- Renderer/session/inspector behavior coverage added in
  `mobile/tests/seconds-chart-workspace.test.ts`.
- Ego TaskSpace 14, anonymous local platform data, no orders submitted.
- Light/dark at 320x720, 390x844, 448x900 and 1280x900:
  horizontal overflow 0, chart/order overlap false. Dark controls hit-test passed.
- Native pan changed inspected time from 1791660060 to 1791658740.
  Crosshair inspection, zoom/latest, 5m switch, indicator checkboxes exercised.
- Canvas pixel check: main plot 548x752 with 56 sampled RGBA colors,
  crosshair overlay nonblank. Screenshots inspected in both themes.
- Collapse/reopen preserved typed amount 600, restored 112px sparkline,
  removed expanded renderer and left body overflow unlocked.

## Limitations

- No installed-PWA, physical phone pinch/long-press, native package or
  production deployment verification. Tauri result is the web build only.
- Not a complete TradingView terminal or external exchange-price parity claim.
- Existing unrelated wallet/source-label changes remain outside this task.
- Awaiting scoped commit confirmation; no push or deployment.
