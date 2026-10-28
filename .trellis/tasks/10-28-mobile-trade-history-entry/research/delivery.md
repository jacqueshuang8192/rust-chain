# Trade history entry delivery

## Result

The spot account rail now renders its history action as a transparent, muted
19px icon centered in a 44x44 target. A light circular surface appears only
on hover or press. Existing keyboard focus, reduced-motion behavior, locale
labels and the orders route handler remain intact.

## Files

- `mobile/src/views/TradeView.vue`: scoped history class and state styling.
- `mobile/tests/pencil-trading-product-selected-parity.test.ts`: normalize only
  the newly authorized presentation class, preserving the original template digest.
- `.trellis/spec/mobile/index.md`: persist the spot-only control contract.
- This task's PRD/context/metadata and `docs/superpowers/PROGRESS.md`.

## Verification

- Four existing targeted test files: 37/37 passed.
- `npm --prefix mobile run release:gate`: passed. Production/test type checks,
  923/923 tests, PWA and Tauri web builds, artifact assertions, bundle budgets,
  source-size and critical-test quality checks all passed.
- PWA output: 1514.5 KiB JS / 657.4 KiB CSS; Tauri web: 1511.0 KiB JS /
  657.4 KiB CSS. Existing budgets were not changed.
- First full run found the expected template fingerprint change; the existing
  normalization now permits only the added class. Final full run passed.
- Ego used the actual Vue app and the existing isolated local API fixture.
  All 12 width/theme/locale combinations (320/390/448, light/dark, zh-CN/en)
  had zero horizontal overflow, exact 44x44 targets, and SVG center deltas of
  0px on both axes. Resting backgrounds were transparent with no border/shadow.
- Six theme/width screenshots, hover and pressed screenshots were inspected.
  Pointer click and keyboard Enter both reached `/#/orders?tab=history`;
  Back restored `/#/trade`. Keyboard focus retained its inset ring. Under
  reduced motion, press kept 44x44 geometry, no transform, and 0s transition.
- The split native mouse-down/up wrapper reset the emulated viewport, so the
  held-press check used CDP input on the same page. Normal selector click and
  keyboard activation both passed independently.
- `git diff --check` and task context validation passed.

## Scope and runtime limits

No live backend or financial order was exercised: this is a presentation-only
change, validated with a local API fixture. No native device build was needed.
No push or deployment. Existing agent-management work remains separate.
