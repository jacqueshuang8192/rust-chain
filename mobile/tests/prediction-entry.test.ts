import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { compileScript, parse } from '@vue/compiler-sfc'
import { createServer } from 'vite'
import ts from 'typescript'
import * as Vue from 'vue'

const source = readFileSync(new URL('../src/views/ProductHubView.vue', import.meta.url), 'utf8')
const compiled = compileScript(parse(source).descriptor, { id: 'prediction-entry', inlineTemplate: true })
const output = ts.transpileModule(compiled.content, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
}).outputText
const flush = () => new Promise<void>(resolve => setImmediate(resolve))

function hubFixture(load: (options: { force: boolean }) => Promise<{ mobileEntryEnabled: boolean }>) {
  const mounted: Array<() => void> = []
  const cleanup: Array<() => void> = []
  const pageDocument = Object.assign(new EventTarget(), { visibilityState: 'visible' })
  const pageWindow = new EventTarget()
  const navigations: object[] = []
  const modules: Record<string, unknown> = {
    vue: { ...Vue, onMounted: (fn: () => void) => mounted.push(fn), onBeforeUnmount: (fn: () => void) => cleanup.push(fn) },
    'vue-router': { useRouter: () => ({ push: (target: object) => navigations.push(target) }) },
    'vue-i18n': { useI18n: () => ({ t: (key: string) => key }) },
    'lucide-vue-next': { BookOpen: 'i', ChevronRight: 'i', Ellipsis: 'i', Gauge: 'i', Newspaper: 'i' },
    '@/components/PageHeader.vue': { default: 'header' },
    '@/api/prediction': { fetchPredictionSettings: load },
  }
  const exports: { default?: { setup(props: object, context: object): () => Vue.VNode } } = {}
  new Function('require', 'exports', 'document', 'window', output)(
    (name: string) => { assert.ok(Object.hasOwn(modules, name), name); return modules[name] },
    exports, pageDocument, pageWindow,
  )
  const render = exports.default!.setup({}, { expose() {} })
  function nodes(node: unknown): Vue.VNode[] {
    if (Array.isArray(node)) return node.flatMap(nodes)
    if (!Vue.isVNode(node)) return []
    return [node, ...nodes(node.children)]
  }
  const all = () => nodes(render())
  const product = (name: string) => all().find(node => node.props?.['data-product'] === name)
  mounted.forEach(fn => fn())
  return { product, all, navigations, pageDocument, pageWindow, stop: () => cleanup.forEach(fn => fn()) }
}

test('product hub waits for config, hides the disabled entry, and preserves news/help navigation', async () => {
  let resolve!: (value: { mobileEntryEnabled: boolean }) => void
  const fixture = hubFixture(async options => {
    assert.equal(options.force, true)
    return new Promise(done => { resolve = done })
  })
  try {
    assert.equal(fixture.product('prediction'), undefined)
    resolve({ mobileEntryEnabled: false })
    await flush()
    assert.equal(fixture.product('prediction'), undefined)
    assert.equal(fixture.all().find(node => node.props?.['data-product-count'])?.props?.['data-product-count'], 1)
    fixture.product('news')!.props!.onClick()
    fixture.all().find(node => node.props?.class === 'product-hub__help')!.props!.onClick()
    assert.deepEqual(fixture.navigations, [{ name: 'news' }, { name: 'news', query: { category: 'product' } }])
  } finally { fixture.stop() }
})

test('product hub refreshes after foreground return, coalesces concurrent refreshes and rejects late results', async () => {
  let calls = 0
  let resolve!: (value: { mobileEntryEnabled: boolean }) => void
  const fixture = hubFixture(async () => { calls++; return new Promise(done => { resolve = done }) })
  resolve({ mobileEntryEnabled: true })
  await flush()
  fixture.product('prediction')!.props!.onClick()
  assert.deepEqual(fixture.navigations, [{ name: 'prediction' }])
  const oldClick = fixture.product('prediction')!.props!.onClick
  fixture.pageDocument.visibilityState = 'hidden'
  fixture.pageDocument.dispatchEvent(new Event('visibilitychange'))
  assert.equal(calls, 1)
  fixture.pageDocument.visibilityState = 'visible'
  fixture.pageDocument.dispatchEvent(new Event('visibilitychange'))
  fixture.pageWindow.dispatchEvent(new Event('focus'))
  assert.equal(calls, 2)
  resolve({ mobileEntryEnabled: false })
  await flush()
  assert.equal(fixture.product('prediction'), undefined)
  oldClick()
  assert.equal(fixture.navigations.length, 1)
  fixture.pageWindow.dispatchEvent(new Event('focus'))
  assert.equal(calls, 3)
  fixture.stop()
  resolve({ mobileEntryEnabled: true })
  await flush()
  assert.equal(fixture.product('prediction'), undefined)
  fixture.pageWindow.dispatchEvent(new Event('focus'))
  fixture.pageDocument.dispatchEvent(new Event('visibilitychange'))
  assert.equal(calls, 3)
})

test('product hub hides unverified entries on failure and can recover on focus', async () => {
  let fail = true
  const fixture = hubFixture(async () => {
    if (fail) throw new Error('offline')
    return { mobileEntryEnabled: true }
  })
  try {
    await flush()
    assert.equal(fixture.product('prediction'), undefined)
    fail = false
    fixture.pageWindow.dispatchEvent(new Event('focus'))
    await flush()
    assert.ok(fixture.product('prediction'))
    fail = true
    fixture.pageWindow.dispatchEvent(new Event('focus'))
    await flush()
    assert.equal(fixture.product('prediction'), undefined)
  } finally { fixture.stop() }
})

test('prediction config shares its cache with assets, preserves false, and supports force and old servers', async context => {
  const server = await createServer({ appType: 'custom', logLevel: 'silent', server: { middlewareMode: true } })
  context.after(() => server.close())
  const { client } = await server.ssrLoadModule('/src/api/client.ts')
  const api = await server.ssrLoadModule('/src/api/prediction.ts')
  let calls = 0
  let flag: unknown = false
  client.get = async () => {
    calls++
    return { data: { mobile_entry_enabled: flag, allowed_assets: [{ asset_id: 7, asset_symbol: 'usdt', max_payout_amount: '1000' }] } }
  }
  const [settings, assets] = await Promise.all([api.fetchPredictionSettings(), api.fetchPredictionConfig()])
  assert.equal(calls, 1)
  assert.equal(settings.mobileEntryEnabled, false)
  assert.deepEqual(assets, [{ assetId: 7, assetSymbol: 'USDT', maxPayoutAmount: '1000' }])
  assets[0].assetSymbol = 'mutated'
  assert.equal((await api.fetchPredictionSettings()).allowedAssets[0].assetSymbol, 'USDT')
  flag = true
  assert.equal((await api.fetchPredictionSettings()).mobileEntryEnabled, false)
  assert.equal((await api.fetchPredictionSettings({ force: true })).mobileEntryEnabled, true)
  flag = undefined
  assert.equal((await api.fetchPredictionSettings({ force: true })).mobileEntryEnabled, true)
  flag = 'false'
  await assert.rejects(api.fetchPredictionSettings({ force: true }), /invalid mobile entry flag/)
  flag = false
  assert.equal((await api.fetchPredictionSettings({ force: true })).mobileEntryEnabled, false)
})
