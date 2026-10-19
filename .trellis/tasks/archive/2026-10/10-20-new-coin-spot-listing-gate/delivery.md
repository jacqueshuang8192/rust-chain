# Delivery: new coin spot listing gate

## Result
Purchase-stage v2 projects retain active generation, ticker/chart read access and
live token purchase/sale, but cannot enter the spot catalog/favorites or place
ordinary spot orders. Real listing (listed status plus actual timestamp) exposes
the original pair. All stage checks are server-owned, including numeric pair IDs
and compatibility order insertion under a shared project lock.

Mobile clears stale rows when a successful catalog is empty, rejects malformed
catalogs/all-failed quotes, and restricts new-coin opportunities to listed projects.
PC live frames cannot create catalog entries. Existing price freshness merging,
legacy issuance and successful financial idempotent replay are preserved.

## Validation
- Dedicated local MySQL 9.3 database with the entire migration chain; standalone
  loopback Redis 16391. Final agent token lifecycle: 2/2 passed. Assertions include
  all prelisting stages with an active feed, missing actual evidence, timestamp
  alone, saved favorites, favorite mutation rejection, legacy policy compatibility,
  12 funded buy/sell x market/limit/stop-limit x symbol/ID denials, compatibility
  writer rejection, unchanged orders/ledger/wallets, live purchase ticker/quote,
  actual listing visibility, market buy/sell and exact retry, pair/engine/price
  continuity.
- Market routes: 13 executed passed, one Mongo-backed worker/history test skipped
  because MONGODB_URI is unavailable. K-line lifecycle request passes the listing
  boundary and reaches the existing missing-Mongo error; no claim of real history
  storage verification.
- Favorites auth 3/3; ordinary spot routes 65/65; wallet/spot repository 2/2;
  legacy new coins 11/11; architecture 11/11; documentation 1/1; OpenAPI 18/18.
- Mobile release:gate passed: 874/874 plus production/test types, PWA/Tauri builds,
  artifact checks, JS/CSS budgets and source/test governance.
- PC type-check and production build passed; all 123 tests passed.
- No layout change: client acceptance executes real API/store/presentation logic
  with replaced transports. No new browser or native-device run in this task.
- Strict `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check`,
  `git diff --check` and task context validation passed; no gate weakened.

## Rollout
Publish the API and both client bundles together. No new migration is required.
Existing 0152/0153+ issuance schema is assumed. Refresh the client catalog to remove
previously exposed purchase rows. This task does not deploy, modify production,
or push remotely. Prior local detail-UI commits remain on main.

## Local completion
Work committed as `fcb123c`. Dedicated test database and Redis process removed;
existing services and `.vite/` cache retained. No remote push or deployment.
