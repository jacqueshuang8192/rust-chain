# Delivery

## Changes
- Mobile/PC holdings title is 我的持仓 / My holdings. Numeral core groups safe integer parts; exact string fractions and larger integers preserve all financial digits. Records, confirmations and notices reuse the formatter. Inputs and request amounts are unchanged.
- Detail/record login and unlock/sell actions have 48px controls with theme-aware emphasis, disabled and focus states. Long record numbers wrap.
- Quote wallet available balance is visible for subscriptions/purchases and refreshes after success and periodically. Mobile reuses the parent wallet lifecycle; PC uses a guarded TokenQuoteBalance component. Empty accounts mean zero; failed reads remain unknown.
- Mobile Home now values prelisting holdings from wallet issuance metadata, independently of spot tickers, using the existing Assets valuation helper. Booked available/frozen/locked amounts count once. Refresh preserves current data while loading, rejects obsolete responses and keeps missing/expired prices unknown.

## Validation
- `npm --prefix mobile run release:gate`: passed, 891/891 tests, production/test type checks, PWA/Tauri Web builds, artifact/budget/governance checks. Log: /tmp/holdings-followup-mobile.log.
- PWA JS rounds to 1500.0 KiB but passes the unchanged 1500 KiB limit; CSS 655.8 KiB / 656 KiB. Tauri JS 1496.4 KiB. Very little bundle headroom remains. Numeral core import and deduplicated scoped styling keep budgets unchanged.
- Final `npm --prefix pc run type-check`, `npm --prefix pc run build`, `node --experimental-strip-types --test pc/tests/*.test.ts`: passed, 127/127. Logs: /tmp/holdings-final-pc-{build,tests}.log.
- Actual Home computed/load lifecycle tests cover prelisting valuation without ticker, expiry/recovery, no loading flash, missing-price fallback rejection, logout and obsolete requests. Quote balance tests execute real component lifecycle and Mobile parent functions for available vs frozen, zero/error, refresh and account replacement.
- Ego space 1: isolated full actual Mobile app with all API calls mocked, 56 combinations across light/dark, Chinese/English, 320/448px, guest/purchase/long/subscription/records/records-long/home. No horizontal page overflow, clipped balance/record values, or controls below 44px. Read matrix made zero writes. Results: research/browser-matrix.json. Repro fixture: mobile/tests/fixtures/new-coin-ui-browser.{html,mjs}.
- Simulated buy of 2 at 0.4 made one mock request retaining exact quantity and refreshed available balance 1000 -> 999.2.
- Actual Home timer integration: total 2000 -> 3000 after quote price changed, then unavailable after wallet failure -> 3000 on recovery, with zero writes.
- Previous holdings-only UI verification covered Mobile/PC in 16 combinations and screenshots were inspected. Follow-up screenshot capture failed twice with CDP Page.captureScreenshot timeout; no new screenshot visual approval is claimed. PC follow-up is verified by component behavior/types/build/tests, not a new browser matrix.
- `git diff --check` and task context validation passed. Ego space closed and own 5187 preview stopped.

## Limits
No production API, real funds, deployment or native Android/iOS package builds exercised. Tauri Web artifacts verified. Local work commit and task archival only; no push. Cloudflare investigation and existing .vite cache remain separate.
