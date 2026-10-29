// Public server catalog snapshot for local UI and browser regression fixtures.
export const rpcPresetFixture = [
  {
    "id": "base-publicnode",
    "network": "base",
    "name": "PublicNode · Base",
    "endpoint": "https://base-rpc.publicnode.com",
    "source": "https://base.publicnode.com/",
    "note": "免注册、免密钥；支持 Base ETH 直接转账和 ERC20。整块读取交易以减少请求。公共服务存在限流，实际收款以扫描状态为准。",
    "recommended": true,
    "request_interval_ms": 250
  },
  {
    "id": "base-drpc",
    "network": "base",
    "name": "dRPC · Base",
    "endpoint": "https://base.drpc.org",
    "source": "https://drpc.org/chainlist/base",
    "note": "免密钥公共备用渠道；支持整块交易读取。免费服务没有可用性保证，限流后按冷却时间重试。",
    "recommended": true,
    "request_interval_ms": 500
  },
  {
    "id": "eth-drpc",
    "network": "eth",
    "name": "dRPC · Ethereum",
    "endpoint": "https://eth.drpc.org",
    "source": "https://drpc.org/docs/node-api/public-endpoints/overview",
    "note": "免密钥；ERC20 可用。ETH 内部转账所需 trace 本次实测通过，但服务商不承诺免费权限；权限不足时扫描会停止，不会改为只识别直接转账。请以完整扫描成功为准。",
    "recommended": true,
    "request_interval_ms": 500
  },
  {
    "id": "btc-publicnode",
    "network": "btc",
    "name": "PublicNode · Bitcoin",
    "endpoint": "https://bitcoin-rpc.publicnode.com",
    "source": "https://bitcoin.publicnode.com/",
    "note": "免注册、免密钥；使用 Bitcoin Core 完整区块读取 BTC 输出。公共节点的历史范围、响应时间及限流以实际扫描结果为准。",
    "recommended": true,
    "request_interval_ms": 500
  },
  {
    "id": "tron-trongrid-public",
    "network": "tron",
    "name": "TronGrid · Tron 公共渠道",
    "endpoint": "https://api.trongrid.io",
    "source": "https://developers.tron.network/reference/rate-limits",
    "note": "可免密钥读取 TRC20 日志和完整回执；匿名请求限流更严格。需要使用自己的 TronGrid Key 时，请添加自定义渠道；暂不支持原生 TRX。",
    "recommended": true,
    "request_interval_ms": 500
  },
  {
    "id": "solana-publicnode",
    "network": "solana",
    "name": "PublicNode · Solana",
    "endpoint": "https://solana-rpc.publicnode.com",
    "source": "https://solana.publicnode.com/",
    "note": "免注册、免密钥；读取已最终确认的完整区块，支持 SOL 和经典 SPL Token。历史区块及持续吞吐受公共服务限制，不支持 Token2022。",
    "recommended": true,
    "request_interval_ms": 250
  },
  {
    "id": "base-official",
    "network": "base",
    "name": "Base 官方公共节点（低频测试）",
    "endpoint": "https://mainnet.base.org",
    "source": "https://docs.base.org/chain/network-information",
    "note": "免密钥，仅适合低频测试；本次日志查询已遇到 429。官方不建议用于生产应用，因此不列入默认主备渠道。",
    "recommended": false,
    "request_interval_ms": 1000
  },
  {
    "id": "solana-official",
    "network": "solana",
    "name": "Solana 官方公共节点（低频测试）",
    "endpoint": "https://api.mainnet-beta.solana.com",
    "source": "https://solana.com/docs/references/clusters",
    "note": "免密钥主网节点；本次完整区块请求已遇到 429。仅作低频测试，不列入默认渠道。",
    "recommended": false,
    "request_interval_ms": 1000
  }
];
