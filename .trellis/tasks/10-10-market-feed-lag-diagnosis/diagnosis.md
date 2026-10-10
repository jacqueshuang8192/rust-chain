# Market Feed Lag Diagnosis

## Scope and Safety

Read-only anonymous public BTCUSDT/ETHUSDT ticker, depth and 1m kline
subscriptions plus public ticker REST calls. No API keys, private endpoints,
account reads, trading, server restarts or production configuration changes.
The supplied exchange credentials were not used or stored.

## Directly Observed

Two bounded 30-second probes are preserved as `public-probe.json` and
`second-probe.json`. Times below are Hong Kong time (UTC+08:00), 2026-10-10.

| Measurement | First probe, ending 11:09:14 | Second probe, ending 11:11:53 |
| --- | --- | --- |
| Platform BTC ticker median age | 397,594 ms | 453,826 ms |
| Direct Bitget BTC ticker median age | 208 ms | 197 ms |
| Platform BTC REST snapshot age at end | 399,157 ms | 462,289 ms |
| Platform BTC REST response duration at end | 770 ms | 832 ms |
| Platform BTC ticker source-time advance | 22,962 ms | 14,279 ms |
| Corresponding receive-time advance | 28,690 ms | 27,911 ms |
| Increase in ticker backlog during window | 5,728 ms | 13,632 ms |

- ETH ticker and both order books show the same systemic lag; in the second
  window both platform 1m kline channels are also minutes old.
- Direct Bitget streams remain near current time. The second window contains
  some initial ~1-second arrivals, not minutes-old data.
- The REST snapshots say `source=external`, `provider=bitget`. Their HTTP Date
  headers are consistent with probe time, while `observed_at` is minutes old.
- Public REST and WS both expose old timestamps, so client rendering and
  HTTP response travel time alone cannot explain the delay.
- Source time advances more slowly than wall time. This directly establishes
  growing backlog behavior somewhere before public delivery, not just an
  unchanged last-traded price.
- The first probe records an unclassified socket error after collecting all
  four channels. The second probe explicitly classifies its socket error as
  after the requested close, not a collection-time disconnect.

## Code-Confirmed Blocking Boundaries

Current repository code (deployed commit identity has not been verified):

1. `src/workers/market_feed.rs:1583`: the upstream reader awaits
   `worker.ingest_frame(&frame)` inside its receive loop. Different channels
   and symbols on that provider connection share this serialization.
2. `src/workers/market_feed.rs:191`: each external ticker first awaits MySQL
   tick archival before entering cache/trigger ingestion.
3. `src/modules/market/infrastructure/adapters/ingestion.rs:220`: after Redis
   CAS accepts a ticker, spot and margin limit triggers are awaited in order.
   Public event publication occurs only after the sink returns
   (`src/modules/market/infrastructure/adapters/feed.rs:888`).
4. `src/modules/spot/application/triggering.rs:53`: four sequential candidate
   SELECT queries run even if no orders are eligible. Eligible orders add
   individual settlement transactions; four categories allow up to 20 each.
5. `src/modules/margin/application/trigger_limit_orders.rs:50`: one candidate
   SELECT plus up to 500 sequential per-position transactions.
6. `src/modules/market/infrastructure/adapters/ingestion.rs:371`: every depth
   snapshot also awaits the spot trigger, repeating its four queries.
7. `src/modules/market/infrastructure/adapters/ingestion.rs:442`: each accepted
   or identical-replay kline awaits Mongo index assurance, lookup and write.
   `src/infra/mongo.rs:37` actually calls `create_index` each time; there is no
   one-time memoization here. This is an index command round trip, not proof
   that Mongo rebuilds an existing identical index every time.
8. `src/workers/market_feed.rs:682`: liveness records the current read time for
   any inbound frame. It detects silence, not increasing provider-event age.
   Buffered old frames can therefore keep the connection classified as alive.

An accepted ticker normally has at least six MySQL statements even with no
eligible orders: one archive INSERT + four spot SELECTs + one margin SELECT,
plus Redis. A depth frame adds Redis and four spot SELECTs. Price equality is
not used to bypass these queries when the provider timestamp advances.
These are code-path counts, not production query-profiler measurements.

## Conclusion and Limits

Confirmed: severe, growing, cross-symbol/cross-channel server-side stale-data
delivery, already visible in the Redis-backed REST snapshot.

Strongly supported explanation: the shared serial receiver spends too long
waiting for database and matching work; backlog accumulates upstream of the
public delivery path while the liveness check continues to see readable frames.

Not yet proven: which production stage dominates (MySQL query/lock/pool time,
Mongo calls, Redis latency, or upstream/server network buffering). No server
logs, database metrics, profiler or deployed version identity were available.
Do not label a particular SQL query, index build, or database outage as the
measured sole root cause. No restart experiment was performed.

## Corrective Order

1. Instrument source timestamp, socket-read time, archive completion, Redis
   completion, trigger completion and broadcast time; include symbol/channel,
   queue age, backlog depth and database pool-acquisition time.
2. Decouple receiving and current quote delivery from slow matching/persistence.
   Use bounded, supervised workers with explicit ordering/durability semantics.
   Do not blindly drop trigger/archive events or spawn unlimited tasks.
3. Move repeated index assurance off each kline write; reduce unconditional
   high-frequency candidate scans without losing threshold-crossing events.
4. Detect stale source timestamps separately from connection silence, with an
   explicit recovery policy that does not falsify timestamps or discard audit
   evidence. A restart might temporarily clear buffers but is not a throughput fix.
5. Validate authoritative opening/settlement quotes as well as UI latency:
   seconds contracts currently read the same Redis snapshot.

## Validation

- Anonymous live probes completed and their JSON outputs parse successfully.
- `node --check probe.mjs` passed; task context validation and diff checks passed.
- No runtime source changed, so no Rust business suite was rerun.
- This report is diagnosis, not a deployed fix or proof of acceptable latency.
