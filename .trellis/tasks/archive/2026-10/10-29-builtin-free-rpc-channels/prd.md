# Built-in free RPC channels and deposit configuration

## Goal

Let administrators configure the existing five deposit networks without finding RPC URLs or obtaining API keys, while preserving current deposit evidence, availability and audit guarantees.

## Confirmed requirements

- User explicitly chose all existing networks: Ethereum, Base, Tron, Bitcoin and Solana; prioritize Base ETH/USDC.
- Research official free/public RPC offerings and verify actual scanner-required methods at low request volume.
- Provide suitable built-in default channels and a clearer Admin configuration flow, preserving custom providers and credentials.
- Prefer no-signup/no-key channels. Label any provider requiring registration or restricted methods accurately.
- Reduce unnecessary Base transaction requests when verified channels can return complete blocks.
- Defaults must not enable deposits, alter addresses/assets, erase cooldowns or skip cursors.

## Acceptance criteria

- [x] Official-source research and sanitized low-volume capability probes recorded for all five networks.
- [x] Compatible no-key defaults are available without typing RPC URLs; unsupported defaults are not presented as ready.
- [x] Existing custom settings, key storage, revision/audit, failover and financial evidence remain intact.
- [x] Administrator can distinguish free built-ins, custom providers, readiness and limits, with complete edit/save behavior.
- [x] Base standard RPC can use verified full transaction blocks without weakening validation or breaking restricted existing endpoints.
- [x] Database/runtime, Web behavior and relevant static/build checks pass, with local browser verification; existing JS budget overflow remains explicitly reported (no threshold relaxation).
- [x] Deployment, remaining public-service limits and progress are documented.

## Decisions

- Server-owned catalog and explicit audited save; defaults are prefilled local drafts, not automatic production configuration writes.
- Base defaults: PublicNode + dRPC; Bitcoin/Solana: PublicNode; Tron: TronGrid; Ethereum: dRPC with explicit conditional trace-permission note.
- Keep limited Base/Solana official endpoints optional. Exclude failed and incompatible alternatives.
- Preserve Ethereum internal transfers; permission failure blocks progress instead of silently downgrading evidence.
- Add Solana V1 parsed-transaction support based on official protocol/RPC work and actual compatible responses.
- Use migration 0168 only for nullable preset identity, preserving existing data and default custom behavior.
- See info.md and research/providers.md for exact lifecycle and limitations.

## Out of scope

Buying plans, registering provider accounts, using user secrets, modifying production settings, enabling deposit admission, moving money, unrelated agent-management edits, or promising public RPC throughput/SLA.

## Plan

1. Inspect existing contracts and official provider documentation; probe bounded public reads.
2. Implement server-owned presets/defaults and compatible Base full-block reads.
3. Improve Admin preset selection, defaults and capability messaging.
4. Verify database safety, scanner behavior, Web interaction and deployment; record and deliver.
