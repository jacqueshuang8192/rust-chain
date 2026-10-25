# Compact scanner recovery error

## Goal
Fix CI result_large_err without disabling lint or changing scanner behavior.

## Requirements and decision
Box AppError inside ScanFailure; preserve retryable flags, original error variants/messages and all persistence semantics. Update every construction/match. Scope excludes toolchain upgrades, workflow changes and unrelated errors.

## Acceptance
- Production Clippy with warnings denied passes.
- Error layout stays below 128 bytes and preserves error/retry classification.
- Closest scanner tests and architecture checks pass; record any unavailable database/toolchain validation.

## Plan
Inspect constructors, make focused representation change, validate, document and commit locally. No new migration or API change.

## Verification
Local Rust/Clippy 1.92 production lint passed with explicit result_large_err denial. Scanner suite: 15 executed, 2 database fixtures skipped; architecture 11 passed. fmt/diff passed. CI Rust 1.99 not installed/tested. No DB integration rerun or migration. Local commit only; remote rerun pending.
