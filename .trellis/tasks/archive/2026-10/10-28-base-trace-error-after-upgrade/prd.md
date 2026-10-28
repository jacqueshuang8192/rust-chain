# Base trace error after upgrade

## Goal

Diagnose and repair the upgrade path that can keep Base unavailable with the
obsolete `debug_traceBlockByHash` error after switching to standard RPC.

## Confirmed context

- The user updated the application and still sees the exact RPC capability error.
- The user confirmed the affected network is Base.
- The user confirmed the message is in Admin deposit configuration / channel status.
- Both Git remotes have `ff0c5dd`; this does not prove the production process version.
- Current Base code returns from the direct-transfer branch before the trace call.
- Migration 0165 preserves legacy errors and leaves shared provider cooldowns intact.
- Provider selection excludes a persisted shared cooldown before any RPC request.

## Requirements

- Reproduce the upgrade failure using an isolated database and mock RPC.
- Repair only demonstrated obsolete Base trace failure state; preserve real quota,
  authentication, network, integrity and unrelated-method failures.
- Preserve all checkpoints, historical work, pending recovery and financial records.
- Do not edit applied migrations or introduce calls to debug/trace for Base.
- Preserve unrelated working changes and never record RPC credentials.

## Open questions

- What production process version and migration set are active? No access supplied.

## Acceptance criteria

- [x] Demonstrate the current upgrade defect before changing production behavior.
- [x] Regression proves Base can resume with standard RPC after the targeted repair.
- [x] Unrelated Base cooldowns and Ethereum trace failures remain intact.
- [x] Checkpoints and accounting stay unchanged by the repair; no synthetic freshness.
- [x] Closest real database tests, formatting and applicable static checks pass.
- [x] Update operational guidance and PROGRESS with verified scope and limitations.

## Plan

1. Trace error persistence, migration and selection; capture evidence.
2. Implement the smallest proven repair and regression tests.
3. Verify real MySQL upgrade/resume, document rollout and deliver within existing authorization.

## Out of scope

- Ethereum scanning behavior, unrelated admin UI, provider credentials or subscriptions.
- Production deployment or database writes without a supplied target and authorization.

## Research

See `research/diagnosis.md` for evidence and the distinction between a reproducible
upgrade defect and the still-unverified production root cause.
