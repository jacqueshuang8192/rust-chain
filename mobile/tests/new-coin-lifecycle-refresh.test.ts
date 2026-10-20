import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import ts from 'typescript'
import * as Vue from 'vue'
import { compileScript, parse } from '@vue/compiler-sfc'
import { createSessionRequestLifecycle } from '../src/core/sessionRequest.ts'
import type { NewCoinProject } from '../src/core/newCoinModel.ts'
import { normalizeDecimalText } from '../src/core/decimal.ts'
import { parseTokenPolicy, tokenDecimal, tokenMultiply, formatTokenAmount, type TokenPolicy } from '../src/core/tokenIssuance.ts'
import * as decimal from '../src/core/decimal.ts'
import * as presentation from '../src/core/newCoinPresentation.ts'
import { formatDateTime } from '../src/core/format.ts'
import type { TokenSubscriptionIntent } from '../src/api/tokenIssuance.ts'

const flush = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); await Vue.nextTick() }
function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: unknown) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}
function project(stage: string, symbol = 'KHB'): NewCoinProject {
  return { id: symbol === 'KHB' ? 2 : 3, assetId: 6, symbol, lifecycleStatus: stage,
    totalSupplyText: normalizeDecimalText('1000'), issuePriceText: normalizeDecimalText('0.2'),
    reservedSupplyText: normalizeDecimalText('100'), allocatedSupplyText: normalizeDecimalText('0'),
    remainingSupplyText: normalizeDecimalText('900'), unlockType: 'immediate_on_listing',
    unlockFeeEnabled: false, postListingPurchaseEnabled: false, status: 'active' }
}
function execute(source: string, modules: Record<string, unknown>, globals: Record<string, unknown> = {}) {
  const code = ts.transpileModule(source, { compilerOptions: {
    target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS,
  } }).outputText
  const exports: Record<string, any> = {}
  new Function('require', 'exports', ...Object.keys(globals), code)(
    (name: string) => { assert.ok(name in modules, `Missing test module: ${name}`); return modules[name] },
    exports, ...Object.values(globals),
  )
  return exports
}

function projectHarness() {
  let mounted = () => {}; let unmount = () => {}
  const timers = new Set<() => void>()
  const listeners = new Map<string, () => void>()
  const document = { visibilityState: 'visible',
    addEventListener: (name: string, fn: () => void) => listeners.set(name, fn),
    removeEventListener: (name: string) => listeners.delete(name) }
  const requests: Array<ReturnType<typeof deferred<NewCoinProject>> & { symbol: string }> = []
  const exports = execute(readFileSync(new URL('../src/composables/useNewCoinProject.ts', import.meta.url), 'utf8'), {
    vue: { ...Vue, onMounted: (fn: () => void) => { mounted = fn }, onBeforeUnmount: (fn: () => void) => { unmount = fn } },
    '@/api/newCoin': { fetchNewCoinProject: (symbol: string) => {
      const request = { ...deferred<NewCoinProject>(), symbol }; requests.push(request); return request.promise
    } },
    '@/core/sessionRequest': { createSessionRequestLifecycle },
  }, { document, window: document, setInterval: (fn: () => void) => { timers.add(fn); return fn }, clearInterval: (fn: () => void) => timers.delete(fn) })
  const symbol = Vue.ref('KHB'); const paused = Vue.ref(false)
  const scope = Vue.effectScope()
  const state = scope.run(() => exports.useNewCoinProject(() => symbol.value, () => paused.value)) as {
    project: Vue.Ref<NewCoinProject | null>; refreshing: Vue.Ref<boolean>; error: Vue.Ref<unknown>; refresh(): Promise<void>
  }
  mounted()
  return { state, requests, symbol, paused, document, timers, listeners,
    tick: () => { for (const fn of timers) fn() }, stop: () => { unmount(); scope.stop() } }
}

