# Proposed local commit

`fix(pwa): recover updates and share the install dialog`

- `mobile/src/pwa/index.ts`
- `mobile/src/pwa/update.ts`
- `mobile/src/components/PwaStatus.vue`
- `mobile/src/i18n/messages/zh-CN.ts` — PWA hunks only
- `mobile/src/i18n/messages/en.ts` — PWA hunks only
- `mobile/tests/pwa-lifecycle.test.ts`
- `mobile/tests/pwa-status-immersive.test.ts`
- `mobile/tests/pwa-prompt-behavior.test.ts`
- `.trellis/spec/mobile/pwa-and-shell.md`
- `.trellis/tasks/10-20-pwa-update-recovery/`
- `docs/superpowers/PROGRESS.md` — PWA entries only

Preserve the previously verified new-coin task, including its independent locale/progress hunks. Its proposed commit remains in `../10-20-new-coin-pending-listing/commit-plan.md`; stage by task if both are approved. Exclude the pre-existing `.vite/` cache. Business commit created; pushed to `origin/main`; no deployment.

## Execution

2026-10-20: proceeding with both verified mobile fixes in separate commits, then task archival and a normal push to `origin/main`. Local `.vite/` cache is excluded.

Business commit: `8ef0a379421e28b41932466be95e5e66145c1ff7`. Staged scope matched this plan exactly; cached diff check passed.

Remote verification: normal push to `origin/main` succeeded; `git ls-remote` matched local HEAD `ead6af5e0b6ba20e941f2fc8e05beaed14d06cfb`. This final delivery record is committed and pushed afterward. No deployment.
