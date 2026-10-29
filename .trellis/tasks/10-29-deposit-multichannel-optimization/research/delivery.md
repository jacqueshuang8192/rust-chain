# Delivery — five-network multi-channel configuration

## Changes

- Shared sequential failover across Ethereum, Base, Tron, Bitcoin and Solana now visits one eligible key per provider in priority/ID order before returning to sibling keys. The cap increases from 4 to 10 while the round remains 45 seconds and an attempt remains 35 seconds. Slow attempts can still consume the round before every candidate runs; no guaranteed instant takeover is claimed.
- Shared provider cooldown discovered during admission excludes the provider's other keys. Key-only cooldown does not. Existing pacing, retry deadlines, encryption, revision/lease checks, full chain evidence and financial error stops remain intact.
- Bitcoin anchors request verbosity 1 instead of 2; Solana anchors request no transaction details instead of full. Payment scans still require full evidence. EVM/Tron already avoided trace/receipt work for empty-asset anchors.
- Settings now returns Base provider `stream_failures` from the existing table in one batch query. No secrets or endpoints are returned and no migration is added. Old API responses without this field display unknown Base state; malformed new data is rejected.
- Admin opens with an RPC overview, then scan progress and setup guidance. Channel counts, configured priority order, key cooldown/error disclosures and Base native/token state are separate. The editor previews ordering, folds existing channels, expands new channels, and preserves credentials and conflict drafts while folded. Collapsed fieldsets use display:none to remove keyboard focus targets while keeping React state.

## Validation

- Rust scanner suite: **77/77** actual tests, all nine isolated MySQL environment variables set. Includes all five networks reaching the sixth provider despite four credentials per preceding provider, anchor parameter assertions, migrations, durable cooldowns, Base streams, revision conflicts and recovery/accounting.
- Final gate test expansion: **1/1** actual isolated DB test; key cooldown keeps sibling eligibility, provider cooldown during a wait excludes sibling credentials. Used a fresh database. Architecture **11/11**, OpenAPI **1/1**, Clippy production lib/bins with -D warnings, all-target check, fmt and diff checks pass.
- Web full suite under UTC: **1233 passed, 1 new failure**. The failure exposed zero-height collapsed controls remaining visible/focusable in the DOM. After explicit hiding, its focused rerun passes and final complete deposit-domain suite is **71/71**. Other full-suite tests were not rerun after this localized fix.
- Web typecheck/lint, production-policy **15/15**, coverage **23/23** with thresholds, and production build pass.
- Exact release export (HEAD plus only this task's Web patches, excluding agent-management WIP): typecheck, lint, **71/71** deposit tests and build pass. Existing bundle limit still fails: **2795023 / 2600000** raw JS bytes (working tree with unrelated WIP: 2800763). Budget not relaxed; previous exact release was 2789517. The final read-only fixture permission string correction is local-test-only and was verified in the browser.
- Ego TaskSpace 12, local actual components with isolated mock API: 1728px populated Base overview and independent stream status; desktop priority changes; collapse/expand preserves independent replacement Key and old blank Key; 1280px empty overview and default network editors. An initial Bitcoin selection raced with menu updates and still showed Tron; rerun explicitly waits for selected network and title.
- 320px actual measured viewport: page scroll width 320, close button within x272..296, successful three-channel save, confirmation dialog x16 width288. 390px actual measured viewport: independent replacement survives HTTP409 with reason, confirmation dialog x16 width358; cancellation/discard performs no save. Inputs and form are contained (one subpixel rounding difference in scroll width). Screenshots visually inspected. One earlier screenshot named confirm-320 reset to 1728 across script invocations; it is NOT counted as narrow validation. The repeated same-invocation 320 save is valid.
- Known browser selector failures were resolved by inspecting current DOM and using the observed option CSS; no production sites or writes involved. Local read-only role is verified separately. Local fixture screenshots are under /tmp/rust-chain-multichannel-*.

## Scope and operational limits

No production configuration, deployment, account creation, live payment or sustained provider-load test was performed. No new RPC providers or guessed quotas were added. Ethereum still requires valid complete trace support. Multiple keys do not multiply account capacity or remove genuine cooldowns.

Official getBlock documentation lookup via web returned no source content; a direct read timed out. No new provider capability or current quota claim is based on that lookup. Anchor options are covered by request-shape and missing-evidence regressions, not a live-provider acceptance claim.

Update API, worker and admin together. No new migration; environments behind prior releases still need their pending migrations. Existing agent-management changes, old progress sections and old task folders remain excluded. Local commits only; no push authorized by this latest request.


## Final browser and cleanup

Bitcoin rerun confirmed its own title and PublicNode default at a measured 1280px;
390px read-only role displayed status and zero mutation buttons. Screenshots of
empty desktop, populated Base, narrow editor, narrow 409 and actual 320 save were
viewed. Vite emitted generic client Script error messages during some fixture
reloads; all named interactions completed and their DOM outcomes were checked.
Ego space 12 was finished, the dedicated Vite process stopped, and only the
isolated rust-chain-multichannel-test MySQL container was stopped/removed. Existing
billing-platform containers were untouched. Temporary exact-export directory was
removed after the recorded checks; source and test artifacts remain in this task.
