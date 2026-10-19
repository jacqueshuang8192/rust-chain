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

Excluded: pre-existing `.vite/` cache. No commit, push or deployment executed.

## Execution

2026-10-20: proceeding with both verified mobile fixes in separate commits, then task archival and a normal push to `origin/main`. Local `.vite/` cache is excluded.
