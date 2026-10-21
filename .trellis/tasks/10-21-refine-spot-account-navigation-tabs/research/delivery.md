# Delivery

CSS-only spot-account-tabs polish:15px labels/14px narrow,24px spacing/20px narrow,3px active underline matching label target width, dedicated44px rounded history action, hover/pressed feedback. Existing48px row, navigation, holdings semantics and focus styles retained.

Validation: npm --prefix mobile run release:gate passed896/896, types, PWA/Tauri Web builds/artifacts/budgets/governance. Log /tmp/spot-tabs-release.log. Ego space5 actual isolated app: light/dark320/390/448, no overflow, all controls44px tall, history44px wide, underline44px matching selected item. Both screenshots inspected. No new tests for CSS-only reversible change; existing layout contracts pass. No new spec pattern beyond existing44px/theme contract.

Own preview and Ego closed. No production/native-device checks; local commit only, no push/deployment.
