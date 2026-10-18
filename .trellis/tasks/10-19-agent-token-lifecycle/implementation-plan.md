# Implementation slices

1. Versioned issuance contract and review: new immutable policy snapshot with fixed subscription price, initial market seed and reference listing price, four independent node modes/times, IANA timezone, five lock choices and three optional fee bases. Publish inside current asset/wallet/project approval transaction; old applications stay v1.
2. Stage execution: manual commands for permitted issuer/Admin actors and durable bounded schedule processing. Record actual times; never skip manual or incomplete distribution gates. Publish the initial synthetic market at purchase; retain the same pair, price and candles at listing. Never reanchor.
3. Scoped selected-batch allocation: new Agent single/batch endpoints based on buyer subtree, server-side membership locked again at mutation time. Ratio is explicit per selected batch; no automatic allocation. Reuse exact original subscription-price refund calculations and terminal receipt behavior.
4. Holding and buyback: immutable acquired quantity/cost/lock/fee snapshots, eligible-at read, one-time user-bound sale quote, atomic token consumption/gross proceeds/fee/net credit/audit/journal. New holdings excluded from legacy auto-release paths; shared asset spend paths cannot double-spend a tracked lot.
5. Agent/Admin UI: full application/review details, own project stage controls, scoped order selection with percentage preview and final confirmation, persisted retry results.
6. User UI: Mobile and PC project stages/prices/timezone display, synthetic chart/holding returns, eligibility and atomic unlock-sell confirmation with exact net proceeds.
7. Validation and rollout: real disposable MySQL, quote source fixtures, worker restart/concurrency, unchanged v1 tests, Web/Mobile/PC gates and browser desktop/narrow; document migration/runtime compatibility. No production deployment in this task.

## Implementation defaults
- All four phase nodes choose manual/automatic individually. Auto distribution opens the phase only.
- A manual batch is a final allocation of each selected pending order, not an installment schedule. Already settled orders cannot receive a second refund/allocation.
- Relative time starts at the original purchase/subscription acceptance timestamp, not the later allocation click. Weeks are seven-day durations; months follow calendar arithmetic in the snapshotted timezone, month-end clamps to the destination month's last day.
- Store UTC instants and original IANA timezone. Display according to each viewer's current timezone, without changing the underlying event. Reject ambiguous/nonexistent entered local times unless an explicit valid offset resolves them.
- Fixed fees must be denominated in the same quote asset as proceeds; profit fees use max(gross minus allocated original cost,0). A fee cannot create negative proceeds or require a separate prepayment.
- Quote expiry/future time, unavailable engine or inconsistent market identity blocks buyback; no fabricated issuance-price fallback.
- No automatic sell at lock maturity or actual phase transition; those events change eligibility only.
