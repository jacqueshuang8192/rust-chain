# Mobile deposit workflow delivery

## Changes
- Record tabs replace the current history entry; Back returns to the source page.
- One on-chain deposit form replaces separate asset/network pages. Legacy URLs
  remain supported. Two searchable bottom sheets show the current selection.
- Configured asset logos take precedence over local common-asset fallbacks;
  network logos use the five canonical network keys, including `btc` and Base.
  The bundled PNGs have pinned source and MIT license records.
- Exact amount entry, explicit order creation, server-corrected countdown,
  cancellation, refresh and payment matching remain intact. Terminal orders
  require an explicit new-deposit action; old polls cannot resurrect them.
- Selection is remembered per session identity, never payment amounts/status.

## Verification
- `npm run release:gate`: 919 tests passed, production/test type checks,
  PWA/Tauri web builds, artifact checks, bundle/source/test-quality budgets passed.
- Final canonical Bitcoin icon correction: 16 focused icon/deposit tests passed;
  both web build modes and bundle checks rerun separately.
- PWA approximately 1513 KiB JS / 657.2 KiB CSS, below existing limits;
  no budget increases or dependency additions.
- Browser used real Vue components and a local API adapter, not real transfers.
  Checked 320/390/448 widths, both themes, English, logo image loads, search/no
  results, selection, keyboard containment, focus return, Escape/backdrop,
  scrolling lock, cancel/new order/expired payment and zero horizontal overflow.
- The 320px English payment page places Copy address within the initial 720px
  viewport. Form and order details retain scroll access on shorter viewports.
- Design skill component guidance retained existing palette/type and financial
  content. No marketing direction draw, stock imagery, decorative animations or
  fabricated production data. Source lint has zero failures; clock padding and
  existing confirmation-button border are intentional warnings. Browser review
  corrected global overlay specificity, dark-sheet contrast and narrow payment
  height. Marketing-only inspector quotas and award scores are inapplicable.

## Limits
- No native Android/iOS device build, live backend payment or deployment tested.
- Only local commits; no remote push requested in this task.
- Browser validation space closed; temporary Vite server cleaned up at finish.
