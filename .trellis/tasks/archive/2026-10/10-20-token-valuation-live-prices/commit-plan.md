# Commit manifest

Work commit: `fix: resolve generated token prices for holdings and wallets`

- `.trellis/spec/backend/token-issuance-v2.md`
- `.trellis/spec/guides/cross-layer-thinking-guide.md`
- `.trellis/tasks/10-20-token-valuation-live-prices/check.jsonl`
- `.trellis/tasks/10-20-token-valuation-live-prices/commit-plan.md`
- `.trellis/tasks/10-20-token-valuation-live-prices/delivery.md`
- `.trellis/tasks/10-20-token-valuation-live-prices/implement.jsonl`
- `.trellis/tasks/10-20-token-valuation-live-prices/prd.md`
- `.trellis/tasks/10-20-token-valuation-live-prices/research/root-cause.md`
- `.trellis/tasks/10-20-token-valuation-live-prices/task.json`
- `docs/superpowers/PROGRESS.md`
- `src/modules/new_coin/application/issuance.rs`
- `src/modules/new_coin/infrastructure/issuance/trading.rs`
- `src/modules/wallet/application/issuance_valuation.rs`
- `tests/agent_routes/token_lifecycle.rs`
- `tests/agent_routes/token_lifecycle/direct_purchase.rs`
- `tests/agent_routes/token_lifecycle/live_valuation.rs`

Exclude existing untracked `.vite/` cache. After the work commit, record its hash
in task metadata, archive only this task and add the session journal in separate
bookkeeping commits. No remote push or deployment is part of this task.
