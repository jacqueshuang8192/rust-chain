# Redis Prices And Short Retention

## 1. Scope

Realtime status and new seconds-order pricing no longer query SQL tick history.
Unlimited retention is replaced with a 24-hour working window. Asynchronous SQL
ingestion remains short-lived for platform follow, continuity and legacy manual
recovery. Mongo candle history is unchanged.

## 2. Signatures

- `market:price-history:v1:<SYMBOL>`: sorted set with microsecond event-time score,
  `PriceObservation` JSON member, exact decimal and RFC3339 time.
- `PRICE_RETENTION_SECONDS = 86400`, shared by SQL cleanup and Redis history.
- `market:received:v1:<SYMBOL>`: Redis-server receipt milliseconds.
- `market:admin-status:v1`: configuration and K-line diagnostics; 30-second TTL,
  refreshed every five seconds independently of feed enablement.
- `seconds_contract_price_evidence`: immutable entry JSON and `redis_window_v1`
  policy per new order. Settlement evidence stays in existing order columns.
- `prune_price_batch(pool, cursor)`: checks up to 500 expired candidate keys per transaction,
  at most 5000 per ten-second round, cycling at the end.

## 3. Contracts

Latest ticker/history insertion and expiry are atomic Lua writes. Validate key
types before changing any cache/queue. Only accepted observations enter Redis
history; replay does not duplicate history. Prune by Redis wall time, with idle
TTL; old source time never extends the event-time window.

New orders commit entry evidence with wallet debit/order insertion. Select the
first accepted tick in `[expires_at, expires_at + 5s)` after the window closes.
Delayed execution must not substitute a later latest price. Legacy orders without
evidence keep their established pricing policy. The five-second wait is not
exchange finality for arbitrarily late network data. Missing history follows
bounded waiting/manual review; dependency errors remain retryable. New-policy
manual review also requires Redis history; after expiry/loss, use the existing
audited refund workflow, not a fabricated/current/SQL-conflicting price.

Status business reads only Redis plus process-local supervisor state; existing
authentication remains unchanged. Bound reads to two seconds. Provider must match
configuration. `last_ingested_at` now denotes Redis admission, not SQL archive.
Config/diagnostics are eventually updated within five seconds. Missing/older-than-
30-second metadata returns 503 `MARKET_STATUS_NOT_READY`, with no SQL fallback.

SQL cleanup requires both observation and ingestion older than 24 hours. Preserve
order-referenced ticks, unresolved opened/manual-review windows and the last tick
per `(symbol, source)` for continuity. Advance cursor only after commit; failures
keep data. Retention is eventual, not exact-time erasure; monitor cleanup backlog.

Migration 0147 creates indexes/evidence table, but deletes nothing itself. Apply
before deploying code. Keep persistent standalone Redis/AOF/noeviction. Do not
downgrade to binaries ignoring evidence policy while new-policy orders are open.
Redis loss can remove unsettled history; already-committed order evidence survives.
Existing durable queues are consumed normally, never deleted to hide backlog.
Size Redis for the full 24-hour accepted-tick rate including sorted-set overhead;
time retention is not a fixed byte cap. Monitor memory and cleanup throughput.
Index creation on a large existing tick table needs a planned migration window.

## 4. Error Matrix

| Condition | Outcome |
| --- | --- |
| Wrong history type/full financial queue | Reject before writes |
| Missing latest/window | Reject entry / wait then manual review |
| Later latest quote | Never replaces deadline-window evidence |
| Missing/expired status metadata | 503, never false healthy |
| Cleanup failure | Roll back, retain cursor/data and retry |
| Referenced/pending/head tick | Preserve beyond retention |

## 5. Cases

Good: a late settlement selects its deadline-window quote. Base: expiry cleanup
leaves order evidence intact. Bad: deleting all old ticks without reference checks.

## 6. Verification

Isolated Redis: wrong-type atomicity, replay, microsecond boundaries, later-price
exclusion, TTL/trimming. Isolated MySQL/Redis: window settlement/replay, exact
decimals, permanent evidence and cleanup reference/pending/head guards. Status:
no SQL dependency, source isolation, receipt timestamps and stale metadata.
Run legacy settlement/routes, OpenAPI, library/architecture/documentation and
strict all-target Clippy.

## 7. Wrong Vs Correct

Wrong: status HTTP -> aggregate all ticks; due order -> current price.

Correct: status HTTP -> Redis metadata/quotes; due order -> immutable policy ->
short Redis window -> permanent evidence in the money transaction.
