import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import ts from 'typescript'
import * as vue from 'vue'
import * as pinia from 'pinia'
import * as lifecycle from '../src/core/marketLifecycle.ts'
import * as freshness from '../src/core/marketTickerFreshness.ts'
import { normalizeSymbol } from '../src/core/format.ts'
import { mapMarketTicker } from '../src/core/marketMapper.ts'
import type { MarketTicker } from '../src/core/types.ts'

function harness() {
  let catalog: unknown = { markets: [{ id: 1, symbol: 'NEWUSDT', base_asset: 'NEW', quote_asset: 'USDT' }] }
  let quoteFails = false
  const source = readFileSync(new URL('../src/api/market.ts', import.meta.url), 'utf8')
  const body = source.match(/export async function fetchMarketTickers\([\s\S]*?\n}/)![0].replace(/^export /, '')
  const code = ts.transpileModule(body, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText
  const fetchMarketTickers = new Function('client', 'requestUrl', 'publicApiRequestConfig', 'normalizeSymbol', 'mapMarketTicker', `${code}; return fetchMarketTickers;`)(
    { get: async (url: string) => {
      if (url === '/markets') return { data: catalog }
      if (quoteFails) throw new Error('quote service unavailable')
      return { data: { symbol: 'NEWUSDT', last_price: '1', volume_24h: '2' } }
    } }, (path: string) => path, () => ({}), normalizeSymbol, mapMarketTicker,
  ) as () => Promise<MarketTicker[]>

  const storeSource = readFileSync(new URL('../src/stores/market.ts', import.meta.url), 'utf8')
  const storeCode = ts.transpileModule(storeSource, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } }).outputText
  const modules: Record<string, unknown> = {
    vue, pinia, '@/api/market': { fetchMarketTickers },
    '@/api/marketSocket': { subscribeTickers: () => () => undefined },
    '@/core/format': { normalizeSymbol }, '@/core/marketLifecycle': lifecycle,
    '@/core/marketTickerFreshness': freshness,
  }
  const exports: Record<string, () => { tickers: MarketTicker[]; error: boolean; refresh(force: boolean): Promise<void> }> = {}
  new Function('require', 'exports', storeCode)((name: string) => {
    assert.ok(name in modules, `unexpected dependency ${name}`)
    return modules[name]
  }, exports)
  pinia.setActivePinia(pinia.createPinia())
  return {
    store: exports.useMarketStore!(), fetchMarketTickers,
    setCatalog(value: unknown) { catalog = value },
    failQuotes() { quoteFails = true },
  }
}

test('an authoritative empty spot catalog removes cached purchase-stage rows', async () => {
  const state = harness()
  await state.store.refresh(true)
  assert.equal(state.store.tickers.length, 1)
  state.setCatalog({ markets: [] })
  await state.store.refresh(true)
  assert.deepEqual(state.store.tickers, [])
  assert.equal(state.store.error, false)
})

test('malformed catalogs and failed quotes stay failures rather than successful empty snapshots', async () => {
  for (const mode of ['malformed', 'quotes']) {
    const state = harness()
    await state.store.refresh(true)
    if (mode === 'malformed') state.setCatalog({})
    else state.failQuotes()
    await assert.rejects(state.fetchMarketTickers())
    await state.store.refresh(true)
    assert.equal(state.store.error, true)
    assert.equal(state.store.tickers.length, 1)
  }
})
