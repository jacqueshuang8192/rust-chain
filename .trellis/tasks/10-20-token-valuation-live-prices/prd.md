# Repair live issuance valuation and holding returns

## Goal
Fix purchase-stage locked-token valuation in the token detail, asset rows and
total portfolio valuation, using the real platform market feed.

## Confirmed requirements
- Detail holdings must expose current value, unrealized gain and return rate
  whenever a valid current market price exists, including locked holdings.
- Asset rows and totals must include booked locked token balances during purchase.
- Preserve existing phase, unlock, buy/sell, and listing-only spot permissions.
- Missing/stale invalid prices remain unknown rather than zero or fabricated.
- Verify the producer-to-consumer contract, not just handwritten ticker fixtures.

## Plan
1. Trace real synthetic ticker publication, Redis parsing and issuance valuation
   checks; compare holdings and wallet paths with the previous tests.
2. Reproduce the failing production-shaped data in isolated integration tests.
3. Apply the smallest shared fix and cover detail, wallet and total calculations.
4. Run focused real-store regressions and relevant frontend checks; document the
   root cause, missed test case and deployment needs; save local commits.

## Scope
No pricing strategy change, lock-rule change, redesign, migration, deployment or
automatic remote push. No unanswered product decisions; the user has authorized
the three valuation corrections.

## Acceptance
- A live producer price gives non-null holding value/PnL/rate and wallet price.
- Locked balances participate in asset totals without duplicating pending tokens.
- Freshness, current-engine identity and prelisting spot restrictions stay tested.
- Existing subscription/distribution fixed valuation remains supported.
- PROGRESS and domain specifications capture the regression and verification.
