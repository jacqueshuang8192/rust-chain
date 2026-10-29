# Tron free provider qualification — 2026-10-29

## Decision
Five distinct free providers: PublicNode, TronGrid, Ankr, Alchemy, Chainstack. PublicNode is new anonymous catalog entry; TronGrid anonymous/account count once. Four account templates remain operator-configured and are not claimed healthy. QuickNode remains optional trial/paid, excluded from the free count. Do not include GetBlock only to inflate the count: the native whole-block receipt method was not confirmed in the available current documentation.

## Official primary sources
| Provider | Free access and restrictions | Native HTTP evidence |
| --- | --- | --- |
| PublicNode | https://tron.publicnode.com/ lists free mainnet HTTP https://tron-rpc.publicnode.com; no published per-project quota guarantee | Live read-only probe: genesis, USDT decimals, fresh head, block at 20 confirmations, all 496 transactions matched 496 receipts with block height/timestamp. See public-probe.json. |
| TronGrid | https://www.trongrid.io/price Basic free: 100,000 requests/day, 3 API keys. https://developers.tron.network/reference/rate-limits explicitly says not to hard-code QPS; account/project/IP/endpoints may share limits. | Existing native /wallet adapter and prior qualified public catalog; account uses TRON-PRO-API-KEY. |
| Ankr | https://www.ankr.com/docs/rpc-service/service-plans/ Freemium recurring 200M API credits/month; sign in without topping up. Node API standard/full/archive supported. https://www.ankr.com/web3-api/chains-list/tron/ shows free monthly quota, 30 req/s; quota is credits, not requests. | https://www.ankr.com/docs/rpc-service/chains/chains-api/tron/ and soliditynode-api-p4/ document /wallet/gettransactioninfobyblocknum via premium-http/tron/API_KEY. Name of URL path does not alone determine paid eligibility; account permission must be verified. |
| Alchemy | https://www.alchemy.com/pricing Free: 30M CU/month, 15 req/s; all mainnets/testnets. Per-method CU separate from request count. | https://www.alchemy.com/docs/reference/tron-api-quickstart shows /v2/KEY/wallet/getnowblock; https://www.alchemy.com/docs/reference/compute-unit-costs lists native /wallet/gettransactioninfobyblocknum and triggerconstantcontract at 20 CU. Earlier /docs/chains/tron/tron-api-quickstart URL is inaccessible; use current /docs/reference/ path. |
| Chainstack | https://chainstack.com/build-better-with-tron/ expressly includes free Developer mainnet, 3M RU/month, 25 req/s, one node, no credit card. TRON requests cost 1 RU. https://chainstack.com/pricing/ discloses extra-usage charges; do not enable paid overages. | https://docs.chainstack.com/docs/tron-tooling and /docs/tron-methods specify native /wallet and /walletsolidity, full history. Use complete console HTTPS URL, not platform management key or JSON-RPC endpoint. |

## Probe boundary
Only five low-frequency read-only requests were made to PublicNode, with one second between requests. No credentials, production configuration, purchases or on-chain payments. No retry or service-limit circumvention. The sample supports recommending a setup draft, not continuous throughput or every historical block. Account templates have documented free access, not live owned-key verification.

## Implementation
Add immutable tron-publicnode public identity at 1000ms, preserving all other identities/order and persisted pools; explicit add/save only. No migration or scanner algorithm changes. Add browser-local freeTier metadata (quota/source/date) to the four Tron accounts; compute distinct free count from supplied public catalog plus documented free accounts, excluding trial/paid and aliases. Retain existing DTO, credentials and cooldown contracts. UI reports both total/free choices and does not equate them with configured/healthy channels.

## Final source recheck
Alchemy current official pricing explicitly lists 15 req/s (300 CU/s), not 25 req/s; corrected the final UI and manual before delivery. TronGrid pricing is indexed as Basic Free/100,000 daily/three keys, while its rate-limit guide deliberately avoids fixed QPS. Chainstack methods table explicitly includes /wallet/getblockbynum, getnowblock, gettransactioninfobyblocknum and triggerconstantcontract. Ankr Tron service page confirms free REST and 200M credits; account permissions still require actual saved-key validation.
