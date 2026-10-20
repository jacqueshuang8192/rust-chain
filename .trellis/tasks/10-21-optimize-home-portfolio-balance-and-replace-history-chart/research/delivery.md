# Delivery

## Implemented
- Home total uses existing precision-safe Numeral formatter after the previous four-decimal rounding; tiny nonzero amounts remain exact. Font 26px, long amount 22px.
- Removed Home historical chart, its requests/state and chart-specific parity CSS. Retained today return, wallet valuation/refresh/privacy and shared history API/domain tests.
- Final generated wallet illustration replaces the temporary brand asset; PNG RGBA 1254×1254, alpha range 0–255.

## Validation
- Mobile release:gate passed 892/892 tests, types, PWA/Tauri Web builds, artifacts, bundle/governance. Log /tmp/home-portfolio-release.log.
- PWA JS1491.3 KiB, Tauri JS1487.8 KiB, CSS654.1 KiB; unchanged budgets.
- Afterwards clarified the test harness to use zero margin balance rather than negative fixture spot amounts; focused Home 3/3 and test type check passed.
- Ego space2/page p1: 24 combinations (light/dark, zh-CN/en, 320/390/448px, ordinary/large balances), no overflow/history calls/writes, expected 26px/22px fonts. Privacy toggle masked assets and today return. Matrix saved beside this file. Image visual acceptance remains pending.

## Final validation
- Final `npm --prefix mobile run release:gate`: exit 0, 892/892 tests, types, PWA/Tauri Web builds, artifacts, budgets and governance passed. Log /tmp/home-portfolio-final-release.log. PWA JS1491.3 KiB, Tauri JS1487.8 KiB, CSS654.1 KiB; budgets unchanged.
- Built-in image generation succeeded; no API key or paid CLI fallback was used. Final project asset is mobile/src/assets/home/portfolio-wallet.png.
- Previous Ego space2 no longer existed (inventory empty). Final asset checked in a new session space1: dark390 ordinary amount 2,000 at26px; light320 unsafe-integer amount 9,007,199,254,741,993 at22px; image loaded, no horizontal overflow, no history requests. Both screenshots visually inspected. Prior 24-case layout matrix remains valid.
- Original Home rounding behavior retained: four decimals for ordinary totals; tiny nonzero totals preserved. No change to monetary arithmetic or backend requests beyond removing unused history reads.
- Final space closed and own5187 preview stopped. No native device or production API verification. Work saved locally; no push or deployment for this task.
