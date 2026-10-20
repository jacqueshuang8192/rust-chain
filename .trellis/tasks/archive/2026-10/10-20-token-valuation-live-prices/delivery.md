# Live token valuation repair

## Result

Fixed the shared backend price reader used by holdings, wallets, quotes and
direct purchases. Real default tickers publish generated/strategy provenance;
the previous consumer compared that public provider with the private default
engine identity and rejected the price. The reader now checks the matching
price/time checkpoint and current runtime identity before applying the existing
freshness, pause, lease and version checks. Redis remains the only price source.

Locked purchase holdings now provide current value and unrealized profit;
existing frontend code calculates the return rate and includes the booked token
balance in asset rows and portfolio totals. Lock eligibility and listing-only
spot permissions remain unchanged. No frontend production code or database
migration is needed for this repair. Deploy the updated backend for it to take
effect online; no remote push or deployment was performed in this task.

## Verification

Only replacing the old fixture with the real producer DTO reproduced the
original failure before the code fix (`/tmp/token-valuation-before.log`).

Final verification passed:

- `DATABASE_URL=mysql://root@127.0.0.1/codex_token_valuation_20261020 TOKEN_LIFECYCLE_REDIS_URL=redis://127.0.0.1:16393 TOKEN_VALUATION_EVIDENCE_PATH=/tmp/token-valuation-api-evidence.json cargo test --test agent_routes token_lifecycle -- --test-threads=1`: 2/2. Uses real default/manual ingestion, isolated MySQL/Redis and authenticated holdings/wallet APIs; covers price/source/version/freshness failures and recovery, direct purchases, and the existing full money lifecycle.
- `cargo test --test backend_architecture --test backend_documentation --test new_coin_routes --test openapi_routes --test wallet_routes --test wallet_spot_sqlx_repositories -- --test-threads=1` with the isolated database: 11 + 1 + 11 + 18 + 15 + 2 passed; one existing manual browser fixture ignored.
- `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`: passed.
- `npm --prefix mobile run type-check:tests` and `npm --prefix mobile test`: passed, 885/885.
- `node --test --experimental-strip-types pc/tests/*.test.ts`: 125/125.
- `git diff --check` and task context validation: passed.

Logs: `/tmp/token-valuation-{final,regression,clippy-final,mobile-tests,pc-tests}.log`.

## Browser evidence and boundaries

Ego space 31 replayed actual test API responses through a read-only local
adapter into the existing Mobile/PC pages. Observation times were shifted to the
replay clock. This validates rendering and calculations against backend output;
it does not claim a live production-server or Mongo history test.

- Locked quantity 2500, cost 500 and live quote 0.4: detail value 1000,
  unrealized profit 500 and return 100%; sell remained disabled.
- At quote-to-USDT rate 2: quote balance 4500 is worth 9000, locked token balance
  is worth 2000, and Mobile/PC total assets are 11000 USDT.
- Null price displays unavailable; restoring a valid price automatically restores
  the Mobile total without manual refresh. The token stays absent from spot.
- Final screenshots were inspected: `/tmp/token-live-valuation-ui-20261020/detail.png`,
  `assets.png` and `pc-assets.png`; fixture writes were zero.

No frontend release build was repeated because production frontend files did
not change. Mongo historical candles and physical-device installation were not
tested. Space 31, preview ports 5187/5188, dedicated Redis 16393 and the isolated
test database were cleaned up. Existing services and the unrelated `.vite/`
cache were preserved.
