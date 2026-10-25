# Mobile deposit flow and record navigation

## Goal
Improve the production PWA/H5 deposit workflow and return from transaction records directly to its source page.

## Requirements
- User follow-up: both picker sheets and selected form values show real asset/network logos. Prefer configured asset logos, with bundled common-asset and five-network icons; unknown images retain the existing AssetMark fallback.
- Record tabs replace the current route; Back returns to the entry page, with existing direct-open fallbacks.
- Consolidate asset/network/amount entry into one form with separate bottom sheets. Preserve legacy route names and parameters using the same shared detail view.
- Show only configured assets/networks. Changing asset selects a supported network and clears the previous amount; dismissing a sheet preserves the draft.
- Separate amount entry, payable order, and terminal order states. Keep exact decimal input, idempotency, cancellation, polling and expired-payment protections.
- Retain current application branding, light/dark themes, Chinese/English, safe areas, 44px touch targets and accessible modal behavior.

## Design decision
Use the requested design-taste-frontend component guidance with the existing app palette/type. Its marketing-page direction draw, picture floor and hero/motion quotas do not apply to this financial app screen. The 30-minute exact-amount reservation is the main hierarchy: selection, quantity, then payment instructions. No invented data or decorative imagery.

## Acceptance
- Tab switching across orders and ledger does not add history entries; direct-open Back stays inside the app.
- Both selection sheets open, select, dismiss, trap focus, restore focus and scroll correctly.
- Payable orders cannot change selection; terminal orders require an explicit new-deposit action.
- Existing deposit behavior tests plus selection/navigation regressions pass.
- Mobile release gate and real browser checks at 320/390/448 widths, both themes, and English.

## Scope
Production mobile only. No backend, PC, admin, payment matching, or new dependencies. No remote push requested.

## Plan
1. Fix record navigation and consolidate deposit entry with selection sheets.
2. Refine deposit state/layout/actions and localization.
3. Verify behavior, browser layouts, release gate; update progress/spec and commit locally.
