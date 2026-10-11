import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'
import { createServer } from 'vite'
import { parse } from '@vue/compiler-sfc'
import en from '../src/i18n/messages/en.ts'
import zhCN from '../src/i18n/messages/zh-CN.ts'

test('convert history preserves directional logos, nulls, old-server fallback and exact money', async (context) => {
  const server = await createServer({ appType: 'custom', logLevel: 'silent', server: { middlewareMode: true } })
  context.after(() => server.close())
  const { client } = await server.ssrLoadModule('/src/api/client.ts')
  const { referenceRequestRegistry } = await server.ssrLoadModule('/src/api/requestCache.ts')
  const { fetchConvertOrdersPage } = await server.ssrLoadModule('/src/api/swap.ts')
  const pair = { id: 1, from_asset_id: 2, to_asset_id: 3, from_asset_symbol: 'BTC', to_asset_symbol: 'USDT', from_asset_logo_url: '/btc.png', to_asset_logo_url: '/usdt.png', min_amount: '0', max_amount: null, fee_rate: '0' }
  let order: Record<string, unknown> = { id: 7, from_asset_id: 2, to_asset_id: 3, from_amount: '9007199254740993.000000000000000001', to_amount: '0.000000000000000001', rate: '2', fee_amount: '0', created_at: 1791684000000, status: 'completed' }
  let pairFailure = false
  const requests: Array<{ path: string; params: unknown }> = []
  client.defaults.adapter = async (config: { url: string; params?: unknown }) => {
    requests.push({ path: config.url, params: config.params })
    const pairs = config.url.endsWith('/convert/pairs')
    if (pairs && pairFailure) throw new Error('catalog unavailable')
    return { data: pairs ? { pairs: [pair] } : { orders: [order], has_more: false }, status: 200, statusText: 'OK', headers: {}, config }
  }
  let page = await fetchConvertOrdersPage({ limit: 20, offset: 40 })
  assert.equal(page.items[0].fromAssetLogoUrl, '/btc.png')
  assert.equal(page.items[0].toAssetLogoUrl, '/usdt.png')
  assert.equal(page.items[0].fromAmount, order.from_amount)
  assert.equal(page.items[0].toAmount, order.to_amount)
  assert.equal(page.nextOffset, 41)
  assert.deepEqual(requests.find((row) => row.path.endsWith('/orders'))?.params, { limit: 20, offset: 40 })
  order = { ...order, from_asset_id: 3, to_asset_id: 2 }
  page = await fetchConvertOrdersPage()
  assert.equal(page.items[0].fromAssetLogoUrl, '/usdt.png')
  assert.equal(page.items[0].toAssetLogoUrl, '/btc.png')
  order = { ...order, from_asset_symbol: 'USDT', to_asset_symbol: 'BTC', from_asset_logo_url: ' /historical-usdt.png ', to_asset_logo_url: null }
  referenceRequestRegistry.invalidate()
  pairFailure = true
  page = await fetchConvertOrdersPage()
  assert.equal(page.items[0].fromAssetLogoUrl, '/historical-usdt.png')
  assert.equal(page.items[0].toAssetLogoUrl, undefined)
  assert.equal(page.items[0].fromAssetSymbol, 'USDT')
})

test('swap history is independently routed, session-owned, localized and uses both real asset images', () => {
  const read = (file: string) => readFileSync(new URL(file, import.meta.url), 'utf8')
  const view = read('../src/views/SwapHistoryView.vue')
  const swap = read('../src/views/SwapView.vue')
  const template = parse(view).descriptor.template!.content
  assert.match(read('../src/router/index.ts'), /path: '\/swap\/history', name: 'swap-history'[\s\S]*?depth: 2, backFallback: '\/swap'/)
  assert.match(swap, /router\.push\(\{ name: 'swap-history' \}\)/)
  assert.doesNotMatch(swap, /fetchConvertOrders|historySection|v-for="order in orders"/)
  assert.match(template, /:src="order\.fromAssetLogoUrl"/)
  assert.match(template, /:src="order\.toAssetLogoUrl"/)
  assert.match(view, /session\.isAuthenticated \? `\$\{session\.generation\}:\$\{session\.token\}` : ''/)
  assert.match(view, /onBeforeUnmount\(\(\) => pagination\.stop\(\)\)/)
  assert.match(view, /pagination\.reset\(\)/)
  assert.match(template, /pagination\.retryLoadMore/)
  assert.match(template, /formatExactAmount\(order\.fromAmount\)/)
  for (const messages of [en, zhCN]) {
    for (const key of ['historyLoading', 'historyLoadFailed', 'historyLoginDescription', 'historyRefresh'] as const) {
      assert.ok(messages.swap[key])
    }
  }
})
