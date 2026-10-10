import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import ts from 'typescript'
import * as vue from 'vue'
import * as pinia from 'pinia'
import { createMemoryHistory, createRouter } from 'vue-router'
import * as format from '../src/core/format.ts'
import * as protocol from '../src/api/marketSocketProtocol.ts'
import * as navigationCore from '../src/core/navigation.ts'

const key = 'hippo_mobile_last_seconds_product_id'
const storeSource = readFileSync(new URL('../src/stores/navigation.ts', import.meta.url), 'utf8')
const viewSource = readFileSync(new URL('../src/views/SecondsView.vue', import.meta.url), 'utf8')
type Product = { id: number; symbol: string; cycles: { id: number }[] }
type Navigation = {
  lastSecondsProductId: number | null
  lastTradeSymbol: string
  rememberSecondsProduct(id: number): void
}
const btc: Product = { id: 11, symbol: 'BTCUSDT', cycles: [{ id: 101 }] }
const eth: Product = { id: 22, symbol: 'ETHUSDT', cycles: [{ id: 202 }] }

function compile(source: string): string {
  return ts.transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
  }).outputText
}

function createNavigation(): Navigation {
  const exports: Record<string, unknown> = {}
  new Function('require', 'exports', compile(storeSource))((name: string) => {
    if (name === 'vue') return vue
    if (name === 'pinia') return pinia
    if (name === '@/core/navigation') return navigationCore
    throw new Error(`Unexpected import: ${name}`)
  }, exports)
  return (exports.useNavigationStore as (store: pinia.Pinia) => Navigation)(pinia.createPinia())
}

function storageFixture(t: test.TestContext, initial?: string) {
  const original = Object.getOwnPropertyDescriptor(globalThis, 'localStorage')
  const entries = new Map<string, string>(initial === undefined ? [] : [[key, initial]])
  const storage = {
    getItem: (name: string) => entries.get(name) ?? null,
    setItem: (name: string, value: string) => { entries.set(name, value) },
  }
  Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: storage })
  t.after(() => {
    if (original) Object.defineProperty(globalThis, 'localStorage', original)
    else Reflect.deleteProperty(globalThis, 'localStorage')
  })
  return { entries, storage }
}

