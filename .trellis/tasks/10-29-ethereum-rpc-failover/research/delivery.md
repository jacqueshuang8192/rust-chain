# Delivery — Ethereum free-account RPC templates

## Implemented

- Added Chainnodes and SolidRPC free-account templates with official application/documentation links, published quota notes and separate encrypted API-key input. Both are existing custom-provider configurations, not new anonymous preset identities.
- Fresh Ethereum configuration proposes an unsaved Chainnodes draft. An enabled URL-template credential requires its own key before save; adding templates preserves existing channels and admission state. SolidRPC is an optional short-term backup with limited daily quota.
- Selecting another service template replaces credential drafts with fresh blank slots and no stored key IDs. Unchanged saved channels retain blank-save behavior. The local `template_id` never leaves the form.
- Retained `eth-drpc` identity and endpoint but removed its default recommendation. Existing records, real cooldowns, progress and internal-transfer requirements remain intact.
- No API DTO, migration, scanner, credit, or failover algorithm change. The legacy single-node editor excludes the two new templates because their separate-key workflow belongs to the multi-provider editor.

## Verification

- `cargo test --lib timed_deposits::scanners -- --nocapture`: **75/75**, with all required isolated MySQL test URLs configured; no database cases skipped. The expanded preset test covers missing-key rejection, append rollback, retained ciphertext/cooldown, fresh key IDs on replacement, and secret-free settings/audits. Existing failover, stream, admission and recovery cases passed.
- Test-only MySQL 8.4 container `rust-chain-rpc-eth-test`, local port 33418 and private tmpfs; stopped and automatically removed after verification. Other containers were not changed.
- `cargo test --test backend_architecture`: **11/11**. `cargo fmt --all -- --check` and `git diff --check` passed. No new Clippy/all-target/OpenAPI run: this task changes only catalog metadata and tests on the Rust side; public signatures, modules and schema are unchanged.
- Working-tree Web suite: **1227 passed / 1 failed**. The failure is the existing `TimestampText` test accepting UTC/Shanghai hours but running in Europe/Istanbul. `TZ=UTC npm --prefix web run test -- src/shared/format.test.tsx`: **11/11**. No unrelated timezone expectation was edited.
- Deposit-domain suite **65/65** passed after aligning the service-template select accessible label. Initial targeted run had one accessibility-label mismatch; its focused rerun and the full suite passed after correction.
- Web typecheck/lint, production-policy **15/15**, coverage **23/23** with thresholds, and production build passed. Working-tree JS **2795283/2600000 bytes** still exceeds the existing budget; the threshold was not changed.
- Exported HEAD Web plus exactly this task's seven Web file patches, excluding agent-management WIP. Typecheck/lint passed, editor tests **46/46** and remaining deposit tests **19/19** passed, build passed. Exact release JS **2789517/2600000 bytes** still exceeds budget (previous release 2786186; increase 3331 bytes).
- Ego Browser TaskSpace 10, actual components with isolated mock API: 1728px template addition/independent keys; 1280px Base PublicNode+dRPC defaults; 320px Chainnodes default, reachable key input, blocked empty-key save, two-channel save and admission remaining off. Confirm dialog 288px at x=16; no document horizontal overflow.
- At a measured 390px viewport, existing-channel append and HTTP 409 retain both draft names, old blank key and reason. Dialog measures 358px at x=16; cancellation/discard yields no successful save. Screenshots were visually inspected. Generic Vite client script errors appeared around fixture reload/navigation; functional checks completed after inspecting state. Production login/API writes were not exercised.
- Task context validation passed. Temporary browser task, Vite process and isolated database were closed; generated Python bytecode was removed.

## Research and production limits

Official sources and eight bounded anonymous candidates are recorded in `providers.md` and the probe files. No extra anonymous node passed the full trace capability probe. No user keys, production data, account registration or real transfers were used.

The two account templates are documentation-qualified only. Exact trace response structure, historical access and continuous capacity still require an owned key. Full transaction identity/call-tree validation remains authoritative; unsupported results fail closed. Do not claim that production HTTP 429 has already been fixed or that several keys multiply an account quota.

Update API and Admin to this version so the catalog recommendation and editor agree. This task adds no migration; deployments still behind the previous release must apply 0168 and all pending migrations using the same-version migrator, with old workers stopped. In Ethereum configuration, add the account template, obtain its RPC API key through the official link, enter it in its independent slot and confirm save. Review priorities and wait for full successful scans; existing cooldowns and history are not reset or skipped. No production deployment or remote CI was verified.

## Scope isolation

Only the twelve related source/test/spec/progress paths and this task's records are delivered. Existing agent-management source/CSS/tests/fixture, its admin UI spec, three older progress entries, four old untracked tasks and `.vite/` remain local. The PROGRESS index is built from HEAD plus this task's appended sections to preserve the other entries uncommitted.
