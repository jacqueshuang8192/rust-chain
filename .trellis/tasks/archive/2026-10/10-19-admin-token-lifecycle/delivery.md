# Admin v2 token lifecycle delivery

## Result

Administrator project creation now uses the v2 issuance contract: fixed subscription price, realtime-purchase seed, listing reference price, independent scheduled/manual phase nodes, IANA timezone, immutable lock rule and exit fee. It selects existing active base/quote assets and atomically creates the preheat project, fixed disabled market pair/generator, zero-balance wallets, audit evidence and recoverable creation receipt. Agent application review and legacy creation APIs retain their existing behavior.

Project management exposes issuer identity, actual purchase time, next automatic node and retry state. V2 contracts are read-only. V2 market drafts use the approved pair and seed, and cannot publish before actual purchase; listing continues the same market history. Existing admin per-order allocation remains the audited fallback; agents retain buyer-subtree manual batch allocation with adjustable ratios.

Three permissioned read-only views expose original holdings/lock rules, executed buybacks with gross/fee/net amounts, and actual operation actors. They distinguish buyer ownership from issuer, support server-side filters/paging and exact loaded-page CSV exports. Historical locks remain available separately.

## API and migration

- `POST /admin/api/v1/new-coins/issuance`: `new_coin.projects.write`; full-intent idempotency scoped to active administrator, audit rollback and deterministic duplicate-project conflict.
- `GET /admin/api/v1/new-coins/holdings`: `new_coin.locks.read`.
- `GET /admin/api/v1/new-coins/buybacks`: `new_coin.unlocks.read`; only executed sell records.
- `GET /admin/api/v1/new-coins/operations`: `audit.logs.read`.
- Migration `0154_admin_token_issuance_receipts.sql` adds the immutable receipt table. Apply after 0152/0153 before serving the new creation endpoint. It was applied only to the disposable test database during this task; do not edit the migration in place.
- `/openapi.json` and `/api/openapi.json` are document aliases, not two API route prefixes.
- Existing contracts are not reinterpreted; no balance minting, automatic allocation/sale, production migration, deployment or treasury funding was performed.

## Verification

| Check | Result |
| --- | --- |
| Admin v2 actual MySQL tests | 3/3; canonical and concurrent replay, conflict, rollback, zero balances, ownership/count/paging, original rules, sell-only records, fixed market pair/seed and actual-purchase guard |
| Legacy admin new-coin regression | 19/19 |
| Agent lifecycle with dedicated Redis | 2/2; approval, scoped batch allocation/refunds and realtime trade/buyback regression |
| Legacy user new-coin routes | 11/11 |
| Issuance policy | 3/3 |
| Architecture / document / OpenAPI | 11/11, 1/1, 18/18 |
| Strict Clippy | `cargo clippy --offline --all-targets --all-features -- -D warnings` passed |
| Rust formatting | `cargo fmt --all -- --check` passed |
| Final Web full suite | `npm --prefix web test -- --maxWorkers=2`: 117 files, 1124/1124 |
| Web type / lint / build | Passed after final detail-label changes |
| Web production policy / coverage | 15/15; 23/23 with configured thresholds passed |
| Browser | Creation/retry, server paging/CSV/error/empty, records/details, contract/market guards, read permissions; no document overflow at tested 1728/1280/390/320 widths |
| Diff / task context | `git diff --check`; task context validation passed |

The first final full Web run had one timeout in the existing financial reconciliation follow-up retry test (1123 passed). The unchanged test passed standalone (16/16 in its file), and the full suite subsequently passed with two workers. No test was skipped or weakened.

The JavaScript bundle budget remains failing: **2,712,495 > 2,600,000 raw bytes**, compared with task-entry **2,699,820 > 2,600,000**. The build itself passes. No threshold was raised; the full release gate must not be described as green.

## Browser evidence

See `research/browser-qa.md`. Local fixture scripts, receipts and inspected screenshots are in `/tmp/admin-token-qa-20261019/`. Real form handlers were exercised against in-memory API fixtures; these browser checks do not assert production connectivity. Actual monetary transactions and authorization scopes were verified separately with isolated database tests.

## Operational boundary and cleanup

Only MySQL 9.3 was exercised locally; MySQL 8.4, physical phones and installed PWA were not tested in this administrator-only task. Production rollout still requires the existing v2 prerequisites and separately audited platform buyback funding.

Task database `codex_admin_tokens_20261019` was dropped, dedicated Redis on `127.0.0.1:16389` stopped, and Ego task space 23 finished. Existing preview services on 3030/5186/5190 and unrelated `.vite/` cache were preserved.

## Commit status

Implementation and verification are complete. User explicitly authorized commit and push on 2026-10-19. The exact approved 54-path file list is in `commit-plan.md`; work commit, current-task archive, journal and normal push are now being executed. No deployment or unrelated historical task is included.

Work commit created: `9dde7acfa626acd1e103139f346f152635a552b6`. Current-task archive and push receipts follow in the session journal and progress log.
