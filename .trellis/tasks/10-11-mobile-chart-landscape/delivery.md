# Delivery

- Spot, market detail (including the margin chart entry) and Seconds now share
  one expandable frame with supported intervals and the existing real renderer.
- All three enable OHLC/crosshair inspection, candle wicks, MA/volume controls
  and zoom/latest. Expansion keeps the renderer and page/order state mounted.
- Native fullscreen/orientation is best effort. Portrait fallback rotates the
  logical viewport and adapts canvas mouse/touch coordinates, not control input.
  Close/Escape/native exit/unmount and late grants clean up owned state only.
- Ten new behavior cases cover ownership, rejected/late native operations,
  reopen, touch identity, coordinate inversion and drag cleanup.
- Combined mobile release gate: 844/844, both type checks, PWA/Tauri web builds,
  artifact, bundle and governance checks passed. Final narrow drag-reset change
  was included in the last complete gate.
- Ego TaskSpace16: anonymous live spot/detail data, 320/390/448px portrait,
  844x390 landscape and 1280x900 desktop. No outer overflow, same canvas identity
  throughout, 18-22 sampled colors; spot quantity 0.0123 survived expansion.
  Mouse crosshair/pan/latest and Escape restoration verified. Screenshots at
  /tmp/chart-landscape-first.png and /tmp/spot-chart-desktop.png were inspected.
- Final framing uses the engine's native visible-range resize lock for inspection
  charts. The last full gate again passed844/844. Final portrait image
  /tmp/landscape-chart-final.png was inspected, with no added blank time range.
  Margin entry/5m selection and Seconds light-theme landscape/collapse were
  also exercised; collapse removed the expanded engine and cleared scroll lock.
- Browser-emulated touch events were dispatched; native phone pinch/long press,
  iPhone/Android orientation locks and installed-PWA behavior were not validated
  on physical devices. No claim that every browser can rotate the OS display.
- No backend price/subscription/trading policy changes in this task. No push
  or deployment. Local preview remains on 5186.
- TaskSpace16 was closed once after all three related features were checked.
