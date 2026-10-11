# Latest Prices And Legacy Retention

## 1. Scope

The user's explicit latest-only decision supersedes the 24-hour ticker/window
design. Realtime prices, seconds pricing, convert market rates, live follow
references and feed status use Redis latest quotes. No new ticker history is
written to Redis sorted sets or MySQL market_price_ticks. Mongo K-line history
and completed order evidence remain unchanged.

## 2. Signatures

- Latest key: market:ticker:<SYMBOL>; no quote-age gate for seconds.
- market:received:v1:<SYMBOL>: Redis receipt milliseconds.
- market:admin-status:v1: configuration/diagnostics, 30-second TTL, five-second refresh.
- seconds_contract_price_evidence: permanent entry evidence, redis_latest_v1.
- Existing settlement price/source/time/hash fields: permanent selected result.
- market_generation_price_checkpoints: UNIQUE(symbol), one overwritten runtime
  checkpoint per generated symbol; not a sequence of historical quotes.
- Old market:price-history:v1 keys are no longer read/written and expire naturally.
- Old archive queue is retired by validated exact-head ACK without SQL writes;
  spot/margin/K-line durable jobs are still processed normally.

## 3. Contracts

Seconds opening and operational settlement read a positive latest Redis quote.
Expiry is checked with database time; no additional five-second wait. Processing
delay can change the selected quote before commit, by explicit user choice.
After commit, replay uses stored evidence, never reprices or pays again. Admin
review still requires result=auto and an audited reason; no user-supplied price.
Missing/invalid Redis is an error/retry, never zero or SQL fallback. Quote age
does not reject seconds pricing; convert and follow retain their own existing
freshness policies.

External admission atomically updates latest/receipt and financial queues, with
no archive work. Old/wrong-type/full archive keys cannot block current quotes.
Generated sources keep lease/version/fence validation and one transactional
checkpoint before publication. This is required restart/idempotency state, not
ticker archival. Current checkpoints may contain price and identity, but their
row count never grows with quote count. No existing order points to these mutable
checkpoints; historical order FKs still reference immutable legacy ticks.

Live follow reads only the latest external quote. Validate it against actual
receipt time, not the frame's rounded second, preserving subsecond observations.
Historical admin follow preview may read remaining legacy records; absent history
is explicitly labelled fallback, never fabricated from current prices.

Status business reads remain Redis plus the local supervisor, with a two-second
deadline. Missing/expired metadata returns 503 MARKET_STATUS_NOT_READY, with no
SQL fallback. last_ingested_at denotes Redis receipt, not archive completion.

Apply immutable migrations through 0148 before code. It updates only pending
entry policy metadata, not completed prices or money, and seeds one generated
checkpoint per symbol from valid legacy evidence. Plan the one-time old-table
scan and stop/drain old generators before rollout. Existing legacy SQL cleanup
remains bounded and protects order references/pending windows/continuity heads.
It is legacy housekeeping, not ongoing retention of newly received ticks.

## 4. Error Matrix

| Condition | Result |
| --- | --- |
| Positive latest quote, even old | Seconds may use it immediately at expiry |
| Missing/malformed/identity mismatch | Error; no financial mutation |
| Changed latest quote after committed settlement | Replay stored result |
| Full/wrong-type legacy archive key | Does not block new admission |
| Full financial work queue | Reject before partial fan-out |
| Old generator owner/version/conflicting replay | Reject, preserve checkpoint |
| Missing status metadata | Explicit 503, no request-time SQL scan |

## 5. Cases

Good: delayed settlement uses the actual execution-time latest quote and records
it atomically. Base: only one generator checkpoint remains after many updates.
Bad: stopping archive writes but leaving convert/follow to read stale SQL prices.

## 6. Verification

Real isolated MySQL/Redis: latest-only settlement/entry evidence, replay and
rollback; convert prices without tick rows; no new Redis history or SQL ticks;
legacy archive retirement and independent financial queues; generator checkpoints,
stale-owner/replay fences. Pure follow tests cover subsecond/future observations.
Run library, architecture/documentation, feed and OpenAPI regressions plus strict
all-target Clippy. Real Mongo runtime suites require an explicit isolated service;
never describe an internally skipped optional service test as executed.

## 7. Wrong Vs Correct

Wrong: Redis latest -> ticker archive -> deadline lookup.
Correct: Redis latest -> order transaction/evidence; K-lines remain separate.
