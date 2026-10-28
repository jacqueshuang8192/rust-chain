import type { Asset, PlatformAsset, Scanner, SharedAddress } from './contracts';

export const scanGuides: Record<string, { title: string; detail: string }> = {
  eth: { title: 'Ethereum 主网 · HTTP RPC', detail: '扫描原生 ETH 与已配置 ERC20；ETH 包含内部转账，需要 callTracer。仅支持主网 chainId 1。' },
  base: { title: 'Base 主网 · HTTP RPC', detail: '原生 ETH 使用区块内的交易与成功回执，仅识别直接转账；合约内部 ETH 转账不会自动到账。节点需支持批量交易查询；ERC20 继续读取转账日志，主网 chainId 8453。ETH 与 ERC20 独立保存进度和接单状态，共用节点额度。' },
  tron: { title: 'Tron 主网 · FullNode', detail: '扫描 TRC20 转账日志及完整交易回执。TronGrid 密钥填写在节点 API 密钥框；暂不支持原生 TRX。' },
  btc: { title: 'Bitcoin 主网 · Core RPC', detail: '扫描已同步节点的完整区块与 BTC 输出。需允许读取历史区块；暂不支持其他 Bitcoin 资产。' },
  solana: { title: 'Solana 主网 · HTTP RPC', detail: '扫描 finalized 完整区块、SOL 和经典 SPL Token，校验 mint 精度。暂不支持 Token2022；持续监听建议专用节点。' }
};

export function scanStatus(scanner?: Scanner) {
  if (!scanner?.endpoint_configured) return { label: '未配置节点', detail: '先保存当前网络的节点接入配置。', ready: false };
  if (scanner.network === 'base' && scanner.streams?.length) {
    const statuses = scanner.streams.map(stream => scanStatus({ ...scanner, ...stream, streams: [] }));
    if (statuses.every(status => status.ready)) return { label: '两路均已追上区块', detail: 'ETH 与 ERC20 独立监听，按各币种对应状态判断是否接单。', ready: true };
    if (statuses.some(status => status.ready)) return { label: '部分币种可收款', detail: 'ETH 与 ERC20 独立监听，请查看下方对应分流的状态。', ready: false };
  }
  if (scanner.last_error !== null) return { label: '扫描异常', detail: '检查节点权限、调用额度、网络及币种身份；排除原因后扫描器会自动重试。', ready: false };
  if (!scanner.scan_fresh) return scanner.last_scan_at === null
    ? { label: '等待扫描确认', detail: '配置已保存，等待后台扫描器验证网络和资产并追上区块。', ready: false }
    : { label: '扫描状态已过期', detail: '超过 120 秒未成功追上区块，请检查后台扫描任务和节点连接。', ready: false };
  return { label: '已追上区块', detail: '服务端检查时处于 120 秒有效窗口；新订单仍由服务端实时校验。', ready: true };
}

export function methodBlockers(asset: Asset, scanner: Scanner | undefined, addresses: SharedAddress[], catalog: PlatformAsset[]): string[] {
  const reasons: string[] = [];
  const platform = catalog.find(row => row.symbol === asset.asset_symbol);
  if (!asset.enabled) reasons.push('监听币种已停用');
  if (!platform) reasons.push('缺少平台资产');
  else if (platform.status !== 'active' || !platform.deposit_enabled) reasons.push('平台资产充值未开启');
  if (!scanner?.enabled) reasons.push('网络已暂停接单');
  if (!addresses.some(row => row.network === asset.network && row.enabled)) reasons.push('缺少启用的收款地址');
  const stream = scanner?.network === 'base' ? scanner.streams?.find(row => row.stream === asset.asset_kind) : undefined;
  const scan = scanStatus(scanner?.network === 'base' ? stream ? { ...scanner, ...stream, streams: [] } : undefined : scanner);
  if (!scan.ready) reasons.push(scan.label);
  return reasons;
}
