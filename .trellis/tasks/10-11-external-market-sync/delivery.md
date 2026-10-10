# Delivery

## Diagnosis

- Two anonymous public samples measured BTC/ETH ticker age of about 465 and
  556 seconds, with depth/K-line and REST snapshots delayed too. See
  `diagnosis.md` and the retained sample JSON for exact windows and counts.
- Remote main was rechecked as 6530773. The earlier pipeline decoupling in
  8d07a73 is local only; the production binary revision is still unverified.
  This is a delivery gap, not evidence that an already deployed decoupled
  pipeline has failed.
- Incoming messages previously refreshed liveness even when their source
  time was persistently old. Silence detection alone cannot recover this case.
- No production slow-query or queue metrics were obtained; MySQL alone is not
  established as the cause. Direct Bitget connections failed in this round.

## Repair

- Per-connection progress checks use the provider's top-level push timestamp,
  never a candle open time or nested historical trade/ticker time.
- More than 30 seconds of source age, sustained for at least 10 monotonic
  seconds and three successfully ingested frames, ends the socket cycle.
  Existing REST fallback, backoff and subscription logic recover the connection.
- Current ingestion/publication finishes before the check. No admitted work
  is removed, no unfinished write is cancelled and order-price policy is unchanged.
- Fresh envelopes reset the observation window; missing/invalid times do not.
  Future times do not trigger recovery. Chinese warnings include source age
  and sustained-lag duration.

## Verification

- `cargo test --lib source_progress -- --nocapture`: 7/7 passed.
- `cargo test --lib --test backend_architecture --test backend_documentation
  --test market_feed_worker`: 523 + 11 + 1 + 33 passed, no failed test cases.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`, source-integrity gate (16 source inputs), task
  validation and `git diff --check`: passed.
- The new integration test uses real loopback WebSockets, ingestion,
  supervisor and public hub with a recording sink. It asserts a backlog
  failure, a second subscription, fresh data and delivery of all admitted frames.
- Existing optional database branches may skip internally. No new live
  Redis/MySQL/Mongo integration run or production recovery is claimed here.
- Combined-tree `npm --prefix mobile run release:gate`: passed, 831/831 tests,
  no skips; both type checks, PWA/Tauri web builds, artifacts and budgets passed.
- PC `node --experimental-strip-types --test tests/*.test.ts`: 119/119 passed,
  no skips; `npm --prefix pc run type-check` and `npm --prefix pc run build` passed.
- Local preview at 5186 returned HTTP 200. Earlier chart browser checks remain
  documented in that task; no new browser or native-package run in this task.

## Commit Scope

The user explicitly requested all current changes be committed. This supersedes
the earlier isolated-commit plan: include the pending Seconds chart workspace,
source-label visibility, wallet localization, tests/specs/task records and full
PROGRESS history alongside this repair. Exclude only generated `.vite/` cache
and Git-ignored local configuration/build outputs. Do not push or deploy.
Frontend gates passed against the combined working tree before commit.
Commit message: `fix: repair market stream recovery and mobile trading UI`.

## Remaining Operations

- Build/deploy a revision containing both 8d07a73 and this repair, with user
  authorization, then repeat the bounded public timestamp probe.
- Check Redis persistence/noeviction, queue capacity, four additional MySQL
  lock connections and wall-clock synchronization before deployment.
- Missing envelope timestamps cannot be monitored by this guard. Persistent
  infrastructure slowness still needs its own repair; reconnect is not a latency SLA.
- Abandoning unread frames has the normal provider-disconnection gap risk,
  not a complete event replay guarantee. Never delete durable financial work
  to conceal delay. Keep the local preview at 5186 running.

## Retrospective

- Root categories: implicit assumption (traffic means freshness), test gap
  (busy stale streams) and change delivery (unpublished local backend fix).
- Prevention: bounded source-time watchdog, deterministic clock regressions,
  loopback reconnect/publication coverage and explicit remote/deployed revision
  checks in `.trellis/spec/backend/external-market-pipeline.md`.
