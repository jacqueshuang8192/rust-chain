# Base RPC pacing and adaptive batch delivery

## Result

The durable gate now waits without reserving future request slots and rechecks
the latest provider/key deadline, network revision, lease and cooldowns under
the established lock order before admission. Cancellation leaves no future slot.
This fixes a reproduced case where a pending request ignored a later completion
deadline, and avoids releasing multiple expired reservations after a scheduler
stall.

Migration 0167 adds `rpc_batch_limit` to key state. A Base transaction-batch
HTTP 429 reduces the cap using the rejected batch size, persists it atomically
with the genuine key cooldown, and leaves the failed checkpoint unchanged.
Fresh candidates use the learned cap. Duplicate failures do not repeatedly
halve the same batch, success/restart retains it, blank saves preserve it and
explicit endpoint/credential replacements reset the relevant caps. Smaller
caps also bound native round block work. Token parsing and financial evidence
are unchanged.

The user confirmed Free Plan, sufficient monthly allowance and no displayed RPS.
This change does not invent an RPS allowance, buy quota or enable methods the
account previously rejected. It is feedback-based envelope control, not a promise
of sustained full-chain indexing on any Free Plan.

## Verification

- Red before the fix: the isolated deadline regression admitted a request about
  204 ms after a sibling response even though the required gap was 500 ms.
- Green: two new isolated MySQL tests cover delayed/concurrent admission,
  cancellation, shared/scoped cooldowns, disabled candidates, lease/revision
  fencing, an actual 25-call HTTP 429 followed by complete 12-call scans, fresh
  candidate loading, complete 98-transaction coverage in two blocks, cap floor,
  independent key state, configuration replacement and legacy writes.
- Migration: upgrade from 0166 preserves key and stream snapshots, backfills 25,
  and the SQLx migrator can run again without changes.
- Expanded scanner run: 62/63 passed on the first run. The old trace-upgrade
  fixture tried to run the new worker without the new column and failed before
  exercising cooldowns. It now withholds only repair migration 0166 while
  preparing current schema; its existing zero-request, progress and unrelated
  cooldown assertions are retained. Its targeted rerun passed (1/1), so all 63
  selected scanner cases were actually executed and passed across those runs,
  with all required isolated database URLs configured and no fixture skips.
- Final static/architecture checks passed: `cargo clippy --lib --bins -- -D warnings`,
  `cargo check --all-targets`, `cargo test --test backend_architecture` (11/11),
  and `cargo fmt --all -- --check`. The final `RequestWindow` row type replaced
  a nested tuple to satisfy Clippy without suppressing the lint.
- Final database regression after that row-type change passed (1/1, 41.82 s) on
  a freshly created isolated database:
  `cargo test --lib scanner_gate_rechecks_delayed_admission_and_cooling_before_send -- --nocapture`.
  This executes the deadline, 429, smaller-batch and configuration/fencing checks.
- `git diff --check`, Trellis context validation and scoped credential/whitespace
  checks passed, including the new migration and untracked tests/task records.

## Deployment

Stop old deposit workers, build/pull the new backend and migration runner, apply
all pending migrations including 0167, then start the new workers. No frontend
DTO or UI change is required. Migration 0167 does not clear existing HTTP 429
cooldowns. Wait for their recorded deadlines and verify actual live/backfill
progress and admission after complete successful rounds. Very low sustained
capacity or dense blocks can still lag or hit the existing attempt deadline.

No production database, live Nodies credential, real deposit or remote CI was
used for this validation. The pre-existing agent-management edits remain local.

## Bug analysis

1. Root categories: implicit timing assumption and coverage gap. Future request
   reservations were treated as valid even after response completion changed
   the deadline; a small successful batch was only a compatibility sample.
2. Earlier fixes addressed distinct failures: trace entitlement, old persisted
   trace cooldown and stream isolation. They did not verify sustained RPC burst
   behavior. Do not describe this 429 as evidence that trace is still invoked.
3. Prevention: real MySQL timing/fencing tests, mock 429 feedback, persisted cap
   lifecycle tests, migration snapshot checks and updated scanner contracts.
4. Scope: all networks benefit from the shared admission race fix; only Base
   transaction batches learn a cap. Shared quota remains shared across streams.
5. Knowledge: `.trellis/spec/backend/timed-deposits.md` and the operator guide
   record these invariants and deployment limits. There is no corresponding
   `src/templates/markdown/spec/` tree in this application repository.
