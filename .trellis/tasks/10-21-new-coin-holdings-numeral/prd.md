# New coin presentation, quote balances and home valuation

## Goal
Rename the new coin detail holdings section to 我的持仓 and improve its numeric presentation using Numeral.

## Scope and decisions
- Apply the same holdings label and presentation to Mobile and PC.
- Add grouping and remove redundant fractional zeros from quantity, cost, valuation and profit; group the existing exact two-decimal return percentage.
- Preserve every effective decimal digit. Only safe integer portions enter Numeral; larger integer portions use string grouping. Never pass monetary fractions through Number.
- Keep missing values as an em dash and preserve existing transaction input, settlement, refresh and eligibility behavior.
- Do not change charts or unrelated numeric displays. Cloudflare investigation remains separate and unresolved.

## Acceptance
- Both holdings labels read 我的持仓.
- Large values, tiny amounts, negative profit, zero and missing values render correctly.
- Existing transaction tests and type checks pass; Mobile release gate passes.

## Plan
1. Inspect existing formatters and contracts (done).
2. Add a precision-safe Numeral presentation helper and connect holdings only.
3. Validate behavior, build and record progress.


## Follow-up scope (2026-10-21)
- Improve sign-in, sell/unlock and related token action styles consistently on detail and records, including disabled, focus and narrow English layouts.
- Show the exact quote asset available wallet balance for authenticated subscription/live purchase, matched by authoritative asset ID (PC also validates symbol); refresh periodically and after successful mutations. Keep missing/error states distinguishable from zero and ignore stale sessions/responses.
- Format legacy new-coin record amounts and confirmation numbers with the existing precision-safe Numeral helper on Mobile and PC.
- Repair Mobile Home asset valuation: use wallet issuance metadata independent of spot catalog, count booked available/frozen/locked buckets exactly once, preserve missing/expired-price semantics and auto-refresh.
- Preserve transaction authorization, uncertain-result recovery, quote authority and eligibility checks. No backend or production configuration change is planned.

## Follow-up acceptance
- Real component tests cover quote-balance identity, zero/missing/error, session replacement and successful transaction refresh.
- Home regression reproduces pre-listing holdings with no spot ticker and verifies exact complete valuation, expiry, recovery and account isolation.
- Mobile release gate and PC types/build/tests pass without increasing budget thresholds.
- Browser validates detail and records, guest/member, enabled/disabled actions, ordinary and large/tiny numbers, light/dark and narrow layouts.
