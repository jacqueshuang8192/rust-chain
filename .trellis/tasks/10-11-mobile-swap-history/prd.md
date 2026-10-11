# Mobile Swap History

## Goal
Move swap records to an independent mobile page and display both asset logos.

## Requirements
- The existing history header action navigates to a dedicated route with
  deterministic Back-to-Swap behavior on direct open.
- Remove the embedded record list and unnecessary history requests from SwapView.
- Reuse authenticated history pagination and existing asset metadata/AssetMark.
- Show source/target asset logos, symbols, amounts, status and timestamp.
- Preserve decimal precision, loading/error/empty/login states, localization,
  account isolation and responsive layouts. Do not alter quote/confirm semantics.

## Verification
Add route, pagination/session, logo fallback and view behavior regressions;
run the mobile release gate and browser checks at phone and desktop sizes.
Record progress/specs and commit this task without pushing or deploying.

## Related Work
Chart landscape work in `10-11-mobile-chart-landscape` remains in progress.
The user's latest message confirms both requests and additionally asks for
admin/agent wallet and fund records (`10-11-console-user-wallet-ledgers`).
