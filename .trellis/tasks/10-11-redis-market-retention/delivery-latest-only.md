# Latest-Only Revision

The user explicitly rejected ticker history. This revision supersedes the window
pricing and short-history retention described in delivery.md.

## Delivered

- Redis latest ticker only: no sorted-set history writes or reads.
- External feed no longer enqueues/writes SQL ticker archives. Legacy archive
  jobs are validated and retired by exact-head ACK; money/K-line queues remain.
- All operational seconds settlement uses execution-time latest Redis prices
  immediately at expiry, with entry and settled evidence retained atomically.
  Explicit reviewed-order recovery keeps authorization, auto result and audit.
- Convert market rates and live follow read Redis, not stale SQL prices.
- Generated prices use a single overwritten checkpoint per symbol for leases,
  idempotency and restart continuity, not an ever-growing ticker archive.
- K-line storage is unchanged. Old SQL price evidence/FKs are preserved; existing
  legacy cleanup remains bounded. Old Redis history expires without new writes.

## Verification

- Latest library: 526 passed (explicit real Redis and MySQL live-follow tests).
- Durable pipeline: real MySQL/Redis; no new tick rows, no sorted-set history,
  obsolete archive key does not block admission, FIFO/restart/ACK tests passed.
- Seconds routes: 28; settlement worker: 12; convert routes: 20; Admin default
  configuration/capability/preview routes: 17, all with isolated real MySQL/Redis.
- Synthetic ticker checkpoint/lease/replay test: 1 real MySQL/Redis case passed.
- Upgrade from migrations through 0147 to 0148: 1 real case passed, including
  repeated SQLx migration, single-checkpoint backfill, invalid old identity
  exclusion, pending policy update, completed price/entry evidence and balance
  preservation.
- Architecture 11, Chinese documentation 1, feed 33, OpenAPI 14, existing
  migration suites 2+1; follow/config/reference/default pure suites 8+5+4+8.
- Strict all-target Clippy, fmt, source integrity and task validation passed.
- Full Mongo-backed generation/transition/follow runtime suites were NOT run:
  no isolated mongod is available and Docker daemon is stopped. Their fixture
  contracts were updated for one checkpoint, but this is not execution evidence.
- No frontend source changes this revision; previous Web bundle budget issue is
  unchanged, not repaired or reclassified as passing.

Logs: /tmp/latest-*.log. Only isolated loopback MySQL 13316 / Redis 16389 were
used for persistent tests. No production cleanup, deployment or push performed.
Both isolated services were shut down after verification; pre-existing services
and frontend development servers were left unchanged.

## Rollout

Apply all migrations through 0148 before code. Existing applied migrations are
unchanged. The one-time generator backfill scans old ticks; plan a migration
window and stop/drain old generator binaries. New pricing is explicitly
processing-time pricing: delayed execution can change the quote before commit.
Already-committed orders always replay their stored result. Missing Redis does
not authorize a guessed price or a SQL-history fallback.