test('visible detail advances only with refreshed API state, including purchase and spot listing', async () => {
  const h = projectHarness()
  try {
    h.requests[0]!.resolve(project('distribution')); await flush()
    h.tick(); h.tick()
    assert.equal(h.requests.length, 2, 'background reads are single-flight')
    assert.equal(h.state.project.value?.lifecycleStatus, 'distribution')
    h.requests[1]!.resolve(project('purchase')); await flush()
    assert.equal(h.state.project.value?.lifecycleStatus, 'purchase')
    h.listeners.get('focus')!()
    h.requests[2]!.resolve(project('listed')); await flush()
    assert.equal(h.state.project.value?.lifecycleStatus, 'listed')
    assert.equal(h.state.refreshing.value, false)
  } finally { h.stop() }
  assert.equal(h.timers.size, 0); assert.equal(h.listeners.size, 0)
})

test('hidden pages and reviewed transactions suspend background reads; return resumes them', async () => {
  const h = projectHarness()
  try {
    h.requests[0]!.resolve(project('subscription')); await flush()
    h.document.visibilityState = 'hidden'; h.tick(); h.listeners.get('visibilitychange')!()
    assert.equal(h.requests.length, 1)
    h.document.visibilityState = 'visible'; h.listeners.get('visibilitychange')!()
    h.paused.value = true
    h.requests[1]!.resolve(project('distribution')); await flush()
    assert.equal(h.state.project.value?.lifecycleStatus, 'subscription', 'an open legacy review retains its snapshot')
    h.tick(); assert.equal(h.requests.length, 2)
    h.paused.value = false; h.tick(); h.requests[2]!.resolve(project('distribution')); await flush()
    assert.equal(h.state.project.value?.lifecycleStatus, 'distribution')
  } finally { h.stop() }
})

test('refresh errors preserve the last stage; route changes and unmount reject stale completions', async () => {
  const h = projectHarness()
  h.requests[0]!.resolve(project('distribution')); await flush()
  h.tick(); h.requests[1]!.reject(new Error('network')); await flush()
  assert.equal(h.state.project.value?.lifecycleStatus, 'distribution')
  assert.ok(h.state.error.value)
  h.tick()
  h.symbol.value = 'OTHER'; await flush()
  assert.equal(h.state.project.value === null, true)
  h.requests[3]!.resolve(project('purchase', 'OTHER')); await flush()
  h.requests[2]!.resolve(project('listed')); await flush()
  assert.equal(h.state.project.value?.symbol, 'OTHER')
  assert.equal(h.state.error.value, null)
  h.tick(); h.stop(); h.requests[4]!.resolve(project('listed', 'OTHER')); await flush()
  assert.equal(h.state.project.value?.lifecycleStatus, 'purchase')
  await h.state.refresh(); assert.equal(h.requests.length, 5)
})

interface SubscriptionState {
  quantity: Vue.Ref<string>; intent: Vue.Ref<TokenSubscriptionIntent | null>; uncertain: Vue.Ref<boolean>
  success: Vue.Ref<boolean>; review(): void; submit(): Promise<void>; cancel(): void
}
function subscriptionHarness() {
  const { descriptor } = parse(readFileSync(new URL('../src/components/TokenSubscriptionForm.vue', import.meta.url), 'utf8'))
  const props = Vue.reactive({ projectId: 2, symbol: 'KHB', quoteSymbol: 'USDT', quoteAssetId: 2, price: '0.2', subscriptionOpen: true })
  const storage = new Map<string, string>()
  const writes: Array<{ intent: TokenSubscriptionIntent } & ReturnType<typeof deferred<void>>> = []
  let completed = 0; let unmount = () => {}
  const exports = execute(compileScript(descriptor, { id: 'subscription-stage-test' }).content, {
    vue: { ...Vue, onBeforeUnmount: (fn: () => void) => { unmount = fn } },
    'vue-i18n': { useI18n: () => ({ t: (key: string) => key }) },
    '@/stores/session': { useSessionStore: () => ({ isAuthenticated: true, identityScope: 'fixture' }) },
    '@/api/tokenIssuance': { subscribeToken: (_symbol: string, intent: TokenSubscriptionIntent) => {
      const write = { ...deferred<void>(), intent: { ...intent } }; writes.push(write); return write.promise
    } },
    '@/core/tokenIssuance': { tokenDecimal, tokenMultiply }, './TokenQuoteBalance.vue': {}, '@/core/tokenIssuanceMessages': {},
  }, { crypto: { randomUUID: () => 'original-subscription-key' }, sessionStorage: {
    setItem: (key: string, value: string) => storage.set(key, value), getItem: (key: string) => storage.get(key), removeItem: (key: string) => storage.delete(key),
  } })
  const scope = Vue.effectScope()
  const state = scope.run(() => exports.default.setup(props, { expose() {}, emit: () => { completed++ } })) as SubscriptionState
  return { state, props, storage, writes, completed: () => completed, stop: () => { unmount(); scope.stop() } }
}

