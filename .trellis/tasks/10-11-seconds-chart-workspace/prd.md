# Seconds Chart Workspace

## Scope
- Preserve the Seconds order form, selected pair route and settlement behavior.
- Add an expandable in-flow, full-width chart below the price sparkline. Avoid
  another modal owner competing with order confirmation and settlement dialogs.
- Reuse MobileMarketChart and lightweight-charts with an opt-in inspection mode:
  candle wicks, MA5/10/20, volume, native crosshair, OHLC/time readout,
  zoom controls and return to latest. Keep other consumers' defaults unchanged.
- Use the platform REST + public WS session, not Bitget data or an iframe.
- Offer only supported 1m/5m/15m/1h/1d intervals. Close/unmount stops subscriptions;
  symbol/interval replacement rejects late results and preserves live authority.
- Localize all controls. Provide loading, empty, error/retry states.

## Verification
- Behavioral renderer tests for opt-in effects, readout and zoom/latest controls.
- Session tests for late REST/live precedence and close/replace cleanup.
- Full mobile release:gate without increasing budgets.
- Browser checks for narrow screens, themes, real canvas rendering and controls.
- Record limitations: no claim of full TradingView parity or native-device testing.

## Plan
1. Implement isolated workspace and opt-in chart inspection.
2. Add focused regression tests and run release gate.
3. Verify browser layout/interactions and record delivery.
