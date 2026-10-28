# Base scanner stream isolation delivery

## Result

Base ETH and ERC20 now have separate persistent realtime/history/recovery/lease
state, separate scheduling loops, scoped RPC capability cooldowns and asset-level
admission. Two-way failure and actual wallet credit/replay are tested. Native
supports only direct transfers, unchanged. All ERC20 contracts share token stream.

## Verification

- Real isolated MySQL 9.3 at 127.0.0.1:13369, mock RPC: new migration preservation
  and end-to-end stream scenarios passed, including a final 2/2 rerun after review.
- 62 infrastructure deposit tests actually executed: initial 61/62, old Base
  snapshot fixture updated to stream storage, isolated failing-test rerun 1/1.
  No skipped database test in this run. Each existing/new case passed at least once.
- Scenarios cover ETH failure/USDC credit and admission, reverse failure, wallet
  ledger exactly-once replay, token-only and legacy both-stream recovery, shared
  HTTP 429, revision save races, repeated token scheduling while native lease is
  held, and expired lease refusing financial evidence. Migration run twice clean.
- Rust architecture 11/11, OpenAPI 20/20, production Clippy, all-target check,
  formatting and whitespace checks passed. A separately attempted domain filter
  selected zero tests and is not counted as validation.
- Web 1215/1215; focused 52/52 after narrow confirmation fix; typecheck, lint,
  production-policy 15/15, coverage 23/23 and build passed.
- Web raw JS remains above existing budget: 2,787,579 / 2,600,000 bytes (previous
  local baseline 2,785,289; this feature +2,290). No budget threshold relaxed.
- Ego used actual components with local fixture API at 127.0.0.1:5199. Verified
  ETH error / USDC available, token-only queued recovery with native unaffected,
  1728/1280/390/320 layouts. Found and fixed 448px confirmation clipping at 320px;
  final modal 288px, drawer 272px, no document overflow. Screenshots inspected at
  /tmp/base-stream-{desktop,recovery-320,confirm-narrow-fixed,token-390}.png.
- Browser fixture is not a production login/real backend HTTP session. No actual
  blockchain payment, production RPC throughput, external configuration or
  deployment was performed. Financial verification used the production application,
  persistence and scanner code with a real local database and simulated RPC.

## Release and rollback

Stop old deposit workers, run 0165, deploy backend and Admin together, restart.
Migration clones exact checkpoints/history, invalidates leases/freshness and
increments network revision. Old/new workers must not run together. Rollback
requires reviewing both lanes and an audited checkpoint repair; frozen legacy
Base cursor is not current progress. Shared provider outages/HTTP quotas can still
stop both lanes. No provider purchase or credential changes are included.

## Workspace

Earlier agent-management edits, its progress entries, .vite and unrelated task
folders remain outside this delivery. Test MySQL and Vite services and browser
fixture session/task space are cleaned up. No credentials are committed.
