# Market Feed Lag Diagnosis

## Scope

Investigate the minutes-old BTC/ETH spot prices observed in the seconds UI.
Perform anonymous public WebSocket/REST sampling and inspect the ingestion hot
path. Separate directly observed facts, code-confirmed blocking boundaries and
unverified production bottlenecks.

## Boundaries

No credentials, private account calls, trades, production configuration changes,
service restarts or database writes. Do not change financial processing semantics
while diagnosing. Existing uncommitted UI changes remain untouched.

## Plan

1. Compare ticker/depth timestamps across two symbols and direct Bitget.
2. Compare REST response latency with cached snapshot age.
3. Trace serial ingestion and count database work even with no eligible orders.
4. Record evidence, limitations and prioritized corrective actions.

## Approved Implementation

The user approved separating realtime publication from archive and order triggers.
Local source changes and isolated tests are now in scope; production writes,
deployment and remote pushes remain out of scope.

- Atomically accept the external snapshot and append bounded Redis work queues.
  Reject stale snapshots before publication. Never trim unprocessed events.
- Run independent, supervised archive, spot, margin and candle consumers.
  Keep FIFO order per lane, retry failed heads, acknowledge after processing,
  and retain queued work over process restarts.
- Use a MySQL session advisory lock to serialize each lane across instances.
  Locks use dedicated connections, not permanently borrowed application pool slots.
- Archive historical observations independently of current configuration generation.
  Old jobs must never rewrite the latest Redis price or publish old public quotes.
- Exclude orders created after the triggering observation when replaying queued work.
- Preserve synthetic provenance transactions and current seconds pricing semantics.
  This task does not switch seconds settlement to historical deadline pricing.
- Cache successful Mongo index initialization per ingestion-service instance.
- Verify atomic admission, bounded capacity, stale/replay suppression, retry/restart,
  FIFO, cross-lane independence, creation-time cutoffs and shutdown behavior.

## Delivery Plan

1. Add durable admission and independently supervised consumers.
2. Connect only the external feed path; preserve synthetic and API paths.
3. Add focused regression tests, run Rust quality gates and update contracts.
4. Commit only this implementation and its records; do not push or deploy.
