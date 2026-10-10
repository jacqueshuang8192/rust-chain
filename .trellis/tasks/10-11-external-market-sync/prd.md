# External Market Synchronization

## Goal
Identify the current cause of delayed/inconsistent external market updates and
repair reproducible defects without changing execution or settlement policy.

## Known Context
- Local mobile uses the configured backend proxy, not necessarily a local API.
- Earlier market-side-effect decoupling exists in commit 8d07a73. Deployment
  status must be established rather than assumed.
- Preserve unrelated chart, localization and provenance changes in the worktree.

## Plan
1. Compare anonymous Bitget and platform BTC/ETH ticker/depth/K-line timings;
   inspect proxy target and Git/deployment evidence.
2. Trace reception, cache admission, publication and frontend reconciliation.
3. Fix confirmed code defects with focused regression coverage.
4. Run relevant quality gates, record findings and commit the user-approved scope.

## Acceptance
- Distinguish measured source delay from inferred bottlenecks and unverified
  deployed version. No false claim of production recovery.
- Reproduce any changed behavior through executable tests.
- Do not drop financial trigger work, overwrite newer snapshots, use exposed
  credentials, modify production configuration or deploy without authorization.
- Record remaining operational work in PROGRESS.md and task delivery.

## Findings And Chosen Repair
- Remote main is 6530773, while the local async pipeline is 8d07a73; the latter
  is not on remote main. The production binary revision remains unverified.
- Two public samples show 465s then 556s median ticker age and growing delay
  across ticker/depth/K-line; REST is also stale. Bitget direct connections
  failed locally in both samples, so no new same-round exchange comparison is
  claimed. Platform HTTP Date agrees with the measuring clock.
- Add a connection-local source-progress watchdog after successful admission:
  >30s old push envelopes sustained >=10 monotonic seconds and >=3 frames end
  the cycle. Existing fallback/backoff performs resubscription. No order-age
  checks, snapshot timestamp changes or deletion of durable work.

## Delivery Status
- Diagnosis, implementation, seven focused recovery tests, 568 backend
  regression tests and strict all-target Clippy are complete.
- User approved committing all current changes, not only this task. Previous
  frontend changes remain intact; combined Mobile831/831, PC119/119, type checks
  and builds passed. Exclude only generated cache and ignored local files.
- No push or production deployment is authorized; see `delivery.md` for
  operational prerequisites and verification limitations.
