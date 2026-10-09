# Initial Stale Entry Ticker Diagnosis

## Confirmed Behavior

- `src/modules/seconds_contract/infrastructure.rs::cached_entry_price` reads
  `market:ticker:{SANITIZED_SYMBOL}`. It validates positivity and rejects
  `observed_at < Utc::now() - 60 seconds`.
- `CachedTickerPayload` uses the cached observation rather than browser receipt
  time. The provider adapters preserve upstream observation timestamps.
- `src/modules/seconds_contract/application.rs::open_order` reads that price
  before order insertion, wallet debit and ledger writes. This rejection path
  therefore does not create an order or debit the stake.
- The Redis cache has no TTL. An old display value can remain readable after a
  feed stops; displaying a price does not prove it is currently executable.
- The reported pair ID does not establish its symbol or provider. No runtime
  evidence from the affected instance has been supplied.

## Verification

- `cargo test --test market_adapters -- --nocapture`: 6 passed, 0 failed.
- Read existing regression
  `tests/seconds_contract_routes.rs::seconds_contract_open_requires_fresh_positive_ticker_before_mutation`.
  It includes missing Redis/ticker, a 61-second-old observation and a zero
  price, followed by zero-order, unchanged-wallet and zero-ledger assertions.
  This real database/Redis regression was not run in this session.
- Local listening-port inspection: MySQL exists, but no API or Redis listener.
- Read-only connection using local `.env` DATABASE_URL: MySQL access denied.
  Do not mistake this local environment failure for a production root cause.

## Next Evidence Required

Read the affected instance's pair 2 metadata, cached `observed_at` and server
UTC time, and `GET /admin/api/v1/market-feed/status`. Check enabled symbols,
selected provider, applied configuration version and reload errors. Inspect
provider connection, parsing, MySQL archival and Redis-write failures.
For strategy/internal pairs, inspect the responsible generator and lease
instead of assuming an external feed.

No business code, production data or runtime settings were changed. Do not
raise the freshness limit or retimestamp an old price to make orders pass.

## Follow-up: Slow But Eventually Successful Settlement

The user clarified that settlement eventually completes, rather than remaining
pending forever.

- `src/workers/seconds_contract_settlement.rs` waits for the
  `[expires_at, expires_at + 5s)` history window to close. The default scan
  interval is another 5 seconds. Healthy low-load backend completion therefore
  usually occurs roughly 5–10 seconds after expiry, excluding processing time.
- A missing historical snapshot produces `Pending`. Both `Pending` and any
  settlement error schedule `next_settlement_attempt_at = processing_time + 60s`.
  A tick archived shortly after the first lookup therefore waits for that
  retry rather than being settled immediately on arrival.
- This 60-second delay is repeated if the next attempt is still unsuccessful.
  Errors include query/locking, invalid evidence, missing entry price and
  wallet/ledger/journal failures. Missing snapshots alone have a separate
  default 300-second wait after window closure before manual review; generic
  transaction failures are not subject to that specific missing-history path.
- Each round selects at most 100 eligible orders by expiry/ID, then settles
  them sequentially. Backlog and slow transactions can add queueing delay.
- Mobile `SecondsView.vue` reconciles unresolved expired orders at 5-second
  retry intervals. Successful backend commit sends a best-effort private
  event, but request/network delay can still postpone visible state.
- Prior stale-entry errors make delayed/stopped upstream observations or
  archival a hypothesis worth checking, not a confirmed production cause.
- `cargo test --lib workers::seconds_contract_settlement::tests -- --nocapture`:
  4 passed. These pure tests validate rules/bounds, not live settlement latency.

Next evidence: one affected order ID and approximate expiry-to-result delay,
its `next_settlement_attempt_at`, event-window tick arrival, and worker logs
for `秒合约事件时点结算失败` / `秒合约事件时点结算周期完成`.
