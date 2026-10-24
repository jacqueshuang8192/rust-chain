# Local delivery: chain deposits

## Result
- Mobile-only redesign, independent history/detail routes, explicit cancellation.
- Admin receipt lookup/confirmation repaired; cancelled-state compatibility only in PC/agent.
- Migration 0161 must ship before the API/mobile release. Original windows and cancellation timestamps remain immutable.

## Evidence
- Isolated MySQL cancellation/accounting and actual HTTP owner/auth tests passed; agent funding DB test passed.
- Architecture 11/11, OpenAPI 20/20, production Clippy passed. Extra all-targets Clippy is blocked by the pre-existing seconds-contract test cmp_owned warning at line 316.
- Mobile full release gate: 907 tests passed, production/test type checks, PWA/Tauri, artifacts, bundles and source/test governance passed. Final focused suite after adding unmount coverage: 12/12. PC type check passed.
- Web full suite 1193/1193; final related suite 16/16; type/lint/policy/coverage/build passed. Web existing raw budget fails: 2,763,960 vs 2,600,000 bytes, +3,057 this task.
- PWA raw JS/CSS 1514.2/658.1 KiB; Tauri JS1510.7 KiB. Narrow feature budgets 1516/659 KiB after deduplication; entry/gzip/count budgets unchanged.
- Ego real API and isolated DB: create/cancel, QR removal, history/detail, manual attribution via complete CD number and reason, one ledger entry, retained cancellation timestamp, blocked additional payment. Screens inspected at 320/390/448 and Admin1728/320; light/dark/English, exact huge decimal and focus return verified. No live-chain transfer was sent.
- Screenshot correction: themed Teleport and a single-column payment block even within a desktop phone canvas.

## Cleanup
The temporary server fixture was removed, the Ego task space finished, the two Vite servers and isolated HTTP/MySQL services stopped, and local test tokens deleted. The user's main MySQL instance was not touched.

## Commit plan (authorized by user)
User authorized commit and push on 2026-10-25. One work commit: `feat(wallet): improve mobile deposits and repair receipt review`, followed by task archive and session records; push to `origin/main`. Deployment is outside this delivery.

Files:
- `.trellis/spec/admin/ui-system.md`
- `.trellis/spec/backend/timed-deposits.md`
- `.trellis/spec/mobile/backend-integration.md`
- `docs/superpowers/PROGRESS.md`
- `docs/superpowers/timed-chain-deposits.md`
- `migrations/0161_timed_deposit_cancellation.sql`
- `mobile/scripts/check-bundle-budget.mjs`
- `mobile/src/api/timedDeposits.ts`
- `mobile/src/components/DepositOrderHistory.vue`
- `mobile/src/components/TimedDepositPanel.vue`
- `mobile/src/core/navigation/accessibility.ts`
- `mobile/src/core/timedDeposit.ts`
- `mobile/src/i18n/messages/en.ts`
- `mobile/src/i18n/messages/zh-CN.ts`
- `mobile/src/main.ts`
- `mobile/src/router/index.ts`
- `mobile/src/styles/deposit.css`
- `mobile/src/views/DepositAssetView.vue`
- `mobile/src/views/DepositDetailView.vue`
- `mobile/src/views/DepositNetworkView.vue`
- `mobile/tests/deposit-order-history.test.ts`
- `mobile/tests/fixtures/timed-deposit-browser.js`
- `mobile/tests/pencil-wallet-flow-parity.test.ts`
- `mobile/tests/route-accessibility.test.ts`
- `mobile/tests/timed-deposits.test.ts`
- `mobile/tests/wallet-secondary-views.test.ts`
- `pc/src/core/timedDeposit.ts`
- `pc/src/i18n/index.ts`
- `src/modules/agent/application/funding.rs`
- `src/modules/wallet/application/timed_deposits.rs`
- `src/modules/wallet/infrastructure/timed_deposits/orders.rs`
- `src/modules/wallet/infrastructure/timed_deposits/settlement.rs`
- `src/modules/wallet/presentation/timed_deposits.rs`
- `src/modules/wallet/routes/timed_deposits.rs`
- `src/openapi/agent_funding.rs`
- `src/openapi/timed_deposits.rs`
- `tests/agent_routes/funding.rs`
- `tests/openapi_routes/timed_deposits.rs`
- `tests/unit_src/src_modules_wallet_timed_deposit_cancellation_tests.rs`
- `tests/unit_src/src_modules_wallet_timed_deposit_database_tests.rs`
- `tests/wallet_routes/timed_deposits.rs`
- `web/src/admin/deposits/DepositReviewPage.test.tsx`
- `web/src/admin/deposits/DepositReviewPage.tsx`
- `web/src/admin/deposits/ReceiptReview.tsx`
- `web/src/admin/deposits/RechargeWorkspace.css`
- `web/src/admin/deposits/contracts.ts`
- `web/src/api/agentFunding.test.ts`
- `web/src/api/agentFunding.ts`
- `.trellis/tasks/10-25-chain-deposit-experience/`

Unrelated paths excluded:
- `.trellis/tasks/10-21-agent-coin-logo-security-block/`
- `.trellis/tasks/10-22-diagnose-seconds-orders-stuck-pending-settlement/`
- `.vite/`
