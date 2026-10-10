# Seconds Chart Workspace

## Scope

`SecondsChartWorkspace` is an in-flow expansion within the existing Seconds
instance. The order form, route pair selection, confirmation and result queue
remain mounted and authoritative. It is not another modal/scroll-lock owner.

## Interfaces

- `SecondsChartWorkspace({ symbol })` owns expansion and supported interval.
- `createSecondsChartSession({ getUrl, load, onChange, startStream? })` exposes
  `open(symbol, interval)` and `stop()`.
- `MobileMarketChart` accepts opt-in `inspection`.
- Renderer emits `inspect(time | null)` and exposes `zoom(factor)`, `latest()`.

## Contracts

- Use only `MARKET_KLINE_INTERVALS` and the existing platform REST/public WS.
  Do not embed an external exchange widget or invent missing candles.
- Replace/close/unmount invalidates old requests and stops the previous stream.
  Real WS candles win over later REST history; REST failure keeps received data
  with a localized retry. History loading only ends when that request settles.
- Candle wicks and native crosshair inspection are opt-in. Other chart consumers
  retain their previous default. MA5/10/20 and volume controls affect real series.
- Crosshair OHLC/MA/volume uses the selected timestamp, otherwise the latest
  candle. Display timestamps explicitly in UTC to match the native time axis.
- Chart values are visualization numbers, never order or settlement authority.
- Zoom cancels pending viewport restoration; return-to-latest restores price
  autoscaling and live-tail position without refitting all history.
- The parent chart grid row is `auto`, not the old fixed 112px sparkline track.
  Hide the sparkline while expanded but preserve its canvas lifecycle.

## Error Matrix

| Event | Required behavior |
|---|---|
| Interval/symbol replaced | Clear old dataset, ignore old results/errors |
| Collapse/unmount | Stop stream; late requests cannot repopulate |
| History failure | Keep live rows; show retry |
| No candle | Unavailable state and disabled zoom/latest |
| Theme/live/history update | Preserve existing viewport anchoring |
| Open/close chart | Preserve order inputs and avoid body scroll locking |

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

## Wrong vs Correct

Wrong: reuse a single unguarded REST completion after closing the chart.
Correct: resolve through the owning market-detail session request generation.
