import assert from 'node:assert/strict'
import test from 'node:test'
import { createSecondsChartSession, type SecondsChartSnapshot } from '../src/core/secondsChartSession.ts'
import type { MarketDetailStreamOptions } from '../src/api/marketDetailStream.ts'
import type { KlinePoint } from '../src/core/types.ts'
import { chartPoints, mountMarketChart } from './helpers/market-chart-renderer.ts'
import { mountMobileMarketChart } from './helpers/mobile-market-chart.ts'

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: Error) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}

function fixture() {
  const requests: Array<ReturnType<typeof deferred<KlinePoint[]>>> = []
  const streams: MarketDetailStreamOptions[] = []
  let stops = 0
  let snapshot: SecondsChartSnapshot = { points: [], loading: false, failed: false }
  const session = createSecondsChartSession({
    getUrl: () => 'wss://example.test/public',
    load() { const request = deferred<KlinePoint[]>(); requests.push(request); return request.promise },
    onChange(value) { snapshot = value },
    startStream(options) { streams.push(options); return () => { stops += 1 } },
  })
  return { session, requests, streams, get snapshot() { return snapshot }, get stops() { return stops } }
}
const candle = (close: number): KlinePoint => ({ time: 1700000040000, open: 10, high: 15, low: 9, close, volume: 2 })

test('seconds chart keeps live candles over late history and subscribes to K-lines only', async () => {
  const f = fixture()
  const pending = f.session.open('BTCUSDT', '1m')
  assert.equal(f.snapshot.loading, true)
  assert.deepEqual(f.streams[0]!.channels, ['kline'])
  f.streams[0]!.onKline(candle(13))
  f.requests[0]!.resolve([candle(11)])
  await pending
  assert.equal(f.snapshot.points[0]!.close, 13)
  assert.equal(f.snapshot.loading, false)
  f.session.stop()
  assert.equal(f.stops, 1)
})

test('seconds chart replacement and close reject old REST, errors and stream callbacks', async () => {
  const f = fixture()
  const first = f.session.open('BTCUSDT', '1m')
  const second = f.session.open('ETHUSDT', '5m')
  assert.equal(f.stops, 1)
  f.streams[0]!.onKline(candle(14))
  f.requests[0]!.reject(new Error('old request'))
  await first
  assert.equal(f.snapshot.failed, false)
  assert.equal(f.snapshot.loading, true)
  assert.deepEqual(f.snapshot.points, [])
  f.session.stop()
  f.streams[1]!.onKline(candle(12))
  f.requests[1]!.resolve([candle(11)])
  await second
  assert.deepEqual(f.snapshot, { points: [], loading: false, failed: false })
  assert.equal(f.stops, 2)
})

test('seconds chart failed history is retryable and retains any live data', async () => {
  const f = fixture()
  const pending = f.session.open('BTCUSDT', '1m')
  f.streams[0]!.onKline(candle(13))
  f.requests[0]!.reject(new Error('offline'))
  await pending
  assert.equal(f.snapshot.failed, true)
  assert.equal(f.snapshot.loading, false)
  assert.equal(f.snapshot.points[0]!.close, 13)
  const retry = f.session.open('BTCUSDT', '1m')
  f.requests[1]!.resolve([candle(12)])
  await retry
  assert.equal(f.snapshot.failed, false)
  assert.equal(f.snapshot.points[0]!.close, 12)
  f.session.stop()
})

test('inspection enables candle wicks and manages crosshair readout lifecycle', () => {
  const chart = mountMarketChart({ points: chartPoints(100), inspection: true })
  assert.equal(chart.series[0]!.options[0]!.wickVisible, true)
  assert.equal(chart.crosshairListenerCount, 1)
  chart.crosshair({ point: { x: 20, y: 30 }, time: 1700000000 })
  chart.crosshair({})
  assert.deepEqual(chart.inspections, [1700000000, null])
  chart.unmount()
  assert.equal(chart.crosshairListenerCount, 0)
  const normal = mountMarketChart({ points: chartPoints(100) })
  assert.equal(normal.series[0]!.options[0]!.wickVisible, false)
  assert.equal(normal.crosshairListenerCount, 0)
  normal.unmount()
})

test('inspection zoom/latest and indicator toggles operate on native chart without fitting history', async () => {
  const chart = mountMarketChart({ points: chartPoints(100), inspection: true })
  const fits = chart.fits
  chart.setRange({ from: 20, to: 60 })
  chart.controls.zoom(.5)
  assert.deepEqual(chart.range, { from: 30, to: 50 })
  chart.controls.zoom(2)
  assert.deepEqual(chart.range, { from: 20, to: 60 })
  chart.controls.zoom(Number.NaN)
  assert.deepEqual(chart.range, { from: 20, to: 60 })
  chart.controls.latest()
  assert.equal(chart.latestCalls, 1)
  assert.equal(chart.inspections.at(-1), null)
  assert.equal(chart.fits, fits)
  assert.ok(!chart.emitted.includes('load-history'))
  await chart.update({ showAverages: false, showVolume: false })
  for (const series of chart.series.slice(1)) assert.equal(series.options.at(-1)!.visible, false)
  await chart.update({ showAverages: true, showVolume: true })
  for (const series of chart.series.slice(1)) assert.equal(series.options.at(-1)!.visible, true)
  chart.unmount()
})

test('actual inspector renders hovered OHLC, restores latest, localizes and bridges controls', async () => {
  const points = [candle(11), { ...candle(14), time: candle(14).time + 60000 }]
  const view = mountMobileMarketChart(async () => [], points)
  await view.update({ inspection: true })
  assert.equal(view.chart().props.inspection, true)
  assert.match(view.text(), /放大|开|收/)
  const readout = () => view.all().find((node) => node.props.class === 'chart-inspector__readout')!
  assert.equal(readout().props['data-inspection-time'], points[1]!.time / 1000)
  ;(view.chart().props.onInspect as (time: number | null) => void)(points[0]!.time / 1000)
  await view.flush()
  assert.equal(readout().props['data-inspection-time'], points[0]!.time / 1000)
  ;(view.chart().props.onInspect as (time: number | null) => void)(null)
  await view.flush()
  assert.equal(readout().props['data-inspection-time'], points[1]!.time / 1000)
  const buttons = view.all().filter((node) => node.type === 'button')
  for (const label of ['放大图表', '缩小图表', '回到最新行情']) {
    const button = buttons.find((node) => node.props['aria-label'] === label)!
    ;(button.props.onClick as () => void)()
  }
  assert.deepEqual(view.controls, [1 / 1.4, 1.4, 'latest'])
  for (const input of view.all().filter((node) => node.type === 'input')) {
    ;(input.props.onChange as (event: unknown) => void)({ target: { checked: false } })
  }
  await view.flush()
  assert.equal(view.chart().props.showAverages, false)
  assert.equal(view.chart().props.showVolume, false)
  view.i18n.global.locale.value = 'en'
  await view.flush()
  assert.match(view.text(), /Open/)
  assert.match(view.text(), /Close/)
  assert.ok(view.all().some((node) => node.props['aria-label'] === 'Return to latest'))
  await view.update({ points: [], symbol: 'ETHUSDT' })
  assert.equal(readout().props['data-inspection-time'], undefined)
  assert.ok(view.all().filter((node) => node.type === 'button').every((node) => node.props.disabled === true))
  view.unmount()
})
