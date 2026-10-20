# Local commit plan

One cohesive task commit:

`fix: streamline token purchases and prelisting valuations`

- `.trellis/spec/backend/token-issuance-v2.md`
- `.trellis/spec/mobile/backend-integration.md`
- `.trellis/tasks/10-20-new-coin-direct-buy-detail/check.jsonl`
- `.trellis/tasks/10-20-new-coin-direct-buy-detail/commit-plan.md`
- `.trellis/tasks/10-20-new-coin-direct-buy-detail/delivery.md`
- `.trellis/tasks/10-20-new-coin-direct-buy-detail/implement.jsonl`
- `.trellis/tasks/10-20-new-coin-direct-buy-detail/prd.md`
- `.trellis/tasks/10-20-new-coin-direct-buy-detail/research/decisions.md`
- `.trellis/tasks/10-20-new-coin-direct-buy-detail/task.json`
- `docs/superpowers/PROGRESS.md`
- `mobile/src/api/tokenIssuance.ts`
- `mobile/src/api/wallet.ts`
- `mobile/src/components/TokenIssuanceRules.vue`
- `mobile/src/components/TokenSubscriptionForm.vue`
- `mobile/src/components/TokenTradingPanel.vue`
- `mobile/src/core/tokenIssuance.ts`
- `mobile/src/core/tokenIssuanceMessages.ts`
- `mobile/src/core/types.ts`
- `mobile/src/core/walletValuation.ts`
- `mobile/src/styles/token-issuance.css`
- `mobile/src/views/AssetsView.vue`
- `mobile/src/views/NewCoinDetailView.vue`
- `mobile/tests/award-ui-assets-profile.test.ts`
- `mobile/tests/new-coin-pencil-contract.test.ts`
- `mobile/tests/token-direct-purchase.test.ts`
- `mobile/tests/token-issuance.test.ts`
- `mobile/tests/wallet-valuation.test.ts`
- `pc/src/api/asset.ts`
- `pc/src/api/backendAdapters.ts`
- `pc/src/api/tokenIssuance.ts`
- `pc/src/components/TokenTradingPanel.vue`
- `pc/src/core/tokenIssuance.ts`
- `pc/src/core/tokenIssuanceMessages.ts`
- `pc/src/core/walletValuation.ts`
- `pc/src/views/User/Assets.vue`
- `pc/tests/token-issuance.test.ts`
- `pc/tests/wallet-valuation.test.ts`
- `src/modules/new_coin/application/issuance.rs`
- `src/modules/new_coin/infrastructure/issuance.rs`
- `src/modules/new_coin/presentation/issuance.rs`
- `src/modules/new_coin/routes/issuance.rs`
- `src/modules/wallet/application.rs`
- `src/modules/wallet/application/issuance_valuation.rs`
- `src/modules/wallet/infrastructure/accounts_ledger.rs`
- `src/modules/wallet/presentation.rs`
- `src/modules/wallet/routes.rs`
- `src/openapi/token_issuance.rs`
- `tests/agent_routes/token_lifecycle.rs`
- `tests/agent_routes/token_lifecycle/direct_purchase.rs`
- `tests/openapi_routes.rs`

Excluded: pre-existing untracked `.vite/` cache.

Follow with task archive and journal bookkeeping commits. No remote push or deployment.
