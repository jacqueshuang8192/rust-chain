# External Market Realtime Pipeline

## 1. Scope

External Bitget/HTX/Coinbase WebSocket and REST fallback ingestion in
`workers/market_feed.rs`. Synthetic/default generation keeps its synchronous
provenance, lease, archive and checkpoint contracts. Seconds opening/settlement
continues using the existing Redis price policy; this change does not introduce
deadline-price selection.

## 2. Signatures

- `RedisMarketCache::admit_external(&ExternalMarketEvent)` returns the existing
  `MarketCacheWriteOutcome`.
- `MarketIngestionSink::accept_{ticker,depth,kline}` returns publication permission.
- Redis lists: `market:external-work:v1:{archive,spot,margin,kline}`.
- Each list is capped at 50,000 unacknowledged observations; no TTL or trimming.
- Queue envelope: `event` plus full-precision UTC `observed_at`. Public snapshot
  timestamps stay milliseconds; archive identity retains microseconds.
- Consumers use one dedicated MySQL connection per lane and a database-scoped
  `GET_LOCK(SHA2(CONCAT(DATABASE(), ':', lane_key),256),0)` session lock.
  These connections are additional to the application pool.

## 3. Contracts

Reception validates/parses the frame, performs one Redis admission script and
publishes an accepted snapshot immediately. It never awaits SQL archive,
candidate queries, order transactions, Mongo index commands or candle writes.
Ticker/cache and accepted trigger work are written together. Check all queue
types/capacities before writes: Lua runtime errors do not roll back prior writes.

Ticker and depth order by observation time; candle order is
`(open_time, observed_at)`, retaining the existing sequence key even if the
public JSON key disappears. Equal-time conflicting payloads cannot overwrite.
Identical replay does not enqueue triggers again, but may broadcast the confirmed
current snapshot: Redis is shared across instances while each public hub is process-local.
Suppressing every replay would starve clients on instances that lose the Redis
CAS race. Every valid ticker, including replay/stale/conflict, enters the
idempotent archive lane, preserving historical evidence and distinct microsecond
observations whose public millisecond JSON happens to match. Stale/conflicting
tickers never trigger orders or broadcast. Stale depth/candles have no side effects.

Four supervised consumers progress independently. Archive batches up to 100
rows into one idempotent INSERT; other lanes process one observation at a time.
FIFO is receive/admission order, including ticker and depth sharing the spot
lane, not a promise to globally reorder different providers by source time.
No unbounded spawn, pending-job eviction or detached financial tasks.

Read without removing the head, process, verify session-lock ownership, then
conditionally acknowledge the exact original head payloads. On SQL, Redis or
Mongo or candidate-query failure, retain the head and reconnect with bounded
backoff. Preserve existing per-order failure isolation and candidate limits:
one unfunded/broken order rolls back, logs and waits for later observations,
not an infinite retry of the same old execution price blocking the whole lane.
Acknowledgement means delivery to the existing bounded trigger use case, not
a guarantee that every eligible order filled; no new per-order retry system is
introduced here.
Previously committed fills are skipped by existing terminal-state/transaction
checks on retry. Private hints stay post-commit, lossy and process-local.
Cancellation finishes the current operation; queued work survives API restarts.
Reload does not let old producers write realtime caches; accepted historical
jobs may finish later without rewriting caches or broadcasting old prices.

Queued triggers require `order.created_at < observation_time`. Equal/later
orders are excluded. Legacy synchronous callers pass no cutoff. Candidate reads
for the four spot categories run concurrently; financial transactions remain
sequential within the lane with their existing wallet locks and journals.
Cancellation/terminal-state checks remain authoritative on replay.

Successful Mongo index assurance is cached with a shared, per-service,
per-symbol `OnceCell`; failure remains retryable. A Mongo freshness conflict
requires rereading the winning document. Only a verified strictly newer candle
supersedes the queued one; a first-insert race won by an older document must retry.

## 4. Error Matrix

| Condition | Result |
| --- | --- |
| Invalid price/provenance/frame | No realtime acceptance |
| Full or wrong-type required queue | Error; no new cache or partial fan-out |
| Stale ticker | Historical queue only; no triggers/public hint |
| Duplicate accepted observation | No repeated trigger admission; idempotent archive and current local hub snapshot allowed |
| Slow archive/spot/margin/Mongo | Other lanes and realtime admission continue within queue capacity |
| Failed candidate query / archive / Mongo write | Retain head, report and retry |
| Individual order cannot execute | Existing rollback/log/continue policy; later observations may retry |
| Competing API instances | One normal-operation consumer per lane via session lock |
| Lost lock/uncertain ACK | Leave/replay work; existing financial idempotency remains required |
| Feed disabled or API down | Work remains; consumption resumes when feed runtime is enabled |
| Redis persistence loss/eviction | Not protected by the SQL archive until consumption; operational incident |

## 5. Operations And Cases

Good: blocked spot processing does not delay a new ticker's Redis value/public
event or prevent archive/margin progression.
Base: consumer stops after an archive commit and before ACK; replay creates no
second tick.
Bad: enqueue only in memory, trim oldest unprocessed ticks, use `join!` inside
the socket read loop, or fill newly created orders against queued old prices.

Deployment requires persistent standalone Redis with `noeviction`, sufficient
memory and the existing AOF volume. AOF's fsync policy determines the Redis-host
crash durability window; API restart recovery is not a claim of zero-loss Redis
disaster recovery. The existing Compose AOF configuration is retained.
Use a single authoritative MySQL primary; advisory locks are not a distributed
fencing protocol across independent primaries/failover split brain.

Inspect `LLEN` and the queued observation timestamp for each lane. A failed
head deliberately stops that lane and can eventually fill the admission bound:
repair the dependency/payload failure, do not delete the list to hide backlog.
WARN reports consumer failures and queues >= 1,000; DEBUG reports
`source_age_ms`, `elapsed_ms` and `queued`. Public realtime and historical
storage now converge asynchronously. Drain all v1 lists before downgrading to
a binary that does not understand them.

## 6. Verification

`tests/unit_src/src_workers_market_feed_side_effects_tests.rs` uses explicit
loopback `EXTERNAL_MARKET_TEST_DATABASE_URL` (an `_test` schema) and
`EXTERNAL_MARKET_TEST_REDIS_URL` (empty isolated database), never generic
production environment fallbacks. Cover wrong-type/full admission, replay,
stale public suppression, generation fencing, durable retry, conditional ACK,
batch archive idempotency, source-time precision, slow lane independence,
Mongo failure retention, consumer restart and creation-time cutoffs.
Also run library, market feed, architecture, existing spot/margin fill tests,
format, all-target compilation and strict Clippy.

## 7. Wrong Vs Correct

```text
Wrong: read socket -> archive -> match spot -> match margin -> publish
Correct: read socket -> atomic Redis cache/queue admission -> publish
         supervised independent FIFO consumers -> commit -> conditional ACK
```
