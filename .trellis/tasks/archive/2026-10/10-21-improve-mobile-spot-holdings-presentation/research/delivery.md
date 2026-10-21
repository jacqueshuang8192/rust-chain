# Delivery

- New SpotHoldingCard: asset identity/icon, exact total holdings, available and combined frozen/locked breakdown, asset-aware grouping and tiny-nonzero formatting. Original filter/state/navigation retained.
- Split the card into an existing-view child after the initial inline version exceeded the source-size budget; thresholds unchanged. Updated the exact Pencil normalization for the card replacement, preserving all remaining template checks.
- npm --prefix mobile run release:gate passed894/894 plus types, PWA/Tauri Web build/artifacts/budgets/governance. Log /tmp/spot-holdings-release.log. Existing numeric-safety tests cover walletTotal/walletFrozen arithmetic.
- Ego actual app with isolated numeric fixture: light/dark at320/390/448px, no page/card overflow; raw total9007199254740993.000000000000000003 and frozen+locked0.000000000000000002 correct; zero writes. Both screenshots inspected. Fixture reload initially navigated out of fixture and showed login; corrected by reopening exact fixture URL. No real transactions.
- Own preview and Ego space3 closed. No production API/native-device validation; local commit only, no push/deployment.
