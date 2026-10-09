# Proposed Commit

Superseded on 2026-10-10: the user explicitly requested committing and pushing
all current work. The task-only inclusion/exclusion plan below is historical.
The authorized batch includes other existing source, tests, design and project
documents, plus the pending local commits. Only disposable `.vite/` cache and
Git-ignored local configuration stay outside the commit. No force push.

Await one user confirmation before staging or committing. Do not push.

`feat(agent): add filterable team order views`

## Included Files

Brace groups below denote only the explicitly listed files, not entire folders.

- `src/modules/agent/{application,infrastructure,presentation,routes}/team_orders.rs`
- `src/modules/agent/{application,infrastructure,presentation,repository,routes}.rs`
- `src/openapi.rs`
- `src/openapi/{agent_portal,agent_team_orders}.rs`
- `tests/agent_routes.rs`
- `tests/agent_routes/team_orders.rs`
- `web/src/agent/TeamOrdersPage.{tsx,css,test.tsx}`
- `web/src/agent/teamOrderModel.{ts,test.ts}`
- `web/src/agent/routes.{tsx,test.tsx}`
- `web/src/api/agent.ts`
- `web/src/api/agentTeamOrders.{ts,test.ts}`
- `web/src/layouts/AgentLayout.{tsx,test.tsx}`
- `web/src/shared/FilterBar.{tsx,test.tsx}`
- `web/src/shared/SemiFormControls.tsx`
- `web/tests/fixtures/agent-team-orders.ts`
- `.trellis/spec/backend/{agent-hierarchy,order-identifiers}.md`
- `.trellis/tasks/10-10-agent-team-orders/` task documents and local browser fixture
- `docs/superpowers/PROGRESS.md`: only entries about this task from 01:30 onward;
  stage selectively, never the complete existing file diff.

## Excluded Existing Work

These changes predate this feature and remain untouched by its proposed commit:

- `.trellis/spec/admin/chinese-presentation.md`
- `.trellis/spec/backend/{index,logging-guidelines,market-provenance,realtime-websockets,seconds-contracts}.md`
- `.trellis/spec/mobile/pwa-and-shell.md`
- `.trellis/tasks/10-10-realtime-console-quality/`
- `.vite/`
- `mobile/index.html`
- `mobile/pencil/hippo-mobile-uiux.pen`
- `mobile/{pnpm-lock.yaml,pnpm-workspace.yaml,vite.config.ts}`
- `mobile/src/components/MobileMarketChart.vue`
- `mobile/src/composables/useSecondsPrivateReconciliation.ts`
- `mobile/src/core/privateUserStreamManager.ts`
- `mobile/src/stores/theme.ts`
- `mobile/src/views/SecondsView.vue`
- `mobile/tests/{mobile-market-chart-history,private-user-stream,seconds-live-multi-orders,seconds-private-reconciliation,theme}.test.ts`
- `pc/src/api/{second,stomp}.ts`
- `pc/src/components/chart/MarketChart.vue`
- `pc/src/stores/second.ts`
- `pc/src/views/SecondOptions.vue`
- `pc/tests/{chart-provider,guest-auth-states,seconds-reconciliation,stomp}.test.ts`
- `src/bin/exchange-migrate.rs`
- `src/infra/{logging,mod}.rs`
- `src/{main,migration}.rs`
- `src/modules/{prediction,quick_recharge}/application.rs`
- `src/modules/seconds_contract/infrastructure.rs`
- `src/workers/{agent_commission_settlement,earn_auto_redemption,loan_overdue,margin_interest,margin_liquidation,seconds_contract_settlement,synthetic_market,unlock_scanner,wallet_chain}.rs`
- `tests/seconds_contract_settlement_worker.rs`
- `tests/unit_src/src_infra_logging_tests.rs`
- `web/src/admin/actions/AgentManagementPage.test.tsx`
- `web/src/admin/resources/AdminResourcePage.test.tsx`
- `web/src/agent/UserPortfolioPage.{css,tsx}`
- `web/src/agent/pages.{tsx,test.tsx}`
- `web/src/layouts/AgentLayout.css`
- `web/src/shared/DetailDrawer.{css,tsx,test.tsx}`
- `web/src/shared/{adminEnumLabels,adminResponseFieldLabels}.ts`
- Earlier entries in `docs/superpowers/PROGRESS.md`

## Before Committing

- Recheck the dirty-file inventory for changes since this plan.
- Preserve all unrelated hunks and stage only the files/sections above.
- Inspect the staged diff and run `git diff --cached --check`.
- Keep known bundle-budget and legacy invite-test failures in the delivery
  record; this feature commit is not deployment or release approval.
