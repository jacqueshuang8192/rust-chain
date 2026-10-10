// Anonymous, bounded public-market observation. No private endpoints or credentials.
import { setTimeout as delay } from 'node:timers/promises'

const durationMs = 30_000
const symbols = ['BTCUSDT', 'ETHUSDT']
const rows = new Map()
const errors = []
const sockets = []
let closing = false
const startedAt = new Date().toISOString()

function record(key, observed, price) {
  const sourceTime = Number(observed)
  if (!Number.isFinite(sourceTime) || sourceTime <= 0) return
  const received = Date.now()
  const entries = rows.get(key) || []
  if (entries.length < 10_000) entries.push({ received, sourceTime, ageMs: received - sourceTime, price })
  rows.set(key, entries)
}

async function restSnapshot(symbol) {
  const start = Date.now()
  try {
    const response = await fetch(`https://hipoex.cllbmz.kdns.fr/api/v1/markets/${symbol}/ticker`, {
      cache: 'no-store',
      credentials: 'omit',
      signal: AbortSignal.timeout(8_000),
    })
    const payload = await response.json()
    const end = Date.now()
    return {
      symbol, status: response.status, elapsedMs: end - start,
      serverDate: response.headers.get('date'),
      sampledAt: new Date(end).toISOString(),
      price: payload.last_price, source: payload.source, provider: payload.provider,
      observedAt: payload.observed_at, ageMs: end - Number(payload.observed_at),
    }
  } catch (error) {
    return { symbol, error: error.message, elapsedMs: Date.now() - start }
  }
}

const before = await Promise.all(symbols.map(restSnapshot))
for (const [name, url] of [
  ['bitget', 'wss://ws.bitget.com/v2/ws/public'],
  ['hippo', 'wss://hipoex.cllbmz.kdns.fr/api/v1/ws/public'],
]) {
  const socket = new WebSocket(url)
  sockets.push(socket)
  socket.addEventListener('error', () => errors.push({
    source: name, event: 'socket error', afterRequestedClose: closing,
    sampledAt: new Date().toISOString(),
  }))
  socket.addEventListener('open', () => {
    if (name === 'bitget') {
      socket.send(JSON.stringify({
        op: 'subscribe',
        args: symbols.flatMap((instId) => ['ticker', 'books50', 'candle1m'].map((channel) => (
          { instType: 'SPOT', channel, instId }
        ))),
      }))
    } else {
      for (const symbol of symbols) {
        for (const channel of ['ticker', 'depth', 'kline']) {
          socket.send(JSON.stringify({
            op: 'subscribe', channel, symbol,
            ...(channel === 'kline' ? { interval: '1m' } : {}),
          }))
        }
      }
    }
  })
  socket.addEventListener('message', (event) => {
    let payload
    try { payload = JSON.parse(event.data) } catch { return }
    if (name === 'bitget') {
      const data = payload.data?.[0]
      if (!data) return
      const channel = payload.arg.channel === 'books50' ? 'depth'
        : payload.arg.channel === 'candle1m' ? 'kline' : payload.arg.channel
      record(`${name}:${payload.arg.instId}:${channel}`, data.ts ?? payload.ts, data.lastPr)
    } else {
      if (!payload.symbol || !payload.observed_at) return
      const channel = payload.last_price !== undefined ? 'ticker'
        : payload.bids ? 'depth' : payload.interval ? 'kline' : 'other'
      record(`${name}:${payload.symbol}:${channel}`, payload.observed_at, payload.last_price)
    }
  })
}

await delay(durationMs)
closing = true
for (const socket of sockets) socket.close()
const after = await Promise.all(symbols.map(restSnapshot))
const streams = {}
for (const [key, samples] of rows) {
  const ages = samples.map((row) => row.ageMs).sort((a, b) => a - b)
  const first = samples[0]
  const last = samples.at(-1)
  streams[key] = {
    count: samples.length,
    minAgeMs: ages[0],
    medianAgeMs: ages[Math.floor(ages.length / 2)],
    maxAgeMs: ages.at(-1),
    sourceAdvanceMs: last.sourceTime - first.sourceTime,
    receiveAdvanceMs: last.received - first.received,
    lagGrowthMs: last.ageMs - first.ageMs,
    first, last,
  }
}
console.log(JSON.stringify({
  startedAt, completedAt: new Date().toISOString(), authenticated: false,
  durationMs, errors, before, after, streams,
}, null, 2))
// A stuck closing handshake must not leave the diagnostic process running.
setTimeout(() => process.exit(0), 500).unref()
