import { createMarketDetailStreamSession, type MarketDetailStreamSessionOptions } from '../api/marketDetailStream.ts'
import type { MarketKlineInterval } from '../api/marketSocketProtocol.ts'
import type { KlinePoint } from './types.ts'

export interface SecondsChartSnapshot {
  points: KlinePoint[]
  loading: boolean
  failed: boolean
}

/** Each expanded chart owns its stream and rejects completed requests after close. */
export function createSecondsChartSession(options: {
  getUrl(): string
  load(symbol: string, interval: MarketKlineInterval): Promise<KlinePoint[]>
  onChange(snapshot: SecondsChartSnapshot): void
  startStream?: MarketDetailStreamSessionOptions['startStream']
}) {
  let version = 0
  let snapshot: SecondsChartSnapshot = { points: [], loading: false, failed: false }
  const publish = (patch: Partial<SecondsChartSnapshot>) => {
    snapshot = { ...snapshot, ...patch }
    options.onChange(snapshot)
  }
  const session = createMarketDetailStreamSession({
    getUrl: options.getUrl,
    channels: ['kline'],
    startStream: options.startStream,
    onDepth() {},
    onTrade() {},
    onKlines(_context, points) { publish({ points, failed: false }) },
  })
  async function open(symbol: string, interval: MarketKlineInterval): Promise<void> {
    const requestVersion = ++version
    session.stop()
    publish({ points: [], loading: true, failed: false })
    const context = session.replace(symbol, interval, requestVersion)
    const request = session.beginKlineRequest(context)
    if (!request) return
    try {
      const points = await options.load(symbol, interval)
      const merged = session.resolveKlineRequest(request, points)
      if (merged) publish({ points: merged })
    } catch {
      if (session.isCurrentKlineRequest(request)) publish({ failed: true })
    } finally {
      if (requestVersion === version && session.isCurrentKlineRequest(request)) publish({ loading: false })
    }
  }
  function stop(): void {
    version += 1
    session.stop()
    publish({ points: [], loading: false, failed: false })
  }
  return { open, stop }
}
