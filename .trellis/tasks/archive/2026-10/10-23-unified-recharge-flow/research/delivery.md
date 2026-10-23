# Unified Recharge Delivery — 2026-10-23

## Delivered behavior

- Two recharge workspaces: GMPay quick recharge (configuration/orders) and
  on-chain recharge (network/node/assets/addresses, orders/receipt review, ledger).
- Direct per-network receiving addresses, batch size 1..500, extensible beyond
  ten. Full-batch validation, canonical duplicate skipping, audit atomicity and
  preservation of disabled rows. Address-only staff use a separate read endpoint.
- Migration 0160 gives shared addresses independent IDs and preserves existing
  timed-order references. No writes to the retired permanent inventory.
- Retired permanent allocation, old Admin inventory CRUD and observation APIs;
  old gateway deposits dead-letter instead of crediting a historical owner.
  Withdrawal directory/gateway/accounting behavior remains covered by regression.
- Exact coin quantities, 30-minute combination reservations, shared addresses for
  differing amounts, same-amount capacity conflicts, no fractional suffixes, and
  audited review of ambiguous late/reused combinations remain authoritative.
- Admission uses current platform policy plus explicit chain mappings, enabled
  addresses and scanner readiness. Disabling new orders does not stop configured
  scanners or settlement. Old in-flight revisions cannot commit.
- Five supported networks remain Ethereum, Base, Tron, Bitcoin and Solana, with
  the adapter limitations in `docs/superpowers/timed-chain-deposits.md`.

## Executed validation

All database runs used the task-only local MySQL at port 13367 with database
`timed_deposits_test`; no production database was touched.

| Command / check | Result |
|---|---|
| `TIMED_DEPOSIT_TEST_DATABASE_URL=… cargo test --lib timed_deposit -- --nocapture` | 13/13, including real MySQL migration twice, 10-address concurrency/capacity, repeated imports, rollback, disabled-admission checkpoint/credit, ambiguity, reversal and stale revision protection |
| `DATABASE_URL=… cargo test --test wallet_routes timed_deposits` | 1/1 actual HTTP test; exact amounts, ownership, role separation and retired routes 404 |
| `DATABASE_URL=… cargo test --test wallet_chain_worker --test wallet_routes` | 4/4 gateway and 13 wallet tests passed; one manual browser fixture intentionally ignored; withdrawal freeze/quote/approval/confirmation/limits/cooling covered |
| `cargo test --test openapi_routes --test backend_architecture` | 19 OpenAPI + 11 architecture passed; optional unrelated database fixtures without their own variables are not claimed as live DB coverage |
| `cargo check --all-targets --all-features` | Passed without warnings after removing retired test helper |
| `cargo clippy --lib --bins --all-features -- -D warnings` | Passed |
| `cargo test --test backend_documentation` | 1/1; audited entry list moved from retired observer to scanner settlement and manual review |
| `cargo fmt --all -- --check` | Passed |
| `npm --prefix web test` | 122 files / 1,142 tests passed |
| `npm --prefix web test -- src/admin/deposits` after final layout changes | 3 files / 9 tests passed |
| `npm --prefix web run lint` and `run build` | Passed; build includes TypeScript checking |
| Web production policy / coverage | 15/15 and 23/23; configured coverage thresholds passed |
| `npm --prefix web run budget` | Existing total JS budget still fails: 2,731,976 > 2,600,000 bytes; no threshold changed |
| Mobile production/test type checks | Passed after removing unused permanent-address APIs/DTOs |
| Mobile timed-deposit/wallet behavior tests | 11/11 passed |
| Mobile `build:pwa`, `check:artifacts:pwa`, `check:bundle` | Passed; this task did not change native startup/dependencies or rerun native packaging |
| PC type check and build | Passed |
| PC security/recharge/withdrawal adapter contracts | 2/2 targeted tests passed |
| Task JSONL validation and `git diff --check` | Passed |

The Web budget is 11,262 bytes lower than the previous delivery record
(2,743,238 bytes), but remains a release gate failure. Previous unrelated full
Clippy test warnings (`seconds_contract` cmp_owned) and the new-coin numeric DTO
guard limitation are documented in the earlier task; this task used production
Clippy and did not rerun or modify those unrelated gates. No claim of an entirely
green repository-wide release pipeline is made.

## Browser evidence

Real Admin layout, routes and components ran against local fixture responses in
Ego TaskSpace 5. Verified the two navigation entries, independent tab permissions,
two-address import confirmation/payload/refresh, address-only roles without node
settings, Base import retaining Base after refresh, and the empty review screen.
Desktop, 390px and 320px were inspected. Final 320px cards remain within the
content area with no clipped controls or horizontal card overflow; a grid
min-content issue discovered in screenshots was fixed with `minmax(0, 1fr)`.

Screenshots were inspected at `/tmp/recharge-address-only.png`,
`/tmp/recharge-final-390.png`, and `/tmp/recharge-fixed-320.png`.
These are ephemeral local artifacts, not evidence of real chain transfers.

Temporary browser fixture files were removed. TaskSpace 5 was finished once;
Vite port 3036 and the task-only MySQL were stopped, retaining diagnostic logs/data.
Other task directories and `.vite/` were not cleaned or included.

## Remaining deployment work

Apply the new migration to the intended environment, configure real receiving
addresses and node credentials, then perform controlled live transfers on each
network. Neither this task nor its browser/RPC fixtures certifies mainnet receipt,
provider uptime or actual payment latency. No production deploy or remote push
was performed as part of this redesign.
