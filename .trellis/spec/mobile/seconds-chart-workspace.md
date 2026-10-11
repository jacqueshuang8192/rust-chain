# Trading Chart Workspaces

## Scope

`SecondsChartWorkspace` is an in-flow expansion within the existing Seconds
instance. The order form, route pair selection, confirmation and result queue
remain mounted and authoritative. Its inner `MarketChartFrame`, also used by
spot and market detail (the margin chart destination), owns fullscreen only.

## Interfaces

- `SecondsChartWorkspace({ symbol })` owns expansion and supported interval.
- `createSecondsChartSession({ getUrl, load, onChange, startStream? })` exposes
  `open(symbol, interval)` and `stop()`.
- `MobileMarketChart` accepts opt-in `inspection`.
- Renderer emits `inspect(time | null)` and exposes `zoom(factor)`, `latest()`.
- `MarketChartFrame({ symbol, interval })` emits `interval` and Teleports its
  existing slot to `body` only while expanded; never key/remount the renderer.
- `createChartOrientation(environment, onExit)` exposes `enter/exit/dispose`.
- `bindRotatedChartInput(host, rotated)` returns listener cleanup.

## Contracts

- Use only `MARKET_KLINE_INTERVALS` and the existing platform REST/public WS.
  Do not embed an external exchange widget or invent missing candles.
- Replace/close/unmount invalidates old requests and stops the previous stream.
  Real WS candles win over later REST history; REST failure keeps received data
  with a localized retry. History loading only ends when that request settles.
- Candle wicks and native crosshair inspection remain opt-in at renderer level;
  Seconds, spot and market detail all enable inspection. MA5/10/20 and volume
  controls affect real series.
- Crosshair OHLC/MA/volume uses the selected timestamp, otherwise the latest
  candle. Display timestamps explicitly in UTC to match the native time axis.
- Chart values are visualization numbers, never order or settlement authority.
- Zoom cancels pending viewport restoration; return-to-latest restores price
  autoscaling and live-tail position without refitting all history.
- Inspection enables native `lockVisibleTimeRangeOnResize`: expansion adjusts
  bar spacing to keep the chosen candle range, rather than creating blank space
  or fitting a different range. Non-inspection renderer defaults stay unchanged.
- The parent chart grid row is `auto`, not the old fixed 112px sparkline track.
  Hide the sparkline while expanded but preserve its canvas lifecycle.
- Fullscreen first requests native fullscreen/landscape when available. Keep
  the normal PWA portrait manifest; release only the chart-owned native state.
  Missing/rejected APIs retain CSS presentation, not a promise of OS rotation.
- While the visual viewport remains portrait, swap its logical dimensions and
  rotate the frame 90 degrees. Invert coordinates for canvas mouse/touch/wheel
  input before the existing engine handlers; do not implement another engine.
  Never remap buttons/links, mutate the original Touch objects, or keep drag
  ownership after returning to a nonrotated view.
- Close, Escape, native fullscreen exit and unmount release owned locks and
  listeners. Late native promises cannot reopen the presentation; reopening
  waits for prior native exit. Do not exit another component's fullscreen.

## Error Matrix

| Event | Required behavior |
|---|---|
| Interval/symbol replaced | Clear old dataset, ignore old results/errors |
| Collapse/unmount | Stop stream; late requests cannot repopulate |
| History failure | Keep live rows; show retry |
| No candle | Unavailable state and disabled zoom/latest |
| Theme/live/history update | Preserve existing viewport anchoring |
| Inline open/collapse | Preserve order inputs and avoid body scroll locking |
| Fullscreen open/close | Preserve canvas identity, inputs and viewport; restore scroll/focus |

## Examples

Good: an old BTC/1m completion cannot replace ETH/5m after selection.
Base: collapsed chart owns no expanded-chart connection.
Bad: fixed-height parent lets the order form cover the native chart controls.

## Verification

Run `npm --prefix mobile run release:gate`. Behavioral tests execute production
session/renderer/inspector logic for live precedence, stale ownership, teardown,
OHLC selection, localization, controls and default compatibility. Browser checks
must cover nonblank canvas, pan/crosshair, periods, input preservation, themes,
320/390/448px and desktop without overflow or chart/order overlap.
`chart-landscape.test.ts` additionally exercises native grant/denial/late
completion, disposal, ownership, coordinate inversion, drag and touch identities.
Real phone orientation and installed-PWA behavior require separate device checks.

## Wrong vs Correct

Wrong: reuse a single unguarded REST completion after closing the chart.
Correct: resolve through the owning market-detail session request generation.
