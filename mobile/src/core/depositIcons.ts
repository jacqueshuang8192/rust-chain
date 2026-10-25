const assetFiles: Record<string, string> = { ETH: 'eth', TRX: 'tron', SOL: 'solana', BTC: 'bitcoin', USDT: 'usdt', USDC: 'usdc' }
const networkFiles: Record<string, string> = { eth: 'eth', base: 'base', tron: 'tron', btc: 'bitcoin', solana: 'solana' }

function source(file: string | undefined, base: string): string | undefined {
  return typeof file === 'string' ? `${base.replace(/\/$/, '')}/deposit-icons/${file}.png` : undefined
}

export function depositAssetIcon(symbol: string, base = import.meta.env?.BASE_URL || '/'): string | undefined {
  return source(assetFiles[symbol.toUpperCase()], base)
}

export function depositNetworkIcon(network: string, base = import.meta.env?.BASE_URL || '/'): string | undefined {
  return source(networkFiles[network.toLowerCase()], base)
}
