# Mainnet recharge presets delivery — 2026-10-24

## Delivered

- Explicit draft-only node and asset presets in existing network editors. Ethereum ETH/USDT/USDC, Tron TRC20 USDT, Solana SOL/USDT/USDC; four node choices (Alchemy, TronGrid, public Solana, Helius).
- Issuer identities and node capability notes verified against sources in presets.md. Preserve existing confirmation depth, revision/reason, permissions, admission state and secrets. No automatic save or seed migration.
- Unresolved credential placeholders cannot be saved, including with admission disabled. Existing asset identities cannot be replaced. Manual symbol changes clear stale token identity.
- Browser found a popup leave/reopen animation race. Added optional AdminSelect motion passthrough and disabled motion only on the two new preset selectors. Three consecutive public/Helius switch cycles and SOL/USDT/USDC switches passed in the same editor.

## Validation

- `npm --prefix web run typecheck` and `npm --prefix web run lint`: passed after final change.
- `npm --prefix web test`: final 125 files, 1182 tests passed (286.46 seconds); includes 26 focused editor/preset tests.
- `npm --prefix web run test:production-policy`: 15 tests passed.
- `npm --prefix web run test:coverage`: passed; statements 85.61%, branches 81.78%, functions 85.33%, lines 92.87%.
- `npm --prefix web run build`: passed.
- `npm --prefix web run budget`: existing failure remains. Total JS 2,754,677 bytes versus 2,600,000 budget, +5,251 bytes from prior delivery 2,749,426. No budget relaxed; no unrelated bundle refactor.
- `git diff --check` and task context validation: passed.

## Real browser / HTTP scope

Ego task space 7 used Vite 3037 and a real Rust router on 18087 backed by isolated MySQL 13368/funding_test. No workers or production services ran. Temporary fixture creation initially failed because its role label exceeded the column size; shortened only the temporary fixture label, then started successfully. Fixture removed after use.

Verified Ethereum Alchemy URL/depth and blocked unresolved key, discard protection, TronGrid URL/depth, real TRC20 USDT creation through confirmation with persisted list result, Solana public/Helius URL/depth and blocked key, all Solana asset identities, repeated selector changes. Mainnet asset exact payloads for all seven presets also covered by component tests.

Desktop 1728 and widths 390/320: page scrollWidth equals viewport width after settled layout, dropdowns usable, narrow footer cancel/discard succeeds. Screenshots `/tmp/presets-solana-final-{1728,390,320}.png`; narrow screenshots visually inspected (desktop inspection also performed on earlier capture). Resize while an old popup is open can retain its old width until it closes; layout checks reopen the popup at each viewport. No claim of a global shell or unrelated resource page audit.

No real RPC credential validation, chain transfer, scanner readiness or production end-to-end test. Existing Ethereum mappings in the fixture were preserved. This is a preset editor delivery, not a preconfigured live deployment.

Task space finished; API/Vite/isolated MySQL stopped; session token deleted; temporary Rust fixture removed. Ego Lite upgrade notice left unacted upon. No push or deployment.
