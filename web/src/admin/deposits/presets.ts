/** Mainnet identities checked against issuers and epusdt aed4a97 on 2026-10-24.
 * Sources and adapter differences: docs/superpowers/timed-chain-deposits.md.
 * Presets populate local drafts only; never include credentials or receiving addresses.
 */
export const nativeAssets: Record<string, { symbol: string; decimals: number }> = {
  eth: { symbol: 'ETH', decimals: 18 }, base: { symbol: 'ETH', decimals: 18 },
  btc: { symbol: 'BTC', decimals: 8 }, solana: { symbol: 'SOL', decimals: 9 }
};

type NodePreset = { id: string; network: string; label: string; endpoint: string; confirmations: number; note: string; source: string };
export const nodePresets: readonly NodePreset[] = [
  { id: 'eth-alchemy', network: 'eth', label: 'Ethereum · Alchemy（需密钥）', endpoint: 'https://eth-mainnet.g.alchemy.com/v2/{API_KEY}', confirmations: 12,
    note: '将节点地址中的 {API_KEY} 替换为自己的 Alchemy 密钥。监听 ETH 需要账号支持 debug_traceBlockByHash / callTracer；保存前确认方法权限和调用额度。此密钥放在 URL 中，不填入下方 Bearer 密钥框。', source: 'https://www.alchemy.com/docs/reference/debug-traceblockbyhash' },
  { id: 'tron-trongrid', network: 'tron', label: 'Tron · TronGrid 主网', endpoint: 'https://api.trongrid.io', confirmations: 20,
    note: '在下方节点 API 密钥框填写自己的 TronGrid API Key，服务端使用 TRON-PRO-API-KEY 请求头。未配置密钥可能被限流；这里只监听 TRC20。', source: 'https://developers.tron.network/reference/select-network' },
  { id: 'solana-public', network: 'solana', label: 'Solana · 公共主网（测试用）', endpoint: 'https://api.mainnet-beta.solana.com', confirmations: 32,
    note: '这是主网公共节点，不是测试网。无需密钥，但完整区块扫描容易触发限流；持续监听建议换专用 RPC。确认深度为 finalized slot 深度，支持 SOL 和经典 SPL Token。', source: 'https://solana.com/docs/references/clusters' },
  { id: 'solana-helius', network: 'solana', label: 'Solana · Helius（需密钥）', endpoint: 'https://mainnet.helius-rpc.com/?api-key={API_KEY}', confirmations: 32,
    note: '将节点地址中的 {API_KEY} 替换为自己的 Helius 密钥。需要 finalized 完整 jsonParsed 区块及 mint 查询权限和足够额度。此密钥放在 URL 中，不填入下方 Bearer 密钥框。', source: 'https://www.helius.dev/docs/api-reference/rpc/http/getblock' }
];

type AssetPreset = { id: string; network: string; symbol: string; kind: 'native' | 'token'; token: string; decimals: number; label: string; source: string };
const tether = 'https://tether.to/en/supported-protocols/';
const circle = 'https://developers.circle.com/stablecoins/usdc-contract-addresses';
export const assetPresets: readonly AssetPreset[] = [
  { id: 'eth-native', network: 'eth', symbol: 'ETH', kind: 'native', token: '', decimals: nativeAssets.eth.decimals, label: 'ETH · Ethereum 原生币', source: 'https://ethereum.org/en/developers/docs/intro-to-ether/' },
  { id: 'eth-usdt', network: 'eth', symbol: 'USDT', kind: 'token', token: '0xdac17f958d2ee523a2206206994597c13d831ec7', decimals: 6, label: 'USDT · Ethereum ERC20', source: tether },
  { id: 'eth-usdc', network: 'eth', symbol: 'USDC', kind: 'token', token: '0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48', decimals: 6, label: 'USDC · Ethereum ERC20', source: circle },
  { id: 'base-native', network: 'base', symbol: 'ETH', kind: 'native', token: '', decimals: nativeAssets.base.decimals, label: 'ETH · Base 原生币（直接转账）', source: 'https://docs.base.org/chain/network-information' },
  { id: 'base-usdc', network: 'base', symbol: 'USDC', kind: 'token', token: '0x833589fcd6edb6e08f4c7c32d4f71b54bda02913', decimals: 6, label: 'USDC · Base ERC20', source: circle },
  { id: 'tron-usdt', network: 'tron', symbol: 'USDT', kind: 'token', token: 'TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t', decimals: 6, label: 'USDT · Tron TRC20', source: tether },
  { id: 'solana-native', network: 'solana', symbol: 'SOL', kind: 'native', token: '', decimals: nativeAssets.solana.decimals, label: 'SOL · Solana 原生币', source: 'https://solana.com/docs/core/accounts' },
  { id: 'solana-usdt', network: 'solana', symbol: 'USDT', kind: 'token', token: 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', decimals: 6, label: 'USDT · Solana SPL', source: tether },
  { id: 'solana-usdc', network: 'solana', symbol: 'USDC', kind: 'token', token: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', decimals: 6, label: 'USDC · Solana SPL', source: circle }
];

export function hasCredentialPlaceholder(value: string) {
  return /\{API_KEY\}|YOUR_API_KEY|<API_KEY>|%7BAPI_KEY%7D/i.test(value);
}
