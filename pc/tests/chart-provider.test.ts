import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { DEFAULT_CHART_PROVIDER, normalizeChartProvider } from '../src/utils/chartProvider.ts'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')

function readProjectFile(path: string) {
  return readFileSync(resolve(repoRoot, path), 'utf8')
}

test('normalizes backend chart providers to supported PC renderers', () => {
  assert.equal(normalizeChartProvider('tradingview'), 'tradingview')
  assert.equal(normalizeChartProvider(' TradingView '), 'tradingview')
  assert.equal(normalizeChartProvider('unsupported'), DEFAULT_CHART_PROVIDER)
  assert.equal(normalizeChartProvider(null), DEFAULT_CHART_PROVIDER)
})

test('all PC trading pages use the backend-selected market chart wrapper', () => {
  for (const path of [
    'src/views/Trade.vue',
    'src/views/Contract.vue',
    'src/views/SecondOptions.vue',
    'src/views/LaunchpadTrade.vue'
  ]) {
    const source = readProjectFile(path)
    assert.match(source, /MarketChart/)
    assert.doesNotMatch(source, /import TVChart/)
  }

  const wrapperSource = readProjectFile('src/components/chart/MarketChart.vue')
  assert.match(wrapperSource, /defineAsyncComponent/)
  assert.match(wrapperSource, /TradingViewChart/)
  assert.match(wrapperSource, /settingStore\.loadPlatformBrand\(\)/)
})

test('PC chart omits source summaries without changing chart data, provider or interval bindings', () => {
  const source = readProjectFile('src/components/chart/MarketChart.vue')
  assert.doesNotMatch(source, /market-chart-sources|marketProvenance\.chartSources|MarketProvenanceLabel|receiveProvenance/)
  assert.match(source, /:is="chartComponent"/)
  assert.match(source, /:key="chartKey"/)
  for (const [attribute, value] of [
    ['data-list', 'dataList'],
    ['fetch-k-line', 'fetchKLine'],
    ['kline-topic', 'klineTopic'],
    ['module', 'module'],
    ['period', 'period'],
    ['precision', 'precision'],
    ['symbol', 'symbol'],
  ]) {
    assert.ok(source.includes(`:${attribute}="${value}"`))
  }
  assert.match(source, /period: '1m'/)
  assert.match(source, /\.market-chart-canvas\s*\{[^}]*flex: 1/)
})
