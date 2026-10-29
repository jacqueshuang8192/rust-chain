# Implementation design

## Configuration and persistence

Expose a server-owned catalog in DepositSettings.rpc_presets. Each preset has a stable ID, network, public endpoint, source URL, note, recommended flag, request interval and internal Base full-block profile. No credentials in the catalog.

Add nullable provider preset_id in immutable migration 0168; existing custom providers remain null. SaveScannerProvider accepts optional preset_id: absent preserves an existing preset for blank saves; empty explicitly selects custom; a nonempty known ID resolves a matching network endpoint server-side. A supplied custom URL clears old preset metadata. Reject unknown/network-mismatched IDs and credentials on free presets. A free preset owns one anonymous key slot and must never send a preserved custom secret to a public host. Retaining the same preset must preserve ciphertext, cooldowns, learned cap and key identities. Explicit replacement follows existing reset rules. Legacy secret replacement clears preset metadata. Audit includes the selected preset ID.

Defaults are prepared as local drafts on first setup, and an append action adds missing recommended presets to existing drafts. Use the normal permission/revision/reason-confirmed save. No startup SQL writes, implicit runtime fallback, auto-enabled deposits, discarded custom channels or anonymous traffic to unused networks.

## Runtime

A selected Base full-block preset reads eth_getBlockByNumber(height,true), then reuses exact existing transaction and receipt validation. Existing custom/Nodies channels retain hash-list plus adaptive transaction batches. Only the hash-list path has the small native-round batch cap. All HTTP calls retain the durable gate/cooldowns and complete checkpoint validation.

Ethereum retains callTracer and internal-transfer semantics, including when a public provider denies permission. Never fall back to direct-only evidence. Solana requests and explicitly accepts legacy/V0/V1 parsed transactions; unknown versions fail closed and classic SPL ownership checks remain unchanged.

## Admin

Pass catalog through strict response parsing. Preselect suitable no-key defaults for empty configuration. Existing pools can append missing defaults without replacing custom providers. Public rows show identity, official documentation, capability/limit notes and no API-key inputs; custom rows retain key pools/templates. Display source on saved provider status; do not label a merely configured public RPC healthy. Preserve discard, permission, busy, reason and revision-conflict behavior and responsive layouts.

## Verification

Real isolated MySQL: migration preserves custom rows/key secrets/cooldowns/cursors, repeat migration, server catalog resolution, cross-network and unknown rejection, preset-to-custom lifecycle, blank-save preservation, free-host credential isolation, audit rollback/version fencing. Scanner mocks: full-block versus legacy method selection, receipt identity/failure and malformed transactions, Ethereum traces unchanged, Solana legacy/V0/V1 and unknown-version rejection. Real Admin tests: empty defaults, append/dedup, custom preservation, no-key save, preset changes and failed/conflicting saves. Browser fixtures exercise all networks and narrow dialogs; run required Rust and Web gates and report existing blockers separately.
