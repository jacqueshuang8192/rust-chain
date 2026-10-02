# Backend Recharge Tests

## Scope

- Owner: dispatched trellis-check, no nested agents.
- Owned files: `tests/agent_routes/recharges.rs`, module declaration in
  `tests/agent_routes.rs`, `src/openapi/agent_recharges.rs`, registration in
  `src/openapi.rs`, additions to `tests/openapi_routes.rs`.
- Main subsequently authorized the bounded lock-scope fix in
  `src/modules/agent/infrastructure/recharges.rs` and correcting only the
  `get_admin_recharge` documentation in `application/recharges.rs`.
- Existing new-coin market changes are preserved. No web files are edited.
- Shared `docs/superpowers/PROGRESS.md` is deliberately not edited, per dispatch.

## Coverage Implemented

- Pending creation has no wallet, ledger or journal effect; 12 simultaneous
  approvals and exact replay preserve one credit and two balanced journal legs.
- Rejection, opposite-decision races, and terminal actor/action/reason conflicts.
- Explicit opt-in, noninheritance, optimistic authorization versions, immediate
  revocation for new intents, original-record replay, and no pending auto-credit.
- Own/descendant scope, sibling/parent/unassigned/missing/self rejection,
  current-subtree history and filter-consistent totals, reassignment before
  approval, and rejection of stale requests.
- Full account-scoped intent binding, raw case-sensitive keys, separate portal
  accounts, different users/accounts submitted simultaneously, and simultaneous
  same-key/different-target conflicts without gap-lock deadlocks.
- Active ancestor/agent/portal account and target-user validation.
- Exact independent read/review/authorize permissions; `agents.write` cannot
  authorize. Agent/admin token scope and missing authentication are checked.
- Precision 0/2/18, trailing zeros, smallest 18-place amount, invalid stored
  precision, malformed/string-only DTOs, storage overflow and existing-wallet
  overflow; successful replay after asset disablement/precision changes.
- Target-specific required-audit and journal trigger failures prove atomic
  pending/direct/approval/rejection rollback; same-intent retry succeeds.
- Authorization-audit failure rolls back flag/version; 8 simultaneous same
  expected-version changes allow exactly one success.
- Admin detail reads reconcile another reviewer's terminal result after a
  local 409, including after reassignment, without changing funds or audit.
- A real request is held on its asset lock after acquiring scope locks;
  unrelated-agent authorization must still finish. The test observes
  `performance_schema.data_lock_waits` instead of guessing with a fixed sleep.
- Missing, duplicate, noncanonical and over-depth ancestor paths fail closed.
- Both OpenAPI aliases expose eight authenticated operations, exact permissions,
  safe integer bounds, decimal strings, required nullable fields and millis.

## Contract Clarifications

- Reported `AgentRechargeRequest.amount` accepting JSON numbers through the
  shared decimal deserializer; main agreed to enforce string-only input.
- Malformed or storage-impossible decimal text is rejected by Axum JSON
  deserialization with 422; valid decimal text exceeding asset precision is 400.
- Replayed successful submission may return the original row's current state;
  it does not manufacture a second receipt or credit.
- Tests use scoped fixtures in a disposable MySQL database. A module mutex
  prevents deliberately invalid active-asset metadata from racing option reads;
  concurrency is exercised explicitly inside the cases, not removed.

## Review Finding Fixed: Overbroad Agent Locks

- Original `lock_recharge_scope` used reverse `LIKE` predicates and
  `ORDER BY id FOR SHARE`. `EXPLAIN` on isolated MySQL 9.3.0 selected a PRIMARY
  full index scan (`type=index`, 72 rows).
- A read-only diagnostic transaction returned exactly `/agent:1`, while
  `performance_schema.data_locks` showed 73 RECORD S locks for that thread:
  all 72 stored agents and the supremum. The transaction was rolled back.
- This made one recharge block authorization updates for unrelated agents.
- The fix validates canonical path IDs, leaf identity, nonzero IDs, duplicate
  path occurrences and maximum depth; a BTreeMap deduplicates/sorts the lock
  set. At most three individual primary-key locking reads avoid an
  optimizer-selected broad scan. Every locked ancestor must exist and retain
  the expected complete path. Existing active-state, session and ownership
  checks remain, with no consistent read introduced before replay lookup.
