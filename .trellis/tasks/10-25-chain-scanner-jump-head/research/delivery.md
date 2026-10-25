# Scanner recovery delivery

## Behavior

- Explicit network-write action selects dual progress or manual skip; no automatic restart jump.
- Persisted recovery request, actor/reason audit, confirmed-head validation and revision/lease fencing.
- Live scanning has priority; retained history uses the same durable provider/key gates, separate progress/error and fixed target anchor.
- Explicit skip discards the skipped interval and existing history. Existing orders/receipts/ledger remain.
- Backfill validates target and full batch before settlement, preserves receipt idempotency and blocks admission on non-transport errors.
- Recovery endpoint returns 202 with JSON `{queued:true}` to match the Admin client. A real browser check found and eliminated the empty-body contract error.

## Verification

- Isolated MySQL / simulated RPC: 22 timed-deposit tests passed, including history settlement, replay ledger count, repeated dual extension, skip acknowledgement, transient/fatal history errors, node failure and revision fencing. Original provider failover/DB tests ran with real isolated MySQL.
- Admin targeted tests: 42 passed. Full Web: 1201 passed (TZ=Asia/Hong_Kong); typecheck/lint, production-policy 15, coverage 23 and production build passed.
- Web bundle gate: existing failure persists at 2,778,572 / 2,600,000 raw JS bytes; +4,354 vs baseline 2,774,218. Budget was not relaxed.
- Final actual HTTP permission/revision/acknowledgement and JSON 202 response test passed; final architecture 11 / OpenAPI 20, production Clippy, all-target check, fmt and diff checks passed. After temporary fixture removal, wallet_routes compilation passed.
- Final layout adjustment: targeted Admin 42 tests, typecheck/lint and build passed; the last CSS-only title flex fix was verified through a rebuilt bundle and inspected screenshots.
- Ego Browser space 11 used actual Axum routes and isolated MySQL plus a local Tron mock. Dual recovery reached 9999 while history remained 740/9999. Skip then cleared history. DB confirmed two requested and two applied audits, live height 9999 and no pending request. Mandatory acknowledgement and mode/reason confirmation were exercised.
- Inspected `/tmp/chain-recovery-dual-1728.png`, `/tmp/chain-recovery-skip-390.png`, `/tmp/chain-recovery-skip-320.png`. Narrow title wrapping keeps the close button within the viewport; no horizontal overflow. Raw CDP screenshots preserve the test viewport because normal Ego Page actions restore its default viewport. No real node or real transfer.

## Release

Apply 0163 after 0162 and release backend/Admin together. User authorized commit and push on 2026-10-25; this delivery includes origin/main publication. No production cursor mutation or deployment performed. Real provider limits, chain access and on-chain transfers remain unverified.

## Scope

Unrelated untracked task folders and `.vite/` remain untouched. Temporary browser fixture/session removed, TaskSpace finished and isolated API/Vite/MySQL stopped. No live credentials were used.
