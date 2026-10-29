# Ethereum free-account RPC channel templates

## Goal

Offer additional Ethereum monitoring channels after the anonymous dRPC trace request was rate-limited, without weakening internal ETH transfer evidence or overwriting existing configuration.

## Confirmed requirements

- The user accepts free provider accounts and personal API keys.
- Add Chainnodes and SolidRPC templates, official application/documentation links, quota/permission notes, and separate secret input. Use the existing custom-provider URL placeholder and encrypted key storage.
- New Ethereum setup starts with an unsaved Chainnodes template requiring a key; existing configurations are preserved. Adding templates appends backup channels and never toggles accepting new deposits.
- Anonymous dRPC remains recognized for existing configurations, with a truthful limitation note, but is no longer a recommended Ethereum default.
- Template replacement clears the old credential draft/slots. Same saved custom channel, blank fields, cooldown and progress preservation remain intact.
- No additional anonymous provider is claimed compatible; free-account templates are documentation-qualified, with live compatibility still requiring an owned key.

## Acceptance criteria

- [x] Official-source findings and bounded compatibility results are recorded in research/providers.md.
- [x] New templates, key-required validation and official links work in the real configuration editor.
- [x] Add/cancel/409 and credential replacement preserve existing unrelated channels, revisions and credentials.
- [x] Existing public defaults for Base/Tron/Bitcoin/Solana and dRPC identity preservation regressions pass.
- [x] Closest Rust/catalog/provider tests, Web quality checks and responsive browser checks run with results recorded.
- [x] Feature contract, operations guide and PROGRESS.md are current.

## Decision

Reuse the existing keyed custom-channel templates instead of extending the anonymous preset DTO/schema. This keeps the same encrypted storage, audited save, durable cooldown and failover path. Chainnodes is the initial free-account draft based on its published all-method Free plan; SolidRPC is an explicit limited backup. No node is marked healthy until complete scanning succeeds.

## Plan

1. Complete research and preserve safe probe evidence.
2. Add templates and draft-only form guidance/actions; retire dRPC recommendation without deleting stored identity.
3. Run focused financial/configuration regression and frontend/browser checks; document results and scope.

## Out of scope

Account creation, paid purchases, production configuration/deployment, bypassing rate limits, direct-only Ethereum semantics, new networks, unrelated agent-management work.

## Research references

- research/providers.md and the accompanying bounded probe JSON.
