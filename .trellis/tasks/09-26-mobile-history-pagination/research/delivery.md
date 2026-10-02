# Pagination Delivery

## Implemented

- Added bounded `limit + offset + has_more` contracts for user history endpoints covering withdrawals, spot orders, margin positions, convert orders, earn subscriptions, loan orders, prediction orders, and quick recharge orders.
- Added stable server-side ordering and fetch-one-extra-row detection for `has_more`; wallet ledger behavior remains unchanged.
- Added `mobile/src/core/historyPagination.ts` with request validation, page mapping, deduplication, stale-session protection, append retry state, and a bounded compatibility helper that drains all pages for existing callers.
- Added page adapters to the mobile wallet, trading, convert, earn, loan, and prediction APIs.
- Connected the mobile withdrawal history view to load-more and append-retry controls. Existing product/order pages continue to receive complete history through the compatibility adapters.

## Verification

- `npm --prefix mobile run type-check`
- `npm --prefix mobile run type-check:tests`
- `node --test --experimental-strip-types mobile/tests/history-pagination.test.ts mobile/tests/wallet-secondary-views.test.ts mobile/tests/pencil-wallet-flow-parity.test.ts`
- `cargo check --all-targets --all-features`
- `cargo test --lib --quiet` (503 passed)
- `cargo fmt --all -- --check`
- `git diff --check`

## Boundaries

- No order matching, settlement, wallet accounting, or historical data was changed.
- No production, native device, or real-funds pagination test was performed.
