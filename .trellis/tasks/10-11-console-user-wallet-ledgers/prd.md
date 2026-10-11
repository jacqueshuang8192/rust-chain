# Admin And Agent User Wallet Records

## Goal
Expose user wallet balance changes and fund-flow records in both consoles.

## Initial Findings
- Admin already exposes `/admin/api/v1/wallet/ledger` and a wallet-ledger resource.
- Agent user asset views and descendant-team authorization already exist.
- Inspect wallet snapshots versus accounting journal records before choosing
  the read model; do not invent accounting history from present balances.

## Requirements
- Both consoles can enter records from user management and filter relevant data.
- Show immutable wallet changes and available/frozen/locked snapshots, plus
  financial flows with their authoritative business references where available.
- Agents may inspect only users in their currently authorized team hierarchy;
  deny sibling, ancestor and unrelated users consistently in every query/count.
- Read-only APIs and UI, Chinese labels, exact decimal strings, bounded server
  pagination and meaningful loading/error/empty states.
- Reuse existing components, permission checks and domain contracts; do not
  change balances, settlement, financial journal writes or user ownership.

## Verification
Focused backend/API authorization and readonly tests, OpenAPI if changed,
frontend API/view/route tests, type/lint/build and responsive browser checks.
Document limitations; commit verified work without pushing or deploying.

## Chosen Read Model
- Both views use immutable user wallet ledger entries, not platform-only
  accounting legs. Wallet changes show the affected bucket and persisted
  available/frozen/locked snapshots; fund flow shows signed amount and business
  reference. Do not sum transfer/freeze legs as net income.
- Select spot or margin explicitly; default spot preserves the existing admin
  endpoint contract. Add account selection to the admin wallet-ledger query,
  plus a scoped agent per-user ledger endpoint and shared read-only UI.
- Agent SQL repeats the subtree predicate for rows and count after membership
  validation, following existing per-user portfolio queries.

## Related Work
Finish alongside the user's chart-landscape and mobile-swap-history requests.