async function createPage(navigation: Navigation, fetchProducts: () => Promise<Product[]>, path = '/seconds') {
  const routerSource = readFileSync(new URL('../src/router/index.ts', import.meta.url), 'utf8')
  const record = routerSource.match(/\{ path: '\/seconds\/:symbol\?'[^\n]+/)?.[0]
  assert.ok(record)
  const secondsRoute = new Function('SecondsView', `return (${record.replace(/,$/, '')})`)({})
  const router = createRouter({ history: createMemoryHistory(), routes: [
    { path: '/', name: 'home', component: {} },
    secondsRoute,
    { path: '/seconds/history', name: 'seconds-history', component: {} },
  ] })
  await router.push('/')
  await router.push(path)
  const selectionExports: Record<string, unknown> = {}
  const selectionSource = readFileSync(new URL('../src/composables/useSecondsProductSelection.ts', import.meta.url), 'utf8')
  new Function('require', 'exports', compile(selectionSource))((name: string) => {
    if (name === 'vue') return vue
    if (name === '@/stores/navigation') return { useNavigationStore: () => navigation }
    if (name === '@/core/format') return format
    if (name === '@/api/marketSocketProtocol') return protocol
    throw new Error(`Unexpected import: ${name}`)
  }, selectionExports)
  const script = viewSource.match(/<script setup lang="ts">([\s\S]*?)<\/script>/)?.[1]
  assert.ok(script)
  const ast = ts.createSourceFile('SecondsView.ts', script, ts.ScriptTarget.Latest, true)
  const functions = ast.statements.filter((node) => (
    ts.isFunctionDeclaration(node) && ['load', 'selectProduct'].includes(node.name?.text ?? '')
  ))
  assert.equal(functions.length, 2)
  const source = `
    const products = ref([]);
    const { selected, restoreProducts, rememberProduct } = useSecondsProductSelection({router, products, onSelect: selectProduct});
    const selectedCycleId = { value: 0 }, direction = { value: 'up' }, amount = { value: '' };
    const loading = { value: false }, error = { value: '' };
    const session = { isAuthenticated: false };
    let loadRequestVersion = 0, privateReconciliationGeneration = 0, componentActive = true;
    const loadSparkline = () => {}, replaceTickerSubscription = () => {}, clearSecondsPrivateState = () => {};
    const apiErrorMessage = () => 'load failed', t = (key) => key;
    ${functions.map((node) => node.getText(ast)).join('\n')}
    return { load, selectProduct, selected, selectedCycleId, error };
  `
  const page = new Function('router', 'fetchSecondsProducts', 'ref', 'useSecondsProductSelection', compile(source))(
    router, fetchProducts, vue.ref, selectionExports.useSecondsProductSelection,
  ) as {
    load(): Promise<void>
    selectProduct(product: Product): void
    selected: { value: Product | null }
    selectedCycleId: { value: number }
    error: { value: string }
  }
  return { ...page, router, async load() { await page.load(); await flushRoutes() } }
}

async function flushRoutes() {
  await new Promise<void>((resolve) => setImmediate(resolve))
  await vue.nextTick()
}

test('Seconds choice survives a fresh app/page with reordered products independently of spot', async (t) => {
  const { entries } = storageFixture(t)
  entries.set('hippo_mobile_last_trade_symbol', 'DOGE_USDT')
  const first = await createPage(createNavigation(), async () => [btc, eth])
  await first.load()
  first.selectProduct(eth)
  assert.equal(entries.get(key), '22')
  const reloadedNavigation = createNavigation()
  const reloaded = await createPage(reloadedNavigation, async () => [btc, eth])
  await reloaded.load()
  assert.equal(reloaded.selected.value?.id, eth.id)
  assert.equal(reloaded.selectedCycleId.value, 202)
  assert.equal(reloadedNavigation.lastTradeSymbol, 'DOGE_USDT')
  const reordered = await createPage(createNavigation(), async () => [eth, btc])
  await reordered.load()
  assert.equal(reordered.selected.value?.id, eth.id)
})

test('Seconds pending refresh retains a newer user selection and uses fresh product data', async (t) => {
  storageFixture(t)
  let finish!: (products: Product[]) => void
  let response = Promise.resolve([btc, eth])
  const page = await createPage(createNavigation(), () => response)
  await page.load()
  response = new Promise((resolve) => { finish = resolve })
  const pending = page.load()
  page.selectProduct(eth)
  const updatedEth = { ...eth, cycles: [{ id: 303 }] }
  finish([btc, updatedEth])
  await pending
  assert.deepEqual(page.selected.value, updatedEth)
  assert.equal(page.selectedCycleId.value, 303)
})

test('Seconds removed product falls back, while empty or failed loads preserve the preference', async (t) => {
  const { entries } = storageFixture(t, '22')
  let response: Promise<Product[]> = Promise.resolve([])
  const page = await createPage(createNavigation(), () => response)
  await page.load()
  assert.equal(page.selected.value, null)
  assert.equal(entries.get(key), '22')
  response = Promise.reject(new Error('offline'))
  await page.load()
  assert.equal(page.error.value, 'load failed')
  assert.equal(entries.get(key), '22')
  response = Promise.resolve([btc])
  await page.load()
  assert.deepEqual(page.selected.value, btc)
  assert.equal(entries.get(key), '11')
})

test('Seconds rejects corrupt identities and tolerates inaccessible storage', async (t) => {
  const { storage, entries } = storageFixture(t)
  for (const invalid of ['', '0', '-1', '1.2', 'NaN', '9007199254740993', '{"id":22}']) {
    entries.set(key, invalid)
    assert.equal(createNavigation().lastSecondsProductId, null)
  }
  storage.getItem = () => { throw new Error('blocked') }
  storage.setItem = () => { throw new Error('quota') }
  const navigation = createNavigation()
  const page = await createPage(navigation, async () => [btc, eth])
  await page.load()
  page.selectProduct(eth)
  assert.equal(navigation.lastSecondsProductId, eth.id)
  const reentered = await createPage(navigation, async () => [btc, eth])
  await reentered.load()
  assert.equal(reentered.selected.value?.id, eth.id)
  navigation.rememberSecondsProduct(-1)
  assert.equal(navigation.lastSecondsProductId, eth.id)
})

test('Seconds URL overrides stored choice, normalizes aliases, and preserves query without pushing history', async (t) => {
  storageFixture(t, '22')
  const page = await createPage(createNavigation(), async () => [btc, eth], '/seconds/btc_usdt?from=share')
  await page.load()
  assert.equal(page.selected.value?.id, btc.id)
  assert.equal(page.router.currentRoute.value.fullPath, '/seconds/BTC_USDT?from=share')
  page.selectProduct(eth)
  await flushRoutes()
  assert.equal(page.router.currentRoute.value.fullPath, '/seconds/ETH_USDT?from=share')
  page.router.back()
  await flushRoutes()
  assert.equal(page.router.currentRoute.value.path, '/')
})

test('Seconds responds to same-component route changes and keeps history distinct from symbols', async (t) => {
  storageFixture(t)
  const page = await createPage(createNavigation(), async () => [btc, eth], '/seconds/BTC_USDT')
  await page.load()
  await page.router.replace('/seconds/ETH_USDT')
  await flushRoutes()
  assert.equal(page.selected.value?.id, eth.id)
  assert.equal(page.selectedCycleId.value, 202)
  await page.router.push({ name: 'seconds-history' })
  await flushRoutes()
  assert.equal(page.router.currentRoute.value.name, 'seconds-history')
  assert.equal(page.router.currentRoute.value.path, '/seconds/history')
  page.router.back()
  await flushRoutes()
  assert.equal(page.router.currentRoute.value.path, '/seconds/ETH_USDT')
})

test('Seconds missing symbol falls back and legacy product entry restores saved choice', async (t) => {
  storageFixture(t, '22')
  const legacy = await createPage(createNavigation(), async () => [btc, eth], '/products/seconds')
  await legacy.load()
  assert.equal(legacy.router.currentRoute.value.path, '/seconds/ETH_USDT')
  const missing = await createPage(createNavigation(), async () => [btc, eth], '/seconds/REMOVED_USDT')
  await missing.load()
  assert.equal(missing.router.currentRoute.value.path, '/seconds/BTC_USDT')
})
