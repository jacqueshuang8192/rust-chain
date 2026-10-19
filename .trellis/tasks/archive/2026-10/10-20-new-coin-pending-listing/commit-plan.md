# Proposed local commit

`fix(new-coin): align mobile phase guidance and refresh`

- `.trellis/spec/mobile/backend-integration.md`
- `.trellis/tasks/10-20-new-coin-pending-listing/check.jsonl`
- `.trellis/tasks/10-20-new-coin-pending-listing/commit-plan.md`
- `.trellis/tasks/10-20-new-coin-pending-listing/delivery.md`
- `.trellis/tasks/10-20-new-coin-pending-listing/implement.jsonl`
- `.trellis/tasks/10-20-new-coin-pending-listing/prd.md`
- `.trellis/tasks/10-20-new-coin-pending-listing/research/browser-checks.mjs`
- `.trellis/tasks/10-20-new-coin-pending-listing/research/browser-results.json`
- `.trellis/tasks/10-20-new-coin-pending-listing/research/browser-setup.mjs`
- `.trellis/tasks/10-20-new-coin-pending-listing/research/diagnosis.md`
- `.trellis/tasks/10-20-new-coin-pending-listing/task.json`
- `docs/superpowers/PROGRESS.md`
- `mobile/src/components/TokenIssuanceRules.vue`
- `mobile/src/components/TokenSubscriptionForm.vue`
- `mobile/src/components/new-coin/NewCoinProjectCard.vue`
- `mobile/src/composables/useNewCoinProject.ts`
- `mobile/src/core/tokenIssuanceMessages.ts`
- `mobile/src/i18n/messages/en.ts`
- `mobile/src/i18n/messages/zh-CN.ts`
- `mobile/src/views/NewCoinDetailView.vue`
- `mobile/src/views/NewCoinsView.vue`
- `mobile/tests/new-coin-lifecycle-refresh.test.ts`
- `mobile/tests/new-coin-pencil-contract.test.ts`
- `mobile/tests/secondary-product-order-views.test.ts`

Excluded: pre-existing `.vite/` cache. Business commit created; pushed to `origin/main`; no deployment.

## Execution

2026-10-20: proceeding with both verified mobile fixes in separate commits, then task archival and a normal push to `origin/main`. Local `.vite/` cache is excluded.

Business commit: `d467cf4eca9d8c76be731b2d7801e151a3b1c598`. Staged scope matched this plan exactly; cached diff check passed.

Remote verification: normal push to `origin/main` succeeded; `git ls-remote` matched local HEAD `ead6af5e0b6ba20e941f2fc8e05beaed14d06cfb`. This final delivery record is committed and pushed afterward. No deployment.
