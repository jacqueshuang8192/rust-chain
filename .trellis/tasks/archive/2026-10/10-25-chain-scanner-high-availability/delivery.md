# Delivery: multi-provider chain scanner

## Result

Each network supports up to ten priority-ordered providers, each with up to ten API key slots. Admins control enablement and provider/key request intervals, retain existing encrypted credentials through blank edits, explicitly remove keys, and inspect errors/cooldowns. Existing single-node configurations migrate to a default provider without changing checkpoints.

The scanner fails over on eligible provider failures, persists pacing/cooldowns, honors bounded Retry-After, and checks chain identity and saved block hashes again for each attempt. Chain continuity, identity and accounting failures stop the network. Exhausting all candidates preserves progress and closes admission until a successful scan catches up. Configuration revision/lease fences stop in-flight obsolete work.

## Verification

- Real API + isolated MySQL browser: added a key and third provider, saved with reason, reopened with blank secret inputs; rejected invalid intervals; confirmed dirty-draft cancellation. Database verified three channels/five key slots. Viewed desktop 1728px and narrow 390/320px screenshots; no horizontal overflow.
- Architecture 11/11 and OpenAPI 20/20 passed.
- Web typecheck, lint, production policy 15/15, coverage suite 23/23 and build passed.
- Existing Web bundle limit still fails: 2,774,218 / 2,600,000 raw JS bytes; +10,258 bytes from the previous recorded build. No thresholds changed.
- Initial full Web run: 1196 passed, one pre-existing timestamp assertion failed under the host +03 timezone. Explicit Asia/Hong_Kong rerun passed 1197/1197 across 126 files.
- Final database and scanner regression: 21/21 passed, including 0161-to-0162 migration/replay, 429→401→healthy fallback, restart/cross-round cooldown, completion-based pacing, configuration-save fencing, cursor preservation, secret-free audit and original accounting regressions. Tron query credentials remain intact when appending REST operations.
- `cargo check --all-targets`, final real-MySQL HTTP route 1/1, production `cargo clippy --lib --bins -- -D warnings`, fmt, diff and task context validation passed.

## Release

Run migration 0162 before upgrading the backend/worker, and release the backend and admin together because the strict settings contract now includes providers. Existing credentials stay encrypted and are migrated once; no cursor reset is required. Add real provider URLs/keys in Admin → 链上充值 → 网络与收款配置 → 编辑监听配置. This task has not configured third-party accounts or verified live-chain receipt/credit. Multiple keys do not establish additional account quota.

## Workspace

The user authorized committing and pushing this delivery on October 25, 2026. The work commit, task archive and session record are included in this origin/main delivery; remote SHA verification follows the push. No deployment performed. Unrelated task directories and .vite/ remain untouched. Temporary browser API fixture was removed, Vite/API processes stopped, temporary auth file deleted and Ego task space finished. The task-owned isolated MySQL process was stopped after the final integration tests.
