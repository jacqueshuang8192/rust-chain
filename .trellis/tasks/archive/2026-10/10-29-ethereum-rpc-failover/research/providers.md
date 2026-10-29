# Ethereum RPC provider research — 2026-10-29

## Decision

The user explicitly accepts free accounts and personal API keys. No additional anonymous candidate has passed the complete scanner capability probe. Keep existing anonymous dRPC records intact, but retire its recommendation for new full-ETH configurations. Add two free-account templates using the existing custom-provider URL-key workflow; do not introduce another credential DTO or database schema.

## Official sources

- Previous archived research cites https://drpc.org/blog/are-public-rpc-endpoints-enough-for-your-needs/ for paid debug/trace policy; this page was unavailable (404/direct fetch, inaccessible via web) during the current recheck, so this run cannot newly confirm that policy. The user-reported trace HTTP 429 and lack of sustained evidence are sufficient to retire the default recommendation. A prior isolated successful response is not permission or sustained capacity.
- https://www.chainnodes.org/pricing states Core free access at 12.5M requests/month, 25 RPS, with all methods, archive and debug/trace included on all plans. https://www.chainnodes.org/docs/ethereum/debug_traceBlockByHash documents the block tracer and callTracer, and https://www.chainnodes.org/docs/ethereum/debug_traceBlockByHash supplies https://mainnet.chainnodes.org/<api-key>. Registration is through https://app.chainnodes.org/. This is the primary free-account template, not a live-tested deployed credential.
- https://solidrpc.io/docs/tracing states debug_traceBlockByHash and callTracer are available on Free, while https://solidrpc.io/docs/pricing lists 10,000 requests/day, one-unit accounting, and full archive access. https://solidrpc.io/docs supplies https://rpc.solidrpc.io/<api-key>/evm/1. Registration is through https://solidrpc.io/register. At least two block methods per Ethereum block means this allowance alone is not a sustained full-chain monitoring guarantee; expose as a limited backup.
- https://docs.tenderly.co/node-rpc/pricing explicitly says Node RPC has no free tier. https://docs.tenderly.co/node-rpc also describes keyless endpoints with 1GB/IP daily egress and 3 req/sec. The anonymous sampled trace returned HTTP 429 even though remaining egress was nonzero. Exclude it from free-account presets.
- https://www.alchemy.com/docs/reference/debug-api-quickstart limits the method to PAYG/Enterprise. The anonymous Alchemy and Blast entrypoints reject non-core methods. Exclude as free full-ETH recommendations; preserve the existing optional Alchemy account template with accurate scope.

## Bounded probes

`probe_ethereum.py`, `ethereum-probes.json`, and `ethereum-followup.json` contain the endpoint/date and safe results. All requests were unauthenticated reads, with 1.5 seconds between requests per endpoint, at most two candidates concurrently, 15-second timeouts and 32MB response limits. No real user key or production state was accessed.

- Tenderly: mainnet identity and blocks pass; debug_traceBlockByHash returns 429.
- OnFinality: mainnet identity passes; block read returns 429.
- Alchemy public / Blast public: identity and blocks pass; trace returns 401 (core EVM methods only).
- MeowRPC: identity and blocks pass; trace is unavailable (-32601).
- LlamaRPC: HTTP 525. Merkle: HTTP 403. 0xRPC candidate path: HTTP 404. Do not register these as working defaults.

No follow-up retries were used to evade rate limits. Public endpoint failure does not prove a keyed tier fails. New account templates are documentation-qualified only; without user-owned keys, exact response shape, historical access, quota and continuous catch-up remain unverified. In particular, preserve required txHash/complete call-tree checks; an illustrative provider documentation example is not a reason to weaken evidence validation.

## Integration constraints

- Reuse nodePresets and custom provider URL placeholders. Selecting a new template uses fresh blank key slots so old provider secrets cannot be sent to another service.
- Existing drafts are unchanged unless explicitly replaced; adding a template appends at the next priority. Keep 409/reason/dirty guards and no automatic accepting-orders switch.
- Preserve server preset identity for old eth-drpc records; only recommended/note changes. Same-preset saves retain durable failures and cursors.
- No new migration, scanner semantics, quota bypass or automatic provider activation. Full runtime validation is authoritative after operators obtain and save their keys.
