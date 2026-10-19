# Proposed commit plan

Status: user explicitly authorized this task's commit and push on 2026-10-19 (「那就提交推送」). Execute the approved 54-path work commit, archive only this task, record the journal and perform a normal push to origin/main.

## Work commit

`feat(admin): complete v2 token issuance management`

One coherent work commit contains the admin v2 create/read contracts, fixed-market guards, UI and compatibility tests, migration 0154, executable specs, progress and task delivery evidence. No existing published migration is rewritten.

Exact file list (54 paths):

- `.trellis/spec/admin/ui-system.md`
- `.trellis/spec/backend/new-coin-project-center.md`
- `.trellis/spec/backend/token-issuance-v2.md`
- `.trellis/tasks/10-19-admin-token-lifecycle/check.jsonl`
- `.trellis/tasks/10-19-admin-token-lifecycle/commit-files.txt`
- `.trellis/tasks/10-19-admin-token-lifecycle/commit-plan.md`
- `.trellis/tasks/10-19-admin-token-lifecycle/delivery.md`
- `.trellis/tasks/10-19-admin-token-lifecycle/implement.jsonl`
- `.trellis/tasks/10-19-admin-token-lifecycle/prd.md`
- `.trellis/tasks/10-19-admin-token-lifecycle/research/admin-gaps.md`
- `.trellis/tasks/10-19-admin-token-lifecycle/research/browser-qa.md`
- `.trellis/tasks/10-19-admin-token-lifecycle/task.json`
- `docs/superpowers/PROGRESS.md`
- `migrations/0154_admin_token_issuance_receipts.sql`
- `src/modules/admin/application.rs`
- `src/modules/admin/application/new_coin_market.rs`
- `src/modules/admin/application/token_issuance.rs`
- `src/modules/admin/infrastructure.rs`
- `src/modules/admin/infrastructure/new_coin.rs`
- `src/modules/admin/infrastructure/token_issuance.rs`
- `src/modules/admin/presentation.rs`
- `src/modules/admin/presentation/new_coin.rs`
- `src/modules/admin/presentation/token_issuance.rs`
- `src/modules/admin/routes.rs`
- `src/modules/admin/routes/token_issuance.rs`
- `src/modules/admin/service/access_control.rs`
- `src/modules/agent/application/coin_applications.rs`
- `src/modules/agent/infrastructure/coin_applications.rs`
- `src/modules/new_coin/infrastructure/issuance.rs`
- `src/openapi.rs`
- `src/openapi/admin_token_issuance.rs`
- `tests/admin_routes.rs`
- `tests/admin_routes/new_coin_market.rs`
- `tests/admin_routes/token_issuance.rs`
- `tests/openapi_routes.rs`
- `web/src/admin/access.tsx`
- `web/src/admin/navigation.tsx`
- `web/src/admin/new-coins/CreateTokenProject.test.tsx`
- `web/src/admin/new-coins/CreateTokenProject.tsx`
- `web/src/admin/new-coins/NewCoinMarketSetup.test.tsx`
- `web/src/admin/new-coins/NewCoinMarketSetup.tsx`
- `web/src/admin/new-coins/NewCoinProjectPage.test.tsx`
- `web/src/admin/new-coins/NewCoinProjectPage.tsx`
- `web/src/admin/new-coins/TokenRecordsPage.test.tsx`
- `web/src/admin/new-coins/TokenRecordsPage.tsx`
- `web/src/admin/new-coins/__tests__/fixtures.ts`
- `web/src/admin/new-coins/projectModel.ts`
- `web/src/admin/new-coins/useNewCoinMarket.ts`
- `web/src/admin/resources/AdminResourcePage.tsx`
- `web/src/admin/resources/resourceConfigs.test.tsx`
- `web/src/admin/resources/resourceConfigs.tsx`
- `web/src/admin/routes.tsx`
- `web/src/layouts/AdminLayout.test.tsx`
- `web/src/shared/adminFieldLabels.ts`

## Excluded dirty files

- `.vite/` — pre-existing local cache; exclude and preserve.
- No other unrecognized dirty files were found in the final inventory. Temporary QA scripts/screenshots/logs are outside the repository.

## After confirmation

Stage exactly the paths listed in `commit-files.txt`, inspect the staged diff and create the work commit above without amend. Then use the normal Trellis archive and journal steps for this task only; those create their separate bookkeeping commits. Do not include older unarchived tasks. Do not push unless the user explicitly authorizes this delivery's push.

The full JavaScript budget is still above the existing threshold (2,712,495 > 2,600,000 bytes); build and functional checks pass. No production deployment or migration is included.
