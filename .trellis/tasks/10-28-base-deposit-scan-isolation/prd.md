# Base native / ERC20 scanner isolation

## Goal and authorization

The user requested splitting the shared ETH/USDC scan after discussing how a
native scan failure delays token deposits. Implement independent Base native
and ERC20 progress, readiness and recovery. Existing unrelated work stays intact.

## Requirements

- Base has native and token streams, each with its own persisted live cursor,
  backfill, recovery request, lease, freshness and error state.
- Copy both streams from the existing checkpoint/history without jumping heights;
  require a successful scan before admission after migration.
- Filter assets, historic confirmation requirements and initialization checks by
  stream. Historical disabled assets remain scanned; receipt identity is unchanged.
- Keep network configuration revision fencing and settlement transactions. Shared
  provider/key request pacing and real HTTP quota cooldowns remain global.
- Scope RPC method/protocol failures and scan-attempt timeouts to the affected
  stream so native capability failures cannot poison the token provider pool.
- Admission and admin status use the asset's stream. Admin can recover a selected
  stream; old recovery payloads target both Base streams. Other networks preserve
  existing behavior.
- Add meaningful real-MySQL/mock-RPC tests for failure isolation, credit/replay,
  migration preservation, recovery and stale configuration/lease fencing.

## Acceptance criteria

- [x] Native RPC failure leaves native cursor unchanged while token progresses and credits USDC once.
- [x] Token failure does not block native progress; shared 429 still respects quota.
- [x] Native errors do not hide token options or reject valid token order creation.
- [x] Migration preserves cursors/history/requests and clears readiness/leases.
- [x] Independent recovery and backfill remain audited, version-fenced and replay-safe.
- [x] Admin displays both streams and sends the selected recovery scope.
- [x] Focused Rust/database/Web checks and relevant static gates pass; limitations recorded.

## Decision

Use a Base-only stream-state table and a scoped provider-failure table. Reuse
scanner algorithms and the existing provider/key pool. Splitting by native/token
isolates the expensive native transaction path without introducing per-token
workers or multiplying provider quota. A separate-token-table-only approach was
rejected because network status would ambiguously mean native status.

## Out of scope

No RPC purchase/configuration, parallel provider load balancing, per-token
isolation, change to Ethereum/internal transfer support, production writes,
deployment or remote push. Real shared provider outages can still affect both.

## Plan

1. Schema, stream persistence/scheduling and scoped provider failures.
2. Asset admission, recovery/API/admin status and regression tests.
3. Isolated database/RPC verification, frontend/static gates and documentation.
