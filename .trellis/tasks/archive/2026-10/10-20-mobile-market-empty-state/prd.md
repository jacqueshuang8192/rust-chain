# Mobile home market empty state

## Goal

Polish the `root-market-empty-state` presentation within the existing mobile home market section.

## Requirements

- Keep changes limited to the empty-state markup and styles in HomeView and prototype-parity.
- Replace the oversized 350px minimum height with the existing 162px three-row list footprint.
- Improve icon treatment, text hierarchy, spacing, and wrapping using existing semantic theme tokens.
- Preserve localized copy, filtering, navigation, market data, loading, and error behavior.
- Check Chinese and English at 320, 390, and 448px in both themes, with no overflow or bottom-navigation overlap.

## Decision

Use a compact centered empty state with a muted icon surface and separate title/description spacing. This app screen retains its existing design system; the marketing-page composition generator in design-taste-frontend is outside this task's scope. No dependencies, animation, or new actions are needed.

## Acceptance Criteria

- [x] Empty state fits the 162px market-list footprint and remains readable.
- [x] Theme and language browser checks pass; populated and unavailable states remain intact.
- [x] Mobile release gate and git diff checks pass without relaxing budgets.
- [x] Update PROGRESS and record validation evidence.

## Implementation Plan

1. Inspect the current empty state in a local browser.
2. Adjust existing component and CSS, then verify runtime geometry and screenshots.
3. Run the required release gate and document delivery.

## Scope and Validation

No business logic, backend, PC, other page styles, or deployment changes. Preserve the pre-existing untracked `.vite/` directory. No implementation-mirroring unit tests for this reversible visual change; use browser layout checks and existing automated gates.

## Validation

- `npm --prefix mobile run release:gate`: exit 0, 871/871 tests, production/test types, PWA/Tauri builds and artifacts, bundle/source/test-quality gates passed. Log: `/tmp/market-empty-release-gate.log`.
- Ego Browser space 28: 320x720, 390x844, 448x900 × light/dark × Chinese/English, 12 layout checks passed. Empty-state height 162px, contained in list, zero horizontal overflow, centered icon, no Dock overlap at scroll end. Muted copy contrast: light 6.52:1, dark 7.87:1.
- Existing Favorites -> Popular -> Favorites interaction renders the original three market rows then restores the empty state. Loading/error states retain three skeleton rows, original busy state, error alert, and no empty state; checked with temporary in-memory store state restored afterwards.
- Screenshots reviewed at 320px English/light and 390px Chinese/dark. Results: `/tmp/market-empty-browser-results.json`; screenshots `/tmp/market-empty-*.png`.
- Spec records the existing list-height boundary to prevent oversized empty states from extending behind the Dock.
- Local commit only; no push or deployment. Existing `.vite/` cache and preview services preserved.
