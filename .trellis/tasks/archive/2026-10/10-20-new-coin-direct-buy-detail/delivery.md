# Delivery and verification

## Result

- Added one-request live market purchases on Mobile and PC. The server chooses
  the valid engine price and commits the receipt, debit, holding and audit in one
  transaction. Canonical idempotent retries return the original trade even after
  a price or phase change. Unknown client outcomes retain the original intent.
- Wallet valuation no longer relies on spot discovery. Subscription/distribution
  use the fixed subscription price; purchase/listed stages use valid engine data,
  converted from the actual quote asset to USDT. Missing or stale prices remain
  unknown. Available, frozen and locked booked balances are counted once; pending
  allocation quantities are excluded to avoid double-counting reserved funds.
- Mobile detail prioritizes the primary action, with compact identity/supply,
  readable stages, a shorter chart and clearer holding value/return. Native
  disclosure preserves purchase/unlock details. Existing themes, locales, legacy
  flow, hidden notices and automatic refresh remain supported.
- Listing remains required for spot discovery and trading. No database migration
  was added. Backend, Mobile and PC must be released together.

## Verification

| Check | Result |
| --- | --- |
| Isolated MySQL full migrations and `cargo test --test agent_routes token_lifecycle -- --test-threads=1` with dedicated Redis | 2/2 passed |
| Backend architecture / documentation / old new-coin / OpenAPI | 11/11, 1/1, 11/11, 18/18 passed |
| Wallet routes / wallet-spot SQLx repositories | 15 passed, 1 existing manual browser fixture ignored / 2 passed |
| Rust formatting and strict Clippy, all targets | Passed |
| `npm --prefix mobile run release:gate` | 885/885 tests; type, PWA/Tauri build/artifact, source/test quality and bundle gates passed |
| PC type-check, build and test suite | Passed; 125/125 tests |
| `git diff --check` and task context validation | Passed |

Financial regression coverage includes concurrent same-key execution, original
receipt replay, changed-intent conflicts, old quote-key conflicts, precision,
supply and balance limits, client price rejection, stale/future prices, paused
engines, final-audit rollback without residual quotes, balanced ledger entries,
cross-asset USDT conversion, and the existing pre-listing spot restrictions.
Frontend tests execute the real Mobile and PC trading components, including
lost responses, remounts, legacy quote recovery, storage failures and identity
changes.

The unchanged Mobile budgets passed at PWA JS 1499.8 KiB, Tauri JS 1496.3 KiB and
CSS 655.9 KiB. No limits or build compression settings were relaxed.

## Browser acceptance

Ego space 30 used local Mobile 5187 and PC 5188 with an in-memory Axios fixture.
Financial write requests were intercepted; no production trades were placed.

- Twelve fresh-module layout combinations (320/390/448 px, Chinese/English,
  light/dark) had no horizontal overflow or clipped stages. Purchase controls
  remained visible on the first screen; inputs measured 52 px high.
- A single action sent `market-purchases`, with zero buy-quote requests.
  A lost response retried the same key and returned the original execution after
  the live price changed, without duplicating the trade.
- With an empty spot catalog, 2500 locked KHB valued at 500 USDT during
  subscription (price 0.2), then 1000 USDT during purchase (price 0.4). Including
  1100 USDT available/frozen funds, total assets automatically changed from 1600
  to 2100 on both Mobile and PC.
- Subscription prioritizes its form; purchase shows direct buy and live chart;
  listed projects show the spot entry without accepting new issuance purchases.
  Holding disclosure was exercised using keyboard focus and Enter.
- Final screenshots were inspected after a hard reload to avoid stale Vite HMR
  templates. The existing chart WebSocket continued to update.

Local evidence: `/tmp/token-direct-ui-20261020/final-layouts.json`,
`final-fresh-390.png`, `final-en-dark-320.png`, `final-pc-assets.png`,
`assets-purchase.png` and `final-holdings-390.png` in that directory.
Logs: `/tmp/token-direct-{financial,regression,clippy-final,mobile-gate,pc-type-final,pc-build-final,pc-tests-final}.log`.

## Scope and cleanup

- No production deployment, remote push or production database changes.
- Mongo historical-candle reads and physical-device installation were not tested;
  neither implementation was changed by this task.
- Closed Ego space 30, stopped only the task's 5187/5188 preview servers, shut down
  Redis 16392, and dropped only the isolated `codex_token_direct_20261020` database.
- Preserved existing services and the pre-existing untracked `.vite/` directory.
- Ego reported an available update; no browser upgrade was performed.
- Implementation saved locally as `bb230723c592d173ffae19c532f9a3c5e41d13ff`. Archive and
  journal are separate bookkeeping commits. No remote push is part of this task.
