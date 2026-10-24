# Mainnet preset research (2026-10-24)

## Sources inspected

- Requested epusdt repository pinned to `aed4a970a28d734c8a35499496604b868a24ef7f`: https://github.com/GMWalletApp/epusdt/blob/aed4a970a28d734c8a35499496604b868a24ef7f/src/model/dao/mdb_table_init.go . `seedChains`, `seedChainTokens`, `defaultRpcNodes` provide TronGrid HTTP, Ethereum PublicNode WebSocket, Solana public HTTP and stablecoin identities/6 decimals; SOL is native/9. No upstream executable was run.
- Tether issuer mainnet identities: https://tether.to/en/supported-protocols/ . Ethereum USDT `0xdac17f958d2ee523a2206206994597c13d831ec7`, Tron USDT `TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t`, Solana USDT `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` match the reference.
- Circle issuer list: https://developers.circle.com/stablecoins/usdc-contract-addresses . Ethereum USDC `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48`, Solana USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` match the reference. No retired Tron USDC preset.
- Tron endpoint/header: https://developers.tron.network/reference/select-network , https://developers.tron.network/reference/rate-limits , https://developers.tron.network/reference/gettransactioninfobyblocknum . Mainnet `https://api.trongrid.io`, `TRON-PRO-API-KEY`; quotas depend on plan. Do not hard-code rate guarantees.
- Solana clusters: https://solana.com/docs/references/clusters . Public mainnet endpoint from epusdt remains listed; public RPC is rate-limited and unsuitable as a reliability promise for full-block scanning.
- Alchemy tracing: https://www.alchemy.com/docs/reference/debug-traceblockbyhash . Ethereum HTTP API supports block hash plus callTracer/onlyTopCall=false. Use a credential URL template; the operator must supply an authorized plan/key and verify access/quota.
- Helius full blocks and mint metadata: https://www.helius.dev/docs/api-reference/rpc/http/getblock and https://www.helius.dev/docs/api-reference/rpc/http/getaccountinfo . Mainnet credential URL, finalized and jsonParsed supported. Template only, no supplied key.

The web tool returned no readable source results, so official pages and pinned GitHub source were retrieved read-only with Python urllib; source snapshots are under `/tmp/chain-presets-source-*.html`.

## Local adaptation

- Current scanner is HTTP polling, not epusdt WebSocket. Do not copy `wss://ethereum.publicnode.com` into an HTTP-only client. Use Alchemy Ethereum credential template for native ETH + ERC20; do not claim arbitrary free endpoints support callTracer.
- Keep local defaults ETH 12 blocks, Tron 20 blocks, Solana 32 finalized slots from migration 0158. Upstream uses different confirmation semantics (3/1/1), so do not silently weaken local policy.
- Node presets: Ethereum Alchemy; Tron TronGrid; Solana public for evaluation and Helius with key. Fill only endpoint/confirmations on explicit selection, preserve API-key field and admission switch. Block unresolved URL credential placeholders even if admission is off, because configured nodes still scan history.
- Asset presets: ETH/USDT/USDC; TRC20 USDT; SOL/USDT/USDC. Empty contract for native, case-preserving Base58, lowercase EVM. Require matching platform catalog entry; exclude existing mappings. Existing identity editors expose no preset control.
- All presets are draft helpers using existing audited save endpoints, revisions, permissions and cancellation guards. No migration, backend mutation, new API, automatic address, automatic save or automatic enable.

## Options and decision

The user chose explicit UI presets over database seeding. Reuse node and asset editors rather than create a bulk partial-save wizard. Keep manual inputs after applying presets so dedicated providers and future assets remain possible. Backend scanners continue validating network, mint program and decimals before readiness.
