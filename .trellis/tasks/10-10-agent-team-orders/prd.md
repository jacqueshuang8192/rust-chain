# Agent Team Orders

## Goal

Add read-only, filterable seconds, margin-contract, earn and spot order pages
to the agent portal, covering users owned by the authenticated agent and all
descendant agents without requiring per-user navigation.

## Requirements

- Four independent navigation entries with paginated server-side queries.
- Default scope is the authenticated agent's subtree. Narrow by own users or
  a selected descendant agent, user ID/email, order ID, status, product/pair and
  creation time. Filters must never broaden token-derived scope.
- "Contract orders" means the existing margin order records, including pending
  limit orders; retain existing status interpretation.
- Show user and owning-agent identity alongside the business order snapshot.
- Include all seconds states by default, including opened and refunded.
- Earn rows read persisted subscriptions only, without redemption/accrual.
- Preserve Decimal strings, millisecond timestamps, filter-consistent totals,
  bounded pagination, and late-response/session guards.
- No trading writes, hierarchy changes, production data changes or deployment.

## Plan

1. Inspect subtree authorization, existing row projections and order semantics.
2. Add validated team-order read endpoints and real-database isolation tests.
3. Add shared portal list/filter presentation, routes and response validation.
4. Run backend/frontend gates and representative browser checks.
5. Update specs/progress, review and propose a task-only commit for confirmation.

## Acceptance Criteria

- [x] Root/child/grandchild visibility includes only their own subtree.
- [x] All four order kinds return scoped rows and matching filtered totals.
- [x] Own/subagent/user/status/product/time filters and pagination work.
- [x] Missing/invalid filters fail safely; sibling/unrelated IDs leak no rows.
- [x] Four Chinese navigation entries and responsive lists are usable.
- [x] Loading, empty/error, stale response and session changes are covered.
- [x] Financial reads produce no wallet/order/ledger/event mutations.
- [x] Closest tests, full gates, browser evidence and limits are recorded.

## Existing Work

The earlier realtime/console task and unrelated chart/theme edits are dirty.
Preserve them and exclude them from this task's commit. Existing Web bundle
budget was already exceeded; never raise its threshold to claim validation.
