# Realtime Settlement And Console Quality

## Goal

Address the five requested issues: seconds-order private WebSocket updates,
settlement latency, readable Chinese backend logs, agent control/table sizing,
and Chinese, well-organized administrator details.

## Requirements

- Reuse authenticated private streams. Reconcile on initial load, events and
  reconnect; retain only bounded recovery for missed events/disconnection.
- Preserve Redis processing-time pricing, positive-price validation, atomic
  funds/evidence writes, idempotency and historical manual-review/refund rules.
- Investigate scheduling, queue fairness, locks and frontend refresh latency.
  Do not claim production diagnosis from local code alone.
- Improve application-owned log messages and formatting without changing API
  errors, stable machine identifiers, or exposing tokens/payloads.
- Improve shared agent form/filter/table layouts and shared administrator
  detail rendering; preserve permissions, action behavior and exact values.
- Preserve all existing unrelated working-tree changes.

## Acceptance Criteria

- [x] Mobile and PC seconds orders respond to private events and reconnect.
- [x] Healthy connections no longer depend on frequent order polling.
- [x] Settlement backlog and failed-order fairness regressions pass with real
  isolated MySQL/Redis; no duplicate payment/event on replay.
- [x] Logs use a compact readable format, Chinese application messages and
  sensible levels; idle high-frequency loops do not flood INFO.
- [x] Agent controls/tables and administrator details are responsive; newly
  exposed known keys have Chinese labels and unknown/raw evidence is retained.
- [x] Focused tests, type checks, lint and closest full gates are recorded.
- [x] Browser checks cover representative populated/empty/detail screens.
- [x] Each delivered slice is recorded in docs/superpowers/PROGRESS.md.

## Verification Boundaries

Local implementation checks are recorded in `research/delivery.md`. This is not
release approval: Web raw-JavaScript budget remains over its unchanged limit.
The first parallel database route run encountered MySQL 1213; serial fixtures
passed, including their own concurrent-open case. No production latency or
MySQL 8.4 run is claimed. UI browser evidence uses read-only local mocks.
Changes remain uncommitted and undeployed; the task stays open for release
follow-up instead of archiving through scripts that auto-commit.

## Plan

1. Inspect existing stream, scheduler and presentation contracts.
2. Implement and test seconds realtime/settlement corrections.
3. Implement and test log formatting/localization.
4. Implement and test shared agent/admin presentation corrections.
5. Verify integration, record limits and present changes without committing.

## Out Of Scope

Production deployment, live financial actions, schema rewrites, payout-policy
changes, replacing the existing design system, unrelated chart/theme work,
automatic commits or pushes.

## Initial Evidence

- Prior commit 140b55c already selects Redis prices without a freshness gate.
- Scheduler processes one bounded batch sequentially; explicit environment
  overrides may retain a five-second interval.
- Private broadcast infrastructure already exists; delivery and consumption
  need verification before replacing anything.
