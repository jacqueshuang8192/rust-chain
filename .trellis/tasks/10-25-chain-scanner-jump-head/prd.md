# Chain scanner recovery: skip or dual progress

## Goal and confirmed decision
User wants both selectable modes in Network and Listener: jump to latest confirmed block while preserving historical backfill, or explicitly skip historical scanning. This is a deliberate admin action, never automatic on restart.

## Requirements
- Admin network-write action with pinned revision, mandatory reason and explicit skip acknowledgement.
- Queue recovery under the network lock; worker validates network/assets/current checkpoint and current confirmed head using existing provider gates before applying it atomically. Node failures leave the old cursor untouched.
- Dual progress saves the old cursor and a fixed head anchor for historical backfill; real-time scanning can reopen admission without waiting for history. Repeated dual recovery extends the existing range. Skip explicitly discards the skipped range AND any existing backfill.
- Separate persisted history cursor, target, error and next-at. Latest scanning has priority; allow a bounded historical round only while latest status is recent, at most once per 15 seconds. Shared provider/Key pacing remains in force.
- Historical transport outages are shown separately; historical chain/continuity/accounting failure closes admission until resolved, and cannot be erased by an otherwise successful latest scan.
- Durable request and completion audit includes actor/reason and old/new checkpoints. No balance/order deletion or rewriting. Existing receipt idempotency handles overlap/replay.
- UI labels distinguish queued recovery, latest cursor, historical cursor/target and history errors. Confirmations and finalized semantics unchanged.

## Plan
1. Add persistence, DTO, permission-mapped action and audited request.
2. Extend worker to apply recovery and schedule fenced historical scans.
3. Add mode selection, explicit warnings, confirmation and progress UI.
4. Verify migration, recovery/failure/race/accounting contracts, HTTP permissions and frontend behavior; update specs/progress.

## Acceptance
- [x] Both modes work and remain persistent after restarts.
- [x] Latest status recovers without waiting for retained historical backlog.
- [x] Failed/stale/unauthorized operations cannot mutate cursor or balances.
- [x] Overlap/replay cannot double credit; historical anchors are checked before settlement.
- [x] Backend, admin and real API browser validation completed.

## Release / non-goals
New immutable migration, coordinated backend/admin release. No live database cursor mutation, live transfer, deployment or remote push in this task.

## Delivery status
Implemented and locally verified; see research/delivery.md. User authorized commit and push on 2026-10-25; preparing origin/main delivery. No deployment.
