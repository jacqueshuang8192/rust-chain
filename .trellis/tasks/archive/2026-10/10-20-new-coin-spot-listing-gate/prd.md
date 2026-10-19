# New coin spot listing boundary

## Confirmed requirement
Purchase-stage v2 tokens have live market charts and token subscription trading,
but must not appear in spot lists/favorites or accept ordinary spot orders until
actual listing. User explicitly confirmed this distinction on 2026-10-20.

## Plan
1. Keep purchase-stage pairs active for generation and symbol-specific public
   ticker/K-line reads. Filter spot discovery/favorites using v2 lifecycle listed
   plus actual_listed_at; preserve ordinary pairs and legacy issuance behavior.
2. Apply the same boundary to spot rule loading and transaction-level order
   insertion (including compatibility repository writes), preserving successful
   idempotent replay and existing settlement/locking rules.
3. Keep REST authoritative for client spot membership: valid empty catalogs clear
   stale mobile rows, malformed/all-failed quotes remain failures; PC live frames
   cannot add unknown symbols; mobile opportunities require listed stage.
4. Extend real MySQL/Redis lifecycle tests for catalog/favorites, funded buy/sell
   rejection across order types and IDs, unchanged wallets/orders, retained
   purchase market data, and successful spot handoff without price/engine reset.
   Add focused client behavior regressions and run relevant release checks.
5. Update specs and PROGRESS, record validation, and save local work per workflow.
   No remote push or deployment is requested this turn.

## Acceptance
- Preheat/subscription/distribution/purchase cannot enter the spot catalog.
- A purchase ticker is still readable and quotes/holdings keep working.
- GET markets/favorites and favorite creation exclude unlisted v2 pairs.
- Numeric IDs/direct API orders cannot bypass listing; rejection has no financial
  side effects. Existing ordinary/legacy markets continue to work.
- Actual listing exposes the original pair and supports spot orders while keeping
  the generator revision, latest price, and chart history.
- No schema migration or production data edits.