- Main owns final spec synchronization; record this lock contract in the
  recharge/agent hierarchy spec.

## Verification (2026-09-22, Complete)

- `cargo test --test openapi_routes --test agent_routes --no-run`: passed
  initial compilation (2m32s including a concurrent Cargo build lock).
- `rustfmt --edition 2024 --config skip_children=true` on owned Rust files:
  passed.
- Initial real-MySQL run: 12/14; two fixtures exceeded existing username
  length because of long labels. Shortened only those labels.
- Recharge suite after fixture corrections: 14/14 passed (14.35s).
- Complete agent routes before detail/lock additions: 37/37 passed (44.37s),
  including all 23 existing cases, with `DATABASE_URL` set.
- `cargo check --test agent_routes --test openapi_routes`: passed.
- `cargo check --all-targets`: passed after the bounded lock fix.
- `cargo clippy --test agent_routes --test openapi_routes -- -D warnings`:
  passed after all final code/test changes (19.93s final invocation).
- Final targeted command:
  `DATABASE_URL=mysql://root@127.0.0.1:13316/agent_recharge_test cargo test --test agent_routes recharges:: -- --test-threads=1 --nocapture`
  passed 17/17 after the detail operation and lock fix (19.24s).
- Final combined command:
  `DATABASE_URL=mysql://root@127.0.0.1:13316/agent_recharge_test cargo test --test agent_routes --test openapi_routes --test backend_architecture --test backend_documentation -- --test-threads=1`
  passed agent routes 40/40 (48.49s), architecture 11/11 and documentation 1/1.
  OpenAPI initially hit its test reader's 256 KiB body limit after the eighth
  operation was added, before JSON parsing. The bounded test reader now allows
  1 MiB; no frontend bundle budget or release-policy threshold was modified.
- `cargo test --test openapi_routes`: final 12/12 passed (0.33s), including both
  aliases, all eight recharge operations and preserved new-coin market tests.
- Targeted `rustfmt --check` and scoped `git diff --check`: passed.
- Database is main-session-owned isolated local MySQL; no production operations,
  commits, pushes or deployments.
- Validation is on MySQL 9.3.0, not production MySQL 8.4. The lock-wait regression
  additionally needs permission to read `performance_schema.data_lock_waits`
  and `data_locks`; audit injection needs CREATE/DROP TRIGGER privileges.

## Remaining Review Boundary

- No unresolved confirmed defect remains in this review's owned recharge
  changes. Existing Admin hierarchy creation/reassignment still locks a target
  before its ancestors; its interaction with ascending ancestor locks was
  flagged to main as an unverified cross-workflow stress-test gap, not claimed
  as a reproduced defect or silently changed here.
- Main owns frontend recovery behavior, final spec/progress synchronization and
  complete release gating. This backend pass does not certify the existing
  market task's failing frontend bundle budget.

## Main-Session Final Follow-Up

- Added Funding classifier `agent_recharge` case and an eighteenth recharge
  route test for the financial-idempotency audit. A valid credited reference
  does not increment orphans; mismatched amount and pending/rejected references
  each do. Scoped corrupt evidence is restored before asserting, and read-only
  audit leaves counts and balance unchanged.
- The follow-up agent became unavailable before returning its verification.
  Main explicitly reran the tests rather than assuming they passed.
- Final command with the same isolated DATABASE_URL and `--test-threads=1`:
  `cargo test --test agent_routes --test openapi_routes --test backend_architecture --test backend_documentation`
  passed **41/41, 12/12, 11/11, 1/1** respectively; real agent tests took 49.67s.
- `cargo test --lib wallet_ledger_classifier_covers_every_category --quiet`:
  **1/1**. Main's earlier full library test was **503/503**.
- Final `cargo clippy --all-targets --all-features -- -D warnings` passed
  (2m34s, including build-lock wait). Global fmt and diff checks passed.
- MySQL was then shut down using the task-specific socket; no live database
  or verification process was left running.
