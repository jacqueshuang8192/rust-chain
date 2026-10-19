# Local commit plan

`fix(mobile): simplify new coin rule labels`

- `mobile/src/components/TokenIssuanceRules.vue` — remove only the listing-reference display row.
- `mobile/src/core/tokenIssuanceMessages.ts` — rename two labels in Chinese and English.
- `docs/superpowers/PROGRESS.md` — record scope and validation.
- `.trellis/tasks/10-20-new-coin-rule-copy/` — task record.

Exclude `.vite/`. No backend, policy data, pricing or unlock logic changes. No new tests are necessary for these copy-only edits; use the existing release gate. No spec change is needed because this introduces no new behavioral contract. Remote push and deployment are outside this edit request.

Business commit: `3e4cb7c411c6e45ce7cd26d6f8bd7e98cc9043fd`. Validation completed successfully; local only, not pushed or deployed.
