# Delivery — built-in anonymous RPC channels

## Implemented

- Server-owned catalog of eight public presets with six recommended entries across Ethereum/Base/Tron/Bitcoin/Solana. First setup prepopulates local drafts; existing pools append missing defaults. Limited Base/Solana official endpoints remain optional.
- Migration 0168 stores nullable provider identity without modifying custom secrets, cooldowns or cursors. Fixed-endpoint server resolution, anonymous key isolation, exact-network validation, same-preset state preservation and effective audit identity.
- Verified Base public profiles use full transaction blocks with unchanged receipt validation/root identity. Existing custom/Nodies batches and Ethereum callTracer semantics remain. Solana legacy/V0/V1 are accepted; unsupported explicit versions stop the block.
- Admin source/limit notes, no-key rows, anonymous interval propagation, Base ETH/USDC asset templates, snapshot-pinned saves and narrow confirmation/discard dialogs.

## Verification

- `cargo test --lib timed_deposits::scanners -- --nocapture`: **75/75**, all dedicated MySQL variables set; no scanner database tests skipped. Migration/replay, old custom secrets and cooldowns, anonymous conversion, old-client blank saves, tampering/cross-network rejection, audit rollback and revision fencing were executed.
- New Base full-block test initially compared differently scaled decimal strings; changed assertion to exact BigDecimal equality, then all scanner tests passed.
- MySQL 8.4 on local 127.0.0.1:33417. Docker disk was full; initial isolated container exited. Recreated this task's database with a private 2 GB tmpfs. Other business containers were unchanged.
- `cargo test --test backend_architecture --test openapi_routes`: architecture 11/11; OpenAPI suite reports 20 passing (its unrelated optional carousel database fixture was not configured). New RPC catalog/identity contract assertion passed.
- Production Clippy `--lib --bins -- -D warnings`, all-target check, fmt and diff checks passed.
- Whole working-tree Web suite: **1225 passed / 1 failed** initially; only failure was the explicit stablecoin whitelist needing the new Base USDC identity. Updated that precise expectation. Final changed-domain suite: **64/64** including all-network drafts, real Semi selection, conflict retention, custom-secret preservation, preset replacement and hidden interval propagation.
- Web typecheck/lint, production-policy 15/15, coverage 23/23 with thresholds, and build passed. Whole working-tree budget is 2791952/2600000 bytes and still fails the existing budget.
- Exported HEAD Web plus only this task's changes, excluding unrelated agent-management WIP. Exact release typecheck/lint, 64/64 deposit tests and build passed; JS budget **2786186/2600000** still fails. No budget limits were changed.
- Local Ego Browser actual components with isolated mock API: 1728px Ethereum default; 1280px Base PublicNode+dRPC; 390px Tron/Bitcoin/Solana defaults with no key fields/overflow. Saved Base defaults remained admission-off and awaiting scan confirmation.
- At 320px reproduced the 448px confirmation overflow; after the scoped fix, save/discard dialogs measure 288px with x=16..304 and no horizontal clipping. At 390px, custom+two defaults remain separate, duplicate append is disabled, a 409 retains three provider drafts/reason and the dialog fits 358px. Screenshots were visually inspected.
- Low-frequency public RPC probes and official references are in `providers.md` and sanitized JSON files; no user keys or real transfers were used. Circle native Base USDC identity was rechecked.

## Release and limits

Stop old deposit workers, run the same-version migrator through 0168 and every pending migration, update API/worker/Admin together, then resume. Complete 1Panel compose update includes its migrate service; an API-only restart does not. Admin must confirm/save drafts; no production defaults were silently activated. Existing cooldowns and progress persist.

Public endpoints are best effort: sampled method success does not guarantee sustained catch-up, history or uptime. Ethereum dRPC trace worked in the probe but its published free-tier policy does not guarantee that permission. Native ETH tracing still fails closed when unavailable. Base native supports direct transfers only. Tron native TRX and Solana Token2022 remain unsupported. Production deployment, actual deposits and remote CI were not verified.

## Scope isolation

Only this task's source, tests, documentation and progress slices are committed. Existing agent-management code/CSS/tests, the admin UI spec edit, three older progress entries, four old untracked task directories and `.vite/` remain local.

The task's isolated database, Vite process and Ego task space were stopped/closed after verification. Both remotes were fetched before committing and matched starting HEAD `9f283b8e` (0 ahead / 0 behind).
