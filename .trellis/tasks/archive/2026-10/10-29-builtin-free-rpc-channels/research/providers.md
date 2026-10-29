# Public RPC research — 2026-10-29

## Sources

- https://publicnode.com/ and https://base.publicnode.com/ publish Allnodes PublicNode free HTTPS/WSS endpoints.
- https://ethereum.publicnode.com/ , https://bitcoin.publicnode.com/ , https://solana.publicnode.com/ identify supported mainnets.
- https://docs.base.org/chain/network-information and https://docs.base.org/base-chain/api-reference/rpc-overview identify mainnet.base.org, HTTP-only public access and public rate limits.
- https://drpc.org/blog/free-tier-update/ describes unauthenticated public endpoints. https://drpc.org/docs/node-api/public-endpoints/overview and https://drpc.org/blog/are-public-rpc-endpoints-enough-for-your-needs/ describe best-effort public service. The latter says free-tier debug/trace access is not guaranteed and belongs to paid service tiers. A successful probe is not an entitlement promise.
- https://drpc.org/docs/ethereum-api lists eth.drpc.org; https://drpc.org/chainlist/base lists base.drpc.org.
- https://developers.tron.network/reference/select-network and https://developers.tron.network/reference/rate-limits document TronGrid mainnet, API-key headers and stricter anonymous rate limits.
- https://solana.com/docs/references/clusters documents public mainnet limits. https://solana.com/docs/rpc/http/getblock documents parsed complete blocks and maxSupportedTransactionVersion.
- https://github.com/solana-foundation/solana-improvement-documents/pull/383 specifies V1 transactions. https://github.com/anza-xyz/agave/pull/11787 adds V1 RPC support. Preserve parsed System/legacy SPL instruction semantics; do not infer support for Token2022 or unknown future versions.

## Bounded probes

All calls were unauthenticated public reads with at least 1.1 seconds between calls per endpoint, at most three concurrent endpoint checks, 18–20 second timeout and bounded response bodies. No user key, production database or transfer was used. These are compatibility samples, not sustained-load or SLA tests. Sources and JSON probe results are retained in this directory.

- Base PublicNode: correct chain ID; a full block of 334 transaction objects in one request; USDC decimals/logs and matching receipt succeeded.
- Base dRPC: full block of 258 transactions, USDC and receipt checks succeeded.
- Base official: identity/full block/decimals succeeded but getLogs returned HTTP 429. Do not preselect it as a default or advertise it as unlimited.
- Ethereum PublicNode: normal block/log/receipt methods succeeded; both debug trace block methods were unavailable. trace_block rejected even latest with a personal-token requirement. Exclude it from complete Ethereum defaults.
- Ethereum dRPC: 565 complete callTracer results for a 565-transaction block, normal identity/logs/receipt methods succeeded. Conditional default only: retain full Ethereum evidence and fail closed if trace becomes unavailable. Disclose that published free-tier policy does not guarantee trace permissions; do not downgrade to direct-only ETH.
- Ethereum LlamaRPC returned HTTP 525; the sampled BlockPI public URL returned HTTP 404. Exclude both.
- Bitcoin PublicNode: synchronized mainnet and a full verbose block with 3025 decoded transactions succeeded.
- TronGrid: mainnet genesis, current and confirmed block, all 417 transaction receipt IDs and USDT decimals succeeded without a key.
- Solana PublicNode: legacy version limit 0 failed because current blocks contain V1. Repeating with supported version 1 returned full parsed metadata/account keys for legacy/V0/V1. Complete follow-up block/time/USDC mint validation succeeded.
- Solana official: its V1 complete-block request returned HTTP 429. Retain only as a clearly limited optional template, not a default.

A Python-default User-Agent was rejected for follow-up requests; the same descriptive capability-client identity used by the original probes was accepted. This was one bounded retry, not a repeated load test. Production-client compatibility is checked separately.

## Base asset preset reference

- https://developers.circle.com/stablecoins/usdc-contract-addresses contains the native Base USDC contract `0x833589fcd6edb6e08f4c7c32d4f71b54bda02913` (rechecked 2026-10-29). The RPC decimals check returned 6. The editor only fills this explicit identity as a draft.
