# Agent token issuance lifecycle and unlock-sale

## Goal
Implement the nine user requirements (2026-10-19) across agent application/review, project lifecycle, user purchases and holdings, allocation/refunds, synthetic market and exit settlement. Existing historical orders/locks must retain their contracts.

## User Requirements
1. All active agents can apply to issue a token. Configure a fixed subscription price and initial purchase-market seed price; any listing price is reference-only and never resets the live market; subscription, distribution, purchase and listing time nodes each independently choose manual or automatic operation.
2. Enter/display dates in the configuring user's timezone; other users view the same instant in their timezone. Persist an explicit IANA zone and UTC instants for unambiguous scheduling.
3. Lock options: no lock; actual listing event; actual purchase-stage event; relative duration after purchase (weeks/calendar months); fixed instant. Preserve original rule snapshots on holdings.
4. Admin approval atomically creates corresponding asset/project and zero-balance user wallets, without minting user balances.
5. Allocation supports configurable 0..100% ratios, manual single-order and selected-batch execution; automatic allocation was explicitly removed by the user on2026-10-19. Different selected batches may use different ratios; allocate purchased quantity times ratio and return unspent quote. Example: price 0.2 USDT, paid 1000, requested5000, 50% delivers2500 and refunds500. Support80% and100%.
6. All users may buy all eligible public agent-issued projects. Record visibility and agent allocation authority follow the buyer's owning agent/subtree, not the issuing agent. A may allocate only B's orders even if B buys C's token; C may allocate only D's orders even if D buys A's token.
7. Purchase stage has a chart driven by the existing platform synthetic market engine.
8. Users see their purchased holdings, chart value and return, with sell eligibility determined authoritatively by original lock rules.
9. Agent configures optional unlock/sale fee: positive-profit ratio, gross-value ratio, or fixed quote value. A user's exit performs unlock and sell together.

## Confirmed / Derived Scope
- Separate purchase phase before listing is required; current post-listing purchase interpretation is insufficient.
- UI spans agent and Admin Web, user Mobile and PC equivalents where exposed.
- Existing pending manual allocations, old issuance applications and original lock snapshots require explicit compatibility rather than reinterpretation.
- No production migration, deployment or financial writes during development/verification.

## Execution Decisions
- Confirmed: immediate platform buyback at a valid synthetic engine quote; fee deducted from gross proceeds; unlock, sale and net credit commit atomically.
- Confirmed: purchases execute at the authoritative realtime platform price. On listing the same pair and historical candles continue into spot with no price reset. New-coin purchase entry ends at listing; tracked issuance holdings retain lock/exit-fee rules.
- Defaults to make explicit: months are calendar months in the configured timezone, each batch applies a final allocation once, timers enable stages but never allocate or sell automatically.

## Acceptance Criteria
- [x] Reviewed application faithfully persists and publishes all prices, node modes/times, timezone, lock and fee rules; no automatic allocation setting.
- [x] Manual and automatic transitions are durable, ordered, retry-safe and auditable; merely editing planned dates cannot claim an actual event.
- [x] Cross-project buyer scope is enforced for single/batch/manual allocation, with exact refunds, idempotency, concurrent retry and rollback tests.
- [x] Immutable holding rules allow/deny selling correctly across no-lock, stage, duration and fixed-time cases and historical records.
- [x] Exit and fees conserve balances and supply in one transaction, with authoritative bounded quote and replay evidence.
- [x] Charts/holdings/returns and forms handle precision, timezones, DST errors, empty/error states and mobile layouts.
- [x] Real MySQL tests, cross-layer API tests, lint/typechecks/build and browser checks completed; the pre-existing Web bundle-budget failure is explicitly documented in delivery.md.

## Out of Scope
Deploying production changes, editing applied migrations, changing unrelated products, or including unrelated leftover Tabs work without explicit accounting.

## Current Status
Implementation and functional verification complete. See delivery.md for evidence, the pre-existing Web bundle-budget failure, and rollout requirements. On 2026-10-19 the user explicitly authorized committing the documented manifest and pushing to origin/main. Commit/push execution is in progress; no production deployment or migration is included.

## Confirmed Decisions
- 2026-10-19: direct platform buyback, valid engine quote, deduct fee from proceeds, atomic unlock/sell/credit.
- 2026-10-19: remove auto allocation. Agent explicitly selects batches from its own team and enters a separate ratio for each batch. Auto distribution-stage entry is distinct from allocating coins.

- 2026-10-19: purchase is realtime market trading; listing continues the same market in spot without reseeding. Purchase price is initial seed, listing price is reference only.
