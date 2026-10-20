import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import ts from 'typescript'
import * as Vue from 'vue'
import { compileScript, parse } from '@vue/compiler-sfc'
import * as contract from '../src/core/tokenIssuance.ts'
const flush = async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); await Vue.nextTick() }
function harness(app: 'mobile' | 'pc') {
  const path = app === 'mobile' ? '../src/components/TokenQuoteBalance.vue' : '../../pc/src/components/TokenQuoteBalance.vue'
  const { descriptor } = parse(readFileSync(new URL(path, import.meta.url), 'utf8'))
  const source = ts.transpileModule(compileScript(descriptor, { id: 'quote-balance-test' }).content, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
  }).outputText
  const requests: { id: number; symbol: string; resolve(value: string): void; reject(error: Error): void }[] = []
  const props = Vue.reactive({ owner: 'user-a', assetId: 1, symbol: 'USDT', revision: 0 })
  let unmount = () => {}, tick = () => {}, cleared = false
  const document = { visibilityState: 'visible' }
  const modules: Record<string, unknown> = {
    vue: { ...Vue, onBeforeUnmount: (fn: () => void) => { unmount = fn } },
    'vue-i18n': { useI18n: () => ({ t: (key: string) => key }) },
    '@/core/tokenIssuance': contract, '@/core/tokenIssuanceMessages': {},
    '@/api/tokenIssuance': { fetchTokenQuoteBalance: (id: number, symbol: string) => new Promise<string>((resolve, reject) => requests.push({ id, symbol, resolve, reject })) },
  }
  const exports: Record<string, any> = {}
  new Function('require', 'exports', 'document', 'setInterval', 'clearInterval', source)(
    (id: string) => { assert.ok(id in modules, id); return modules[id] }, exports, document,
    (fn: () => void) => { tick = fn; return 1 }, () => { cleared = true },
  )
  const scope = Vue.effectScope()
  const state = scope.run(() => exports.default.setup(props, { expose() {} })) as {
    balance: Vue.Ref<string | null>; loading: Vue.Ref<boolean>; failed: Vue.Ref<boolean>; load(): Promise<void>
  }
  return { props, state, requests, document, tick: () => tick(), stop() { unmount(); scope.stop(); assert.equal(cleared, true) } }
}
for (const app of ['pc'] as const) {
  test(`${app}: quote balance preserves exact spendable funds and refreshes after a trade`, async () => {
    const h = harness(app)
    try {
      assert.deepEqual([h.requests[0]!.id, h.requests[0]!.symbol], [1, 'USDT'])
      h.requests[0]!.resolve('9007199254740993.000000000000000001'); await flush()
      assert.equal(h.state.balance.value, '9007199254740993.000000000000000001')
      h.props.revision++; await flush(); h.requests[1]!.resolve('0'); await flush()
      assert.equal(h.state.balance.value, '0')
      h.tick(); h.tick(); assert.equal(h.requests.length, 3, 'no overlapping timer requests')
      h.requests[2]!.reject(new Error('offline')); await flush()
      assert.equal(h.state.balance.value, null); assert.equal(h.state.failed.value, true)
      h.tick(); h.requests[3]!.resolve('12.5'); await flush(); assert.equal(h.state.balance.value, '12.5')
      h.document.visibilityState = 'hidden'; h.tick(); assert.equal(h.requests.length, 4)
    } finally { h.stop() }
  })
  test(`${app}: quote balance ignores obsolete assets, logout and unmounted requests`, async () => {
    const h = harness(app)
    h.props.assetId = 2; h.props.symbol = 'USDC'; await flush()
    h.requests[1]!.resolve('25'); await flush(); h.requests[0]!.resolve('999'); await flush()
    assert.equal(h.state.balance.value, '25')
    h.tick(); h.props.owner = ''; await flush(); assert.equal(h.state.balance.value, null)
    h.requests[2]!.resolve('888'); await flush(); assert.equal(h.state.balance.value, null)
    h.props.owner = 'user-b'; await flush(); h.stop()
    h.requests[3]!.resolve('777'); await flush(); assert.equal(h.state.balance.value, null)
  })
}

test('mobile detail reuses wallet reads for issuance balance, periodic refresh and session isolation', async () => {
  const { createSessionRequestLifecycle } = await import('../src/core/sessionRequest.ts')
  const { decimalTextFromBoundary, normalizeDecimalText: n } = await import('../src/core/decimal.ts')
  const source = readFileSync(new URL('../src/views/NewCoinDetailView.vue', import.meta.url), 'utf8')
  const selected = source.slice(source.indexOf('const selectedAccount ='), source.indexOf('const amountText ='))
  const available = source.slice(source.indexOf('const availableText ='), source.indexOf('// Both subscription'))
  const lifecycle = source.slice(source.indexOf('const accountRequests ='), source.indexOf('watch(() => props.symbol'))
  const project = Vue.ref<{ id: number; quoteAssetId: number; issuancePolicy: object } | null>(null)
  const refreshing = Vue.ref(true), session = Vue.reactive({ isAuthenticated: true, identityScope: 'a' })
  const requests: { resolve(value: unknown[]): void; reject(error: Error): void }[] = []
  const bindings = { computed: Vue.computed, ref: Vue.ref, watch: Vue.watch, project, refreshing, session,
    createSessionRequestLifecycle, decimalTextFromBoundary, refreshProject: async () => {},
    fetchWalletAccounts: () => new Promise<unknown[]>((resolve, reject) => requests.push({ resolve, reject })),
  }
  const code = ts.transpileModule(`const accounts=ref([]), accountsReady=ref(false), error=ref('');
    ${selected}\n${available}\n${lifecycle}
    return { availableText, loadAccounts, stop:()=>accountRequests.stop() }`, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText
  const scope = Vue.effectScope()
  const state = scope.run(() => new Function(...Object.keys(bindings), code)(...Object.values(bindings))) as {
    availableText: Vue.ComputedRef<string | null>; loadAccounts(): Promise<void>; stop(): void
  }
  try {
    project.value = { id: 7, quoteAssetId: 2, issuancePolicy: {} }; refreshing.value = false; await flush()
    requests[0]!.resolve([{ assetId: 1, symbol: 'USDT', availableText: n('9999') }, { assetId: 2, symbol: 'USDC', availableText: n('1234.000000000000000001'), frozenText: n('500') }]); await flush()
    assert.equal(state.availableText.value, '1234.000000000000000001')
    refreshing.value = true; await flush(); refreshing.value = false; await flush()
    assert.equal(state.availableText.value, '1234.000000000000000001', 'refresh keeps current value visible')
    requests[1]!.reject(new Error('offline')); await flush(); assert.equal(state.availableText.value, null)
    const retry = state.loadAccounts(); requests[2]!.resolve([]); await retry
    assert.equal(state.availableText.value, '0', 'successful empty wallet differs from unavailable balance')
    const old = state.loadAccounts(); session.identityScope = 'b'; await flush(); assert.equal(state.availableText.value, null)
    requests[3]!.resolve([{assetId:2,availableText:n('999')}]); await old; assert.equal(state.availableText.value, null)
    requests[4]!.resolve([{assetId:2,availableText:n('10')}]); await flush(); assert.equal(state.availableText.value, '10')
    session.isAuthenticated = false; session.identityScope = ''; await flush(); assert.equal(state.availableText.value, null)
  } finally { state.stop(); scope.stop() }
})
