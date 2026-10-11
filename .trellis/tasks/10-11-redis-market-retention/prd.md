# Redis Market Snapshots And Bounded Retention

## Superseding User Decision

The user explicitly rejected all ticker history: use Redis latest prices only.
This supersedes the 24-hour/window design below (retained as historical context).
Stop Redis history writes and external SQL ticker archival. Seconds opening and
all operational settlement use current Redis prices with no five-second wait.
Keep order evidence, K-line history, money queues, and bounded generator restart
checkpoints (one overwritten row per symbol, not event history). Move live convert
and follow references off tick archives. Preserve existing historical order FKs;
do not delete production data or rewrite applied migrations. Legacy archive work
is retired explicitly without new tick writes; spot/margin jobs still run.

## Goal

Serve realtime prices and feed status from Redis, replace unlimited tick retention
with a short window, and preserve prices actually used by orders.

## Design

- Default short retention: 24 hours; user was offered 7 days, no override received.
- Redis holds latest prices plus bounded, expiring event-time ticker history.
- New seconds orders retain entry evidence; window-priced settlement reads Redis
  history, never a later latest price. Legacy orders keep their established policy.
- Keep short-lived SQL ticks for existing platform-follow/recovery dependencies;
  a bounded background collector removes expired, unreferenced rows. Preserve
  order-linked evidence, unresolved order windows and per-source continuity heads.
  This cancels unlimited retention, not every transient SQL write.
- The status request reads cached configuration/diagnostics and current Redis
  ticker timestamps, with no request-time SQL aggregation. Background refresh
  computes K-line diagnostics. Missing/stale metadata is explicit, not healthy.
- Do not delete existing production data or deploy during implementation.

## Acceptance

- Redis publication/history admission is atomic and wrong-type safe.
- Delayed settlement chooses the same deadline-window event; evidence survives
  history expiry, with exact decimal values and idempotent wallet transactions.
- Cleanup never deletes order references, unresolved windows or continuity heads.
- Status does not query market_price_ticks or wait for SQL in the HTTP path.
- Real isolated Redis/MySQL tests, affected Rust tests, fmt and strict Clippy pass.
- Document retention, migration, rollback and Redis-loss limitations.
