/** Mainnet identities checked against issuers and epusdt aed4a97 on 2026-10-24.
 * Sources and adapter differences: docs/superpowers/timed-chain-deposits.md.
 * Presets populate local drafts only; never include credentials or receiving addresses.
 */
export const nativeAssets: Record<string, { symbol: string; decimals: number }> = {
  eth: { symbol: 'ETH', decimals: 18 }, base: { symbol: 'ETH', decimals: 18 },
  btc: { symbol: 'BTC', decimals: 8 }, solana: { symbol: 'SOL', decimals: 9 }
};

export type NodePreset = {
  id: string; network: string; label: string; endpoint: string; confirmations: number; note: string; source: string;
  legacy?: boolean;
  account?: {
    provider: string; name: string; signup: string; interval: number;
    plan: '免费账号' | '免费备用' | '需付费权限' | '试用 / 付费' | '按账号套餐';
    credential: 'url-key' | 'tron-header' | 'endpoint';
    setup: string;
    freeTier?: { quota: string; source: string; checkedOn: string };
    publicPresetIds?: readonly string[];
  };
};
const depth: Record<string, number> = { eth: 12, base: 12, tron: 20, btc: 6, solana: 32 };
const networkNames: Record<string, string> = { eth: 'Ethereum', base: 'Base', tron: 'Tron', btc: 'Bitcoin', solana: 'Solana' };
const scanRequirements: Record<string, string> = {
  eth: '需要主网历史区块、完整 callTracer 及 ERC20 日志；不降级为只识别直接转账。',
  base: '需要主网区块、交易批量读取、成功回执和 ERC20 日志；支持 ETH 直接转账与 USDC。',
  tron: '必须选择原生 HTTP / Wallet API，支持创世块、合约常量调用和整块交易回执；只监听 TRC20。',
  btc: '需要 Bitcoin Core 主网 RPC：getblockchaininfo、getblockhash 和 getblock verbosity=2；浏览器 REST API 不能替代。',
  solana: '需要 finalized 完整 jsonParsed 区块、交易版本上限 1 和 mint 查询；支持 SOL 与经典 SPL Token。'
};
function chainstack(network: string): NodePreset {
  const name = networkNames[network];
  const plan = network === 'eth' ? '需付费权限' : network === 'tron' ? '免费账号' : '按账号套餐';
  return { id: `${network}-chainstack`, network, label: `${name} · Chainstack（${plan}）`, endpoint: '', confirmations: depth[network],
    account: { provider: 'chainstack', name: `Chainstack · ${name}`, signup: 'https://console.chainstack.com/', interval: 500, plan, credential: 'endpoint',
      setup: '从 Chainstack 控制台复制该主网的完整 HTTPS 节点地址（含节点访问令牌），填入节点基础地址；不要填平台管理 API Key。Tron 地址不要重复包含 /wallet 方法路径。',
      ...(network === 'tron' ? { freeTier: { quota: 'Developer 免费计划：每月 300 万 RU、25 次/秒、1 个节点，无需信用卡。Tron 请求按 RU 计费；不要开通付费超额用量。', source: 'https://chainstack.com/build-better-with-tron/', checkedOn: '2026-10-29' } } : {}) },
    note: `${scanRequirements[network]} ${network === 'eth' ? '官方要求付费计划的 Global Node 才能使用 debug/trace。' : '节点和方法的免费额度、历史范围及套餐权限需在控制台核对。'} 模板只核对文档，须等待完整扫描成功。`,
    source: network === 'eth' ? 'https://docs.chainstack.com/docs/debug-and-trace-apis' : `https://docs.chainstack.com/docs/${network === 'btc' ? 'bitcoin' : network}-tooling` };
}
function ankr(network: 'tron' | 'btc' | 'solana'): NodePreset {
  const name = networkNames[network];
  const plan = network === 'tron' ? '免费账号' : '按账号套餐';
  return { id: `${network}-ankr`, network, label: `${name} · Ankr（${plan}）`, endpoint: `https://rpc.ankr.com/${network === 'tron' ? 'premium-http/tron' : network}/{API_KEY}`, confirmations: depth[network],
    account: { provider: 'ankr', name: `Ankr · ${name}`, signup: 'https://www.ankr.com/rpc/', interval: 1000, plan, credential: 'url-key',
      setup: '创建该主网的 RPC API Key，填入下方独立凭据。确认账户开放所需方法和历史数据；同一账号的多个 Key 可能共享额度。',
      ...(network === 'tron' ? { freeTier: { quota: 'Freemium 免费计划：每月 2 亿 API credits，Tron 页面标示 30 次/秒。credits 不等于请求次数；选择原生 HTTP 地址，账号 Key 不需要充值。', source: 'https://www.ankr.com/docs/rpc-service/service-plans/', checkedOn: '2026-10-29' } } : {}) },
    note: `${scanRequirements[network]} 公开地址、普通 RPC 与原生 HTTP 的权限可能不同，以账号套餐和完整扫描为准。`,
    source: `https://www.ankr.com/docs/rpc-service/chains/chains-api/${network}/` };
}
function quicknode(network: 'tron' | 'solana'): NodePreset {
  const name = networkNames[network];
  return { id: `${network}-quicknode`, network, label: `${name} · QuickNode（试用 / 付费）`, endpoint: '', confirmations: depth[network],
    account: { provider: 'quicknode', name: `QuickNode · ${name}`, signup: 'https://dashboard.quicknode.com/', interval: 1000, plan: '试用 / 付费', credential: 'endpoint',
      setup: `在控制台创建 ${name} 主网节点，复制含访问令牌的完整 HTTP 地址；试用到期和方法权限需自行核对。${network === 'tron' ? '选择原生 HTTP，基础地址不要包含 /wallet/get… 方法路径。' : ''} 下方 API Key 留空。` },
    note: `${scanRequirements[network]} 试用不等于永久免费；未使用你的节点验证，配额和实际支持版本以完整扫描为准。`,
    source: `https://www.quicknode.com/docs/${network}/${network === 'tron' ? 'wallet-gettransactioninfobyblocknum' : 'getBlock'}` };
}

