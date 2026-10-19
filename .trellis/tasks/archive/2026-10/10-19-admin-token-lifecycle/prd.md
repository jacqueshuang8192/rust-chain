# Complete admin new coin lifecycle management

## Goal
Bring the administrator new-coin workflows into line with the agent issuance lifecycle delivered in ec781e2, retaining the confirmed business rules and historical v1 behavior.

## Known requirements
- Admin must review the complete issuance contract and manage approved projects coherently.
- Fixed-price subscription, manual batch allocation/refunds, realtime purchase trading, continuous listing into spot, immutable lock/fee rules, and atomic platform buyback remain authoritative.
- No automatic coin allocation or automatic sale. Agent order authority remains scoped to the buyer's agent subtree.
- Existing admin project center already contains partial v2 summaries and lifecycle handling; inspect actual gaps before changing behavior.

## Planned scope
- Audit admin creation/review, project center, market configuration, subscription/allocation records, holdings and buyback visibility.
- Fill missing backend/read contracts and corresponding admin UI with existing permission/audit patterns.
- Preserve legacy workflows and reject edits that would invalidate approved v2 contracts or historical holdings.

## Confirmed decisions
- 2026-10-19: user explicitly requested upgrading administrator direct creation to the same v2 price, node, lock and fee model. Agent applications still require review.
- Administrator direct creation uses an existing active base asset and quote asset from the current admin asset workflow; it atomically creates a preheat project, disabled fixed pair/generator and zero-balance wallets, without minting balances.
- Existing admin single-order distribution remains available as an audited fallback. Agent manual selected-batch permissions remain unchanged.

## Requirements
- Add recoverable, idempotent admin v2 creation; preserve the existing legacy API for historical clients.
- Show issuer identity, actual purchase event, next automatic node and failed/retry status in project management.
- Add authorized, paginated v2 holdings, executed buyback records and operation history; distinguish buyer ownership from issuer.
- Rename purchase UI to cover realtime recognition and historical purchases; point v2 project links to v2 holdings/buybacks.
- Pin v2 market configuration to its published pair and seed; do not publish before the actual purchase stage or overwrite approved policy.
- Show readable lock/fee rules and precise settlement amounts; retain original evidence in details.

## Open questions
- None. The creation decision is confirmed; remaining choices follow existing contracts.

## Acceptance criteria
- [x] Admin workflows reflect the v2 lifecycle with accurate language and operations.
- [x] V2 holdings/fees/buybacks and allocation/refunds can be inspected through authorized admin views.
- [x] Relevant permission, precision, pagination, concurrency and historical compatibility cases are verified.
- [x] Targeted backend/frontend tests, type/lint/build checks and browser verification pass or limitations are recorded.
- [x] Progress and executable contracts are updated.

## Out of scope
- Production deployment, production migrations or funds operations.
- Unrelated UI redesign or changing the agreed financial rules.

## Implementation plan
1. Inspect and document admin gaps; resolve only material product ambiguities.
2. Implement missing admin contracts and project/record workflows in small slices.
3. Verify backend, frontend and browser behavior, then record delivery.
