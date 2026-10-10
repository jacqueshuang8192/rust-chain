# Delivery

## Implementation

External socket reception now waits only for atomic Redis cache/queue admission
before public publication. Archive, spot, margin and candle history run in four
independent supervised FIFO consumers. Source-time cutoffs exclude orders created
after queued observations. Existing financial transactions, per-order failure
isolation and seconds pricing rules remain in place.

## Final Verification

- `cargo fmt --all`: passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- Library 516, architecture 11, documentation 1 and market feed worker 33 tests:
  passed with explicit isolated external-pipeline Redis/MySQL environment.
- Spot 65, margin 44 and Redis cache 5 integration tests: passed against the
  task-owned schema `market_pipeline_20261010_test` and loopback Redis 16482.
- Pipeline regression exercised bounded/atomic admission, archive retries,
  microsecond identity, stale suppression, local replay publication,
  source-time order cutoffs, 500 frames during a blocked spot lane,
  independent consumers, restart recovery and Mongo failure retention.
- Source integrity: 16 inputs passed. Task context validation and diff checks passed.

Some unrelated library tests internally skip optional database branches; the
library count is not a claim that every existing database branch was exercised.

## Limitations And Deployment

No successful live Mongo persistence/index-initialization test was available.
No deployment or post-change production latency measurement was performed.
The observed backlog and serial code path do not establish MySQL's individual
share of production latency.

Require persistent standalone Redis with noeviction and sufficient capacity,
an authoritative MySQL primary and four additional MySQL connections.
Queue capacity is finite: prolonged failures eventually backpressure admission.
Keep queued events; repair failed dependencies rather than deleting queues.
Drain v1 queues before downgrade. See the external-market-pipeline backend spec.

Commit only this task's source, tests, spec and records. Do not push or deploy.
Unrelated UI changes and shared historical progress entries remain untouched.