/** Account entries are documentation-qualified setup drafts, never enabled or marked healthy automatically. */
export const nodePresets: readonly NodePreset[] = [
  { id: 'eth-chainnodes', network: 'eth', label: 'Ethereum · Chainnodes（免费账号）', endpoint: 'https://mainnet.chainnodes.org/{API_KEY}', confirmations: 12,
    account: { provider: 'chainnodes', name: 'Chainnodes · Ethereum', signup: 'https://app.chainnodes.org/', interval: 500, plan: '免费账号', credential: 'url-key', setup: '注册后填入自己的 RPC API Key；系统按凭据填入节点 URL。' },
    note: '需注册并申请自己的 API Key。官方 Core 免费计划包含 debug/trace；具体额度以账号控制台为准。未使用你的 Key 验证，须等待完整扫描成功。', source: 'https://www.chainnodes.org/docs/ethereum/debug_traceBlockByHash' },
  { id: 'eth-solidrpc', network: 'eth', label: 'Ethereum · SolidRPC（免费备用）', endpoint: 'https://rpc.solidrpc.io/{API_KEY}/evm/1', confirmations: 12,
    account: { provider: 'solidrpc', name: 'SolidRPC · Ethereum', signup: 'https://solidrpc.io/register', interval: 1000, plan: '免费备用', credential: 'url-key', setup: '填写免费账号的 RPC API Key，不要填管理凭据。' },
    note: '免费层文档开放 debug/trace，日额度较小，适合短时备用；不保证单独持续追块。权限、配额及完整 callTracer 证据以完整扫描结果为准。', source: 'https://solidrpc.io/docs/tracing' },
  { id: 'eth-google', network: 'eth', label: 'Ethereum · Google Cloud（免费账号）', endpoint: '', confirmations: 12,
    account: { provider: 'google', name: 'Google Cloud · Ethereum', signup: 'https://console.cloud.google.com/blockchain/rpc', interval: 1000, plan: '免费账号', credential: 'endpoint', setup: '在 Google Cloud 项目中启用 Blockchain RPC，从控制台复制 Ethereum Mainnet 的完整 JSON-RPC URL（含项目和 API Key）；下方 API Key 留空。' },
    note: '官方列出 debug_traceBlockByHash 与免费用量限制；trace 按请求倍数消耗配额，不能把普通请求额度当作 trace 次数。核对账号权限和历史范围，须等待完整扫描成功。', source: 'https://docs.cloud.google.com/blockchain-rpc/docs/rpc-api' },
  { id: 'eth-alchemy', network: 'eth', label: 'Ethereum · Alchemy（需付费权限）', endpoint: 'https://eth-mainnet.g.alchemy.com/v2/{API_KEY}', confirmations: 12, legacy: true,
    account: { provider: 'alchemy', name: 'Alchemy · Ethereum', signup: 'https://dashboard.alchemy.com/', interval: 1000, plan: '需付费权限', credential: 'url-key', setup: '创建 Ethereum Mainnet 应用并开通 Debug API 权限，填入应用 API Key；普通免费 Key 不能作为完整 ETH 监听渠道。' },
    note: '监听 ETH 所需 Debug API 要求 Pay As You Go 或 Enterprise。这里只提供可选模板，不自动开通或购买。完整 trace 和历史数据权限需实际验证。', source: 'https://www.alchemy.com/docs/reference/debug-api-quickstart' },
  chainstack('eth'),
  { id: 'base-chainnodes', network: 'base', label: 'Base · Chainnodes（免费账号）', endpoint: 'https://base-mainnet.chainnodes.org/{API_KEY}', confirmations: 12,
    account: { provider: 'chainnodes', name: 'Chainnodes · Base', signup: 'https://app.chainnodes.org/', interval: 500, plan: '免费账号', credential: 'url-key', setup: '创建 Base 主网 RPC Key，并填入独立凭据；与其他网络共用账号时留意共享额度。' },
    note: `${scanRequirements.base} 免费 Core 计划有请求限额；本渠道使用现有自定义节点扫描方式，实际持续吞吐需验证。`, source: 'https://www.chainnodes.org/docs/base/eth_getBlockByNumber' },
  { id: 'base-alchemy', network: 'base', label: 'Base · Alchemy（免费账号）', endpoint: 'https://base-mainnet.g.alchemy.com/v2/{API_KEY}', confirmations: 12,
    account: { provider: 'alchemy', name: 'Alchemy · Base', signup: 'https://dashboard.alchemy.com/', interval: 1000, plan: '免费账号', credential: 'url-key', setup: '创建 Base Mainnet 应用，填入该应用的 API Key。Base 直接转账监听不要求 Debug API。' },
    note: `${scanRequirements.base} 免费计划仍有 CU、批量请求及频率限制；以完整扫描状态为准。`, source: 'https://www.alchemy.com/docs/chains/base/base-api-quickstart' },
  chainstack('base'),
  { id: 'tron-trongrid', network: 'tron', label: 'Tron · TronGrid 主网', endpoint: 'https://api.trongrid.io', confirmations: 20, legacy: true,
    account: { provider: 'trongrid', name: 'TronGrid · Tron', signup: 'https://www.trongrid.io/', interval: 500, plan: '免费账号', credential: 'tron-header', publicPresetIds: ['tron-trongrid-public'], setup: '填入自己的 TronGrid API Key，系统通过 TRON-PRO-API-KEY 请求头发送。与免密钥 TronGrid 属于同一服务商，不重复计数。',
      freeTier: { quota: 'Basic 免费计划：每天 10 万次请求、最多 3 个 API Key。Key 不会增加账户总额度；频率限制以控制台与接口响应为准。', source: 'https://www.trongrid.io/price', checkedOn: '2026-10-29' } },
    note: 'TronGrid 免费账号仍有每日和频率限制；这里监听 TRC20，必须读取完整交易回执，不能仅凭转账调用参数入账。', source: 'https://developers.tron.network/reference/rate-limits' },
  ankr('tron'),
  { id: 'tron-alchemy', network: 'tron', label: 'Tron · Alchemy（免费账号）', endpoint: 'https://tron-mainnet.g.alchemy.com/v2/{API_KEY}', confirmations: 20,
    account: { provider: 'alchemy', name: 'Alchemy · Tron', signup: 'https://dashboard.alchemy.com/', interval: 1000, plan: '免费账号', credential: 'url-key', setup: '创建 Tron Mainnet 应用，填入应用 API Key。必须开放原生 HTTP / Wallet API，不是只支持 EVM 兼容 JSON-RPC 的地址。',
      freeTier: { quota: 'Free 免费计划：每月 3000 万 CU、15 次/秒。不同方法消耗不同 CU，不能按请求次数估算额度；不要切换付费套餐。', source: 'https://www.alchemy.com/pricing', checkedOn: '2026-10-29' } },
    note: `${scanRequirements.tron} 官方免费计划提供主网访问；须核对账号方法权限，模板不代表已验证到账或持续吞吐。`, source: 'https://www.alchemy.com/docs/reference/tron-api-quickstart' },
  chainstack('tron'), quicknode('tron'),
  { id: 'btc-getblock', network: 'btc', label: 'Bitcoin · GetBlock（按账号套餐）', endpoint: '', confirmations: 6,
    account: { provider: 'getblock', name: 'GetBlock · Bitcoin', signup: 'https://account.getblock.io/', interval: 1000, plan: '按账号套餐', credential: 'endpoint', setup: '在控制台为 Bitcoin Mainnet 创建 JSON-RPC 节点，复制含 Access Token 的完整区域节点地址；不要选择 Blockbook / REST，也不要填账户管理 API Key。下方 API Key 留空。' },
    note: `${scanRequirements.btc} 免费/共享配额与方法权限需在控制台确认；逐块完整输出和历史数据必须可用。`, source: 'https://docs.getblock.io/api-reference/bitcoin-btc' },
  ankr('btc'), chainstack('btc'),
  { id: 'btc-drpc-account', network: 'btc', label: 'Bitcoin · dRPC（按账号套餐）', endpoint: '', confirmations: 6,
    account: { provider: 'drpc', name: 'dRPC · Bitcoin', signup: 'https://drpc.org/login', interval: 1000, plan: '按账号套餐', credential: 'endpoint', setup: '在 dRPC 控制台选择 Bitcoin Mainnet，复制含访问 Key 的完整 HTTP RPC 地址；核对历史区块和完整交易权限。下方 API Key 留空。免密钥地址本次返回 403，不作为默认节点。' },
    note: `${scanRequirements.btc} 账号额度与高级方法权限以控制台为准，不能据公开文档推定你的 Key 已可用。`, source: 'https://drpc.org/docs/bitcoin-api/blocksinfo/getblock' },
  { id: 'solana-public', network: 'solana', label: 'Solana · 公共主网（测试用）', endpoint: 'https://api.mainnet-beta.solana.com', confirmations: 32,
    note: '这是主网公共节点，不是测试网。无需密钥，但完整区块扫描容易触发限流；不计入推荐服务商数量。', source: 'https://solana.com/docs/references/clusters' },
  { id: 'solana-helius', network: 'solana', label: 'Solana · Helius（免费账号）', endpoint: 'https://mainnet.helius-rpc.com/?api-key={API_KEY}', confirmations: 32, legacy: true,
    account: { provider: 'helius', name: 'Helius · Solana', signup: 'https://dashboard.helius.dev/', interval: 1000, plan: '免费账号', credential: 'url-key', setup: '创建 Mainnet API Key 并填入独立凭据，系统编码后放入 URL；不要额外填写 Bearer 凭据。' },
    note: `${scanRequirements.solana} 免费额度与历史归档、getBlock 频率限制需核对；持续追块以实际扫描为准。`, source: 'https://www.helius.dev/docs/api-reference/rpc/http/getblock' },
  ankr('solana'), chainstack('solana'), quicknode('solana')
];

/** Count distinct documented account providers plus recommended public profiles; credentials are not channels. */
export function providerChoiceCount(network: string, publicPresets: readonly { id: string; network: string; recommended: boolean }[], freeOnly = false) {
  const accounts = nodePresets.filter(row => row.network === network && row.account && (!freeOnly || row.account.freeTier));
  const publicAliases = new Set(accounts.flatMap(row => row.account?.publicPresetIds ?? []));
  return new Set(accounts.map(row => row.account!.provider)).size
    + publicPresets.filter(row => row.network === network && row.recommended && !publicAliases.has(row.id)).length;
}

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
