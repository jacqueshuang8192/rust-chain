# Delivery

Implemented Redis-only status business reads, atomic short price history, new
order entry evidence/window pricing and bounded SQL retention. Transient SQL
archive writes remain for platform-follow/recovery compatibility; unlimited
unreferenced retention is removed. Existing orders keep their pricing policy.

## Verification

- Library 525; architecture 11; documentation 1; market feed 33; OpenAPI 14.
- Real isolated MySQL/Redis: seconds routes 28, settlement worker 12, admin
  status 1, durable pipeline 1, final retention-focused rerun 1. Redis history
  and status library tests ran with explicit loopback Redis, not a skipped path.
- Strict all-target Clippy and source-integrity gate passed.
- Web full tests 1061, focused page 6, policy 15, coverage suite 23;
  typecheck/lint/build passed. Existing JS budget remains blocked:
  2,648,182 bytes versus 2,600,000. No budget change.
- Early test-only failures: JSON SQL result cast, fixture symbol normalization;
  corrected. Shared reused database runs also encountered one 1213 and leftover
  due-order interference; final full suites use separate fresh databases with
  serial test scheduling (internal concurrency assertions still execute).

Logs are under `/tmp/market-retention-*.log` and
`/tmp/market-status-web-*.log`. No production reads requiring credentials,
orders, deployments or production data deletion were performed.
The isolated MySQL instance on 13316 and Redis on 16389 have been shut down;
existing development servers and the pre-existing MySQL instance are unchanged.

## Rollout

Apply migration 0147 before code. Index creation needs a large-table migration
window. Size persistent Redis/AOF/noeviction for 24 hours of accepted ticks.
Configuration/diagnostics refresh every five seconds; cache-miss/expiry returns
503, never a SQL scan. New orders wait for the five-second deadline window;
missing history waits within the existing bound then becomes manual review.
Already-settled evidence remains after Redis expiry. Preserve legacy tick
references and unresolved windows. Do not downgrade while new-policy orders
are open. No push or deploy in this delivery.
