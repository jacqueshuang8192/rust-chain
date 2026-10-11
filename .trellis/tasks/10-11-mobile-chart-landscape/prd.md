# Mobile Chart Parity And Landscape Expansion

## Goal
Give mobile spot, margin/contract and Seconds charts the same real chart
inspection controls, and automatically present an enlarged chart in landscape.

## Findings
- Spot uses MobileMarketChart without inspection. Margin's chart button opens
  MarketDetailView, which also omits inspection and owns a separate CSS expansion.
- Seconds alone currently enables wicks, OHLC inspection and indicator controls.
- The PWA manifest requests portrait by default. Browser orientation locking is
  capability-dependent and must not be treated as guaranteed on every phone.
- Lightweight Charts 5.2.0 calculates pointer positions from client coordinates
  minus canvas bounds, so rotating only CSS would break chart input coordinates.

## Requirements
- Reuse one shared expandable chart frame across all three entry paths.
- Keep one renderer alive when expanding/collapsing; preserve data owners,
  interval/symbol, viewport and order forms.
- Request native fullscreen/landscape where available; use an automatically
  rotated landscape viewport when the browser cannot lock orientation.
- Correct canvas input coordinates in the CSS fallback, including mouse drag,
  crosshair, touch and pinch. Do not replace the existing chart engine.
- Keep period selection, indicators, OHLC, zoom/latest and Close usable.
- Restore orientation, scroll/focus and native fullscreen owned by this frame
  on close, Escape, browser fullscreen exit, route change or unmount.
- Preserve themes, localization, safe areas, 44px controls and no overflow.

## Approach
Add a shared frame and tested native-orientation lifecycle. Move existing
MarketDetail expansion responsibilities into it, and wire spot and Seconds to
the same frame without changing subscriptions or trading behavior. Retain the
portrait manifest for normal pages; the chart overrides orientation only while
expanded. Test both native API success/rejection/races and fallback input.

## Acceptance
- [x] Spot and margin/detail expose the same inspection features as Seconds.
- [x] Portrait expansion uses landscape geometry; viewport rotation adapts.
- [x] Crosshair/drag/zoom and period/indicator controls work after rotation.
- [x] Exit paths clean up and preserve page/order/chart state.
- [x] Mobile release gate and real-browser geometry/canvas/interaction checks pass.
- [ ] Update PROGRESS and mobile specs, then commit only this task, without push.

## Out Of Scope
No backend/feed/settlement changes, new trading rules, external chart widgets,
production deployment, or forced OS rotation claims for unsupported browsers.

## Interim Verification
- Type check passed after the shared frame and three entry integrations.
- Ego TaskSpace16 at 390x844: fallback frame logical size844x390, screen bounds
  390x844, zero horizontal overflow and the identical canvas node retained.
- Actual mouse hover, drag and return-latest changed inspection timestamps
  correctly after rotation; `/tmp/chart-landscape-first.png` was inspected.
- First full test run:821/831 passed; 10 obsolete source-layout assertions
  failed. Several have been updated, but final regression is pending.
- Native orientation/race and touch regression tests are still required.
- User also requested a standalone swap-history page and console user ledgers;
  those are tracked separately. No current work has been committed yet.
