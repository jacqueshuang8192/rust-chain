# Provider qualification — 2026-10-29

## Scope and evidence
Five distinct providers per network, preserving existing complete evidence. Official documentation qualifies account templates only; no owned keys or production were accessed. Low-frequency public probes are recorded separately. Do not count multiple keys or one provider public/account variants twice.

## Final provider matrix (account entries require operator credentials)
| Network | Five distinct providers | Qualification boundary |
| --- | --- | --- |
| Ethereum | Chainnodes, SolidRPC, Google Cloud, Alchemy, Chainstack | First three document free/account access; Alchemy and Chainstack require paid trace permissions. No account keys tested. |
| Base | PublicNode, dRPC, Chainnodes, Alchemy, Chainstack | First two retain existing anonymous profiles. New account channels use existing custom batch reads. |
| Tron | TronGrid, Ankr, Alchemy, Chainstack, QuickNode | Native HTTP /wallet methods required. TronGrid public/account counted once; QuickNode trial/paid, others subject to account plan. |
| Bitcoin | PublicNode, GetBlock, Ankr, Chainstack, dRPC | Bitcoin Core RPC only, verbosity 2 required. GetBlock and dRPC URLs copied from console; no guessed regional endpoint or query key. |
| Solana | PublicNode, Helius, Ankr, Chainstack, QuickNode | Full finalized jsonParsed block and version ceiling 1 required; actual account compatibility remains untested. |

The user previously accepted free accounts and personal keys. An optional question on trial/paid templates has not received a reply. We add only clearly labelled optional setup entries; no paid subscription/account is created, purchased, enabled by default or used. This is not five anonymous/free always-on channels.

## Official references inspected
- https://www.chainnodes.org/pricing — free Core tier, 12.5M requests/month and 25 RPS. Existing trace reference remains relevant.
- https://docs.chainstack.com/docs/debug-and-trace-apis — Ethereum debug/trace explicitly requires paid plan and Global Node. Do not label as free.
- https://docs.chainstack.com/docs/authentication-methods-for-different-scenarios — node token goes in URL; platform API bearer keys are NOT node credentials.
- https://docs.chainstack.com/docs/tron-tooling — single endpoint exposes /wallet and /walletsolidity; full history, native HTTP required by scanner.
- https://docs.chainstack.com/docs/bitcoin-tooling — Bitcoin Core RPC tooling reference.
- https://docs.cloud.google.com/blockchain-rpc/docs/rpc-api?hl=en — Ethereum mainnet Erigon supports debug_traceBlockByHash, weighted request accounting and project quotas documented separately; no fixed throughput promise is inferred. Account entitlement still needs operator verification.
- https://docs.cloud.google.com/blockchain-rpc/docs/quickstart — copy JSON-RPC endpoint including key from console; do not invent project-specific URL.
- https://www.ankr.com/docs/rpc-service/chains/chains-api/tron/ — FullNode methods include all scanner methods.
- https://www.ankr.com/docs/rpc-service/chains/chains-api/tron/soliditynode-api-p4/ — despite title, example is POST https://rpc.ankr.com/premium-http/tron/YOUR_ANKR_API_KEY/wallet/gettransactioninfobyblocknum.
- https://drpc.org/docs/tron-api — public https://tron.drpc.org plus HTTP method list. Individual receipt-method link returns 404; probe before including as working anonymous profile.
- https://drpc.org/docs/bitcoin-api/blocksinfo/getblock — public https://bitcoin.drpc.org; verbosity 2 documented.
- https://getblock.io/pricing/ — free shared and paid plans, limits vary, no guaranteed capacity.
- https://docs.nownodes.io/trx/mainnet/node-wallet-solidity/node-wallet-solidity/ — requires api-key header, incompatible with existing Tron header default. Exclude unless necessary; do not silently put key in unsupported query parameter.

## Configuration constraints discovered
- Current custom providers support URL {API_KEY} with encoding; otherwise Tron uses TRON-PRO-API-KEY, other networks Bearer.
- Tron appends wallet paths to base path and preserves query string.
- Account template metadata is frontend-local; secrets remain separate encrypted credentials. No migration needed when compatible URL/header format exists.
- Dashboard-specific URL templates must start empty and require explicit address, including when replacing a saved provider. Never retain old provider address after selecting a new empty template.
- Existing anonymous presets are immutable identities; only new public profiles that pass full sampled capability can be recommended.

## Additional final sources and correction notes
- https://www.quicknode.com/docs/tron/wallet-gettransactioninfobyblocknum — official POST /wallet/gettransactioninfobyblocknum, complete array receipts; source includes related getblockbynum and triggerconstantcontract methods.
- https://www.alchemy.com/docs/chains/tron/tron-api-quickstart — native HTTP Wallet API and Tron mainnet URL. https://www.alchemy.com/docs/chains/tron/tron-api-endpoints/get-transaction-info-by-block-num documents complete block receipt method.
- https://www.ankr.com/docs/rpc-service/chains/chains-api/btc/ — official Bitcoin Core guide, HTTPS rpc.ankr.com/btc/API_KEY. Use btc, not the initially guessed bitcoin docs slug.
- https://www.chainnodes.org/docs/base/eth_getBlockByNumber — official Base URL, full block and batch methods. Replaces the guessed overview link.
- https://docs.getblock.io/api-reference/bitcoin-btc/btc_getblock — current API reference shows regional shared endpoints; therefore copy the full operator-generated mainnet JSON-RPC URL rather than invent a fixed regional token format.
- https://cloud.google.com/blockchain-rpc/docs/rpc-api and https://cloud.google.com/blockchain-rpc/docs/quotas-and-limits — documented trace method and multiplier. This qualifies a setup template, not a particular operator entitlement or sustained trace throughput.
- https://docs.chainstack.com/docs/debug-and-trace-apis — paid Global Node requirement confirmed. Chainstack node endpoint auth docs distinguish node tokens from platform API credentials.
- Helius, QuickNode and Chainstack Solana getBlock references are linked in each template. Unknown supported transaction version/archival permissions fail closed at runtime; no response validator is weakened to qualify a provider.

## Public probe results and limits
`public-probes-initial.json` records one unauthenticated call to each public dRPC Bitcoin/Tron endpoint, both HTTP 403 from this environment. Do not classify them as ready anonymous presets or claim that paid/account access also fails. No retry/rotation to bypass limits. No real user API keys, paid accounts, production database or on-chain payments were accessed.

## Implementation plan outcome
Existing anonymous identities/defaults remain unchanged. Account templates are frontend-local and emit the existing custom-provider DTO only. Templates selected on a saved row reset all key drafts and require a fresh endpoint; explicit blank preservation remains available only for unchanged saved channels. Full endpoint providers deliberately start blank, and operators use the supplied console link. New template IDs and validation flags never enter the server payload. All-five runtime scheduling already existed and is unchanged.
