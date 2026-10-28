# Base obsolete trace cooldown repair

## Verified result

The user confirmed Base Admin channel status. It reads a persisted provider error.
The old exact trace error survives 0165 and excludes the channel before any new
standard RPC can run. A real isolated MySQL regression reproduced zero requests
and unchanged cursors, then failed the expected retirement assertion before 0166.

Migration 0166 clears only exact legacy Base provider trace failure/cooldown/count
and stale provider success. It preserves key 429/401 cooldowns, other methods,
connection errors, Ethereum trace errors, disabled providers, pacing, network
revision, history/queued recovery/leases and all checkpoint state. The production
worker then advances both streams from 100 to 101 without trace calls. This is
not a production DB inspection or proof of production recovery.

During testing, adding the migration did not recompile the existing SQLx macro
output. Added root build.rs with cargo:rerun-if-changed=migrations, copied into the
Docker builder; Cargo metadata confirms directory tracking. SQLx official macro
documentation describes the same incremental-build requirement:
https://docs.rs/sqlx/0.8.6/sqlx/macro.migrate.html

## Validation

- New real-MySQL regression: red before repair; green after repair/build tracking.
- Three Base migration/stream/upgrade DB tests passed together, no skipped tests;
  fresh migration and repeated SQLx runs clean.
- Base ordinary-RPC regressions: 41/41.
- Architecture: 11/11; Docker image contracts: 10/10.
- cargo clippy --lib --bins -- -D warnings, cargo check --all-targets,
  cargo fmt --all -- --check passed.
- New SQL/build file whitespace/conflict checks and tracked diff checks passed.
- One initial --exact short-name command matched zero tests and is not counted.
- Prior package cache also caused an intermediate stale-migrator failure;
  build.rs corrected that before the final three-test run.
- No frontend contract/visual change for Base, no production credentials or RPC
  traffic. Full Docker image publication and CI were not verified.

## Release

Stop old deposit workers, build/pull the updated backend and migration runner,
apply all pending migrations including 0166, then restart the current worker.
The migration does not grant readiness; verify both streams actually resume and
catch up. Do not roll back to a trace-based Base worker or clear unrelated errors.

## Retrospective

Root category: change-propagation failure and upgrade test gap. Changing the RPC
call did not retire a durable capability failure. Earlier tests began with healthy
provider rows. Future removal of provider capabilities requires an upgrade fixture
with the old failure/cooldown, plus checks that real quota and history remain.
The backend timed-deposit and container-delivery specs now capture this contract.
No src/templates/markdown/spec mirror exists in this application repository.