test('a phase change preserves unknown subscription receipts and retries the original intent', async () => {
  const h = subscriptionHarness()
  try {
    h.state.quantity.value = '100'; h.state.review()
    const first = h.state.submit()
    h.props.subscriptionOpen = false
    h.writes[0]!.reject(new Error('response lost')); await first
    assert.equal(h.state.uncertain.value, true)
    const original = h.state.intent.value!
    const retry = h.state.submit()
    assert.deepEqual(h.writes[1]!.intent, original)
    h.writes[1]!.resolve(); await retry
    assert.equal(h.completed(), 1); assert.equal(h.state.success.value, true)
    assert.equal(h.storage.size, 0)
    h.state.quantity.value = '200'; h.state.review(); await h.state.submit()
    assert.equal(h.state.intent.value, null); assert.equal(h.writes.length, 2)
  } finally { h.stop() }
})

test('a reviewed but unsubmitted subscription cannot execute after its phase closes', async () => {
  const h = subscriptionHarness()
  try {
    h.state.quantity.value = '100'; h.state.review()
    h.props.subscriptionOpen = false; await h.state.submit()
    assert.equal(h.writes.length, 0)
    h.state.cancel(); assert.equal(h.state.intent.value, null)
    assert.equal(h.storage.size, 0)
  } finally { h.stop() }
})

test('project cards use v2 lock rules and planned listing time without falling back to legacy fields', () => {
  const { descriptor } = parse(readFileSync(new URL('../src/components/new-coin/NewCoinProjectCard.vue', import.meta.url), 'utf8'))
  const at = 1792529760000
  const policy = parseTokenPolicy({ version: 2, timezone: 'Asia/Hong_Kong', purchase_initial_price: '0.2', listing_reference_price: '0.2',
    subscription: { at: at - 4000, automatic: true }, distribution: { at: at - 3000, automatic: true },
    purchase: { at: at - 2000, automatic: true }, listing: { at, automatic: false }, lock: { kind: 'fixed', at }, fee: { kind: 'none' } })
  const props = Vue.reactive({ project: { ...project('distribution'), issuancePolicy: policy }, now: at - 1000 })
  const exports = execute(compileScript(descriptor, { id: 'project-card-stage-test' }).content, {
    vue: Vue, 'vue-i18n': { useI18n: () => ({ t: (key: string, params?: { count: number }) => `${key}${params ? `:${params.count}` : ''}`, locale: Vue.ref('en') }) },
    '@/components/AssetMark.vue': {}, '@/core/decimal': decimal, '@/core/financialDisplay': {},
    '@/core/format': { formatDateTime }, '@/core/newCoinPresentation': presentation,
  })
  const scope = Vue.effectScope()
  const state = scope.run(() => exports.default.setup(props, { expose() {}, emit() {} })) as {
    unlockLabel: Vue.ComputedRef<string>; timing: Vue.ComputedRef<string>
  }
  try {
    assert.equal(state.unlockLabel.value, formatDateTime(at))
    assert.equal(state.timing.value, formatDateTime(at))
    for (const [lock, label] of [
      [{ kind: 'none' }, 'newCoin.noLock'], [{ kind: 'on_purchase' }, 'newCoin.purchaseUnlock'],
      [{ kind: 'on_listing' }, 'newCoin.immediateUnlock'], [{ kind: 'weeks', count: 3 }, 'newCoin.lockWeeks:3'],
      [{ kind: 'months', count: 3 }, 'newCoin.lockMonths:3'],
    ] as Array<[TokenPolicy['lock'], string]>) {
      props.project.issuancePolicy.lock = lock
      assert.equal(state.unlockLabel.value, label)
    }
  } finally { scope.stop() }
})
