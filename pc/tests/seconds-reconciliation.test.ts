import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import ts from 'typescript'
import { computed, proxyRefs, ref } from 'vue'
import { parse } from '@vue/compiler-sfc'
import * as adapters from '../src/api/backendAdapters.ts'

function harness() {
  let token = 'first'
  const requests: Array<{ kind: string; resolve: (value: unknown) => void }> = []
  const request = (kind: string) => () => new Promise(resolve => requests.push({ kind, resolve }))
  const api = {
    fetchSecondCurrentOrders: request('current'),
    fetchSecondHistoryOrders: request('history'),
    fetchSecondBalance: request('balance'),
    fetchSecondOrders: request('snapshot'),
  }
  const bindings = {
    exports: {} as Record<string, any>,
    require: (name: string) => {
      if (name === 'pinia') return { defineStore: (_name: string, setup: () => unknown) => setup }
      if (name === 'vue') return { ref }
      if (name.includes('authStorage')) return { readAuthToken: () => token }
      return api
    },
  }
  const source = readFileSync(new URL('../src/stores/second.ts', import.meta.url), 'utf8')
  const code = ts.transpileModule(source, { compilerOptions: {
    target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS,
  } }).outputText
  new Function(...Object.keys(bindings), code)(...Object.values(bindings))
  return { store: bindings.exports.useSecondStore(), requests, readToken: () => token, changeSession: () => { token = 'second' } }
}

function viewHarness(h: ReturnType<typeof harness>) {
  const bindings = {
    exports: {} as Record<string, any>,
    require: (name: string) => {
      if (name === 'vue') return { ref, computed, onMounted: () => {}, onUnmounted: () => {} }
      if (name === 'vue-router') return { useRoute: () => ({ params: {} }), useRouter: () => ({}) }
      if (name === 'vue-toastification') return { useToast: () => ({}) }
      if (name === 'vue-i18n') return { useI18n: () => ({ t: (key: string) => key }) }
      if (name.includes('stores/second')) return { useSecondStore: () => proxyRefs(h.store) }
      if (name.includes('stores/setting')) return { useSettingStore: () => ({}) }
      if (name.includes('useAuthRequired')) return { useAuthRequired: () => ({ isLoggedIn: ref(true) }) }
      if (name.includes('authStorage')) return { readAuthToken: h.readToken }
      return {}
    },
  }
  const { descriptor } = parse(readFileSync(new URL('../src/views/SecondOptions.vue', import.meta.url), 'utf8'))
  const code = ts.transpileModule(`${descriptor.scriptSetup!.content}
    export { reconcileOrders, resultData, showResultModal, symbol }`, { compilerOptions: {
    target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS,
  } }).outputText
  new Function(...Object.keys(bindings), code)(...Object.values(bindings))
  return bindings.exports
}

const snapshot = (orders: unknown[]) => ({ data: { data: orders } })
const flush = () => new Promise<void>(resolve => setImmediate(resolve))

test('seconds reads ignore stale symbols, sessions and unmounted views', async () => {
  const h = harness()
  const old = h.store.loadCurrentOrders('BTC/USDT')
  const latest = h.store.loadCurrentOrders('ETH/USDT')
  h.requests[1]!.resolve({ data: [{ id: 2, symbol: 'ETH/USDT' }] })
  await latest
  h.requests[0]!.resolve({ data: [{ id: 1, symbol: 'BTC/USDT' }] })
  await old
  assert.equal(h.store.currentOrders.value[0].id, 2)
  const balance = h.store.loadBalance()
  h.changeSession()
  h.requests[2]!.resolve({ data: '1000' })
  await balance
  assert.equal(h.store.balance.value, 0)
  let active = true
  const abandoned = h.store.loadCurrentOrders('ETH/USDT', () => active)
  active = false
  h.requests[3]!.resolve({ data: [] })
  await abandoned
  assert.equal(h.store.currentOrders.value[0].id, 2)
})

test('history page zero supersedes pending pagination without stale loading cleanup', async () => {
  const h = harness()
  const older = h.store.loadHistoryOrders('BTC/USDT', 1)
  const latest = h.store.loadHistoryOrders('ETH/USDT', 0)
  h.requests[0]!.resolve({ data: [{ id: 1 }] })
  await older
  assert.equal(h.store.loadingHistory.value, true)
  assert.deepEqual(h.store.historyOrders.value, [])
  h.requests[1]!.resolve({ data: [{ id: 2 }] })
  await latest
  assert.equal(h.store.loadingHistory.value, false)
  assert.deepEqual(h.store.historyOrders.value.map((order: { id: number }) => order.id), [2])
})

test('one snapshot reconciles many orders and excludes refunds, review and historical results', async () => {
  const h = harness()
  h.store.currentOrders.value = [1, 2, 3, 4, 5].map(id => ({ id, status: 'OPEN' }))
  const read = h.store.reconcileOrders('BTC/USDT')
  assert.deepEqual(h.requests.map(r => r.kind), ['snapshot'])
  h.requests[0]!.resolve(snapshot([
    { id: 1, status: 'CLOSE', result: 'WIN', profit: 8 },
    { id: 2, status: 'CLOSE', result: 'LOSE', profit: -10 },
    { id: 3, status: 'REFUNDED', result: 0, profit: 0 },
    { id: 4, status: 'MANUAL_REVIEW', result: 0 },
    { id: 5, status: 'OPEN' },
    { id: 6, status: 'CLOSE', result: 'WIN' },
  ]))
  assert.deepEqual((await read).map((o: { id: number }) => o.id), [1, 2])
  assert.deepEqual(h.store.currentOrders.value.map((o: { id: number }) => o.id), [5])
  assert.equal(h.store.historyOrders.value.length, 5)
  assert.equal(h.store.historyOrders.value.find((o: { id: number }) => o.id === 3).profit, 0)
  assert.equal(h.store.loadingHistory.value, false)
})

test('late snapshots cannot overwrite a newer read, another session or an abandoned view', async () => {
  for (const reason of ['newer', 'session', 'view']) {
    const h = harness()
    let active = true
    h.store.currentOrders.value = [{ id: 1, status: 'OPEN' }]
    const read = h.store.reconcileOrders('BTC/USDT', () => active)
    if (reason === 'newer') {
      const next = h.store.loadCurrentOrders('ETH/USDT')
      h.requests[1]!.resolve({ data: [{ id: 2, status: 'OPEN' }] })
      await next
    } else if (reason === 'session') h.changeSession()
    else active = false
    h.requests[0]!.resolve(snapshot([{ id: 1, status: 'CLOSE', result: 'WIN' }]))
    assert.deepEqual(await read, [])
    assert.equal(h.store.currentOrders.value[0].id, reason === 'newer' ? 2 : 1)
    assert.deepEqual(h.store.historyOrders.value, [])
  }
})

test('event bursts retain one follow-up snapshot and preserve the last settlement result', async () => {
  const h = harness()
  const view = viewHarness(h)
  h.store.currentOrders.value = [1, 2, 3].map(id => ({ id, status: 'OPEN' }))
  const first = view.reconcileOrders()
  assert.equal(view.reconcileOrders(), first)
  assert.equal(view.reconcileOrders(), first)
  const orders = [
    { id: 1, symbol: 'BTC/USDT', status: 'CLOSE', result: 'WIN', profit: 8, amount: 10 },
    { id: 2, symbol: 'BTC/USDT', status: 'CLOSE', result: 'LOSE', profit: -10, amount: 10 },
    { id: 3, symbol: 'BTC/USDT', status: 'REFUNDED', result: 0, profit: 0, amount: 10 },
  ]
  h.requests[0]!.resolve(snapshot(orders))
  await flush()
  assert.deepEqual(h.requests.map(r => r.kind), ['snapshot', 'balance'])
  assert.equal(view.resultData.value.id, 2)
  assert.equal(view.resultData.value.isLose, true)
  assert.equal(view.showResultModal.value, true)
  h.requests[1]!.resolve({ data: '123' })
  await flush()
  assert.deepEqual(h.requests.map(r => r.kind), ['snapshot', 'balance', 'snapshot'])
  h.requests[2]!.resolve(snapshot(orders))
  await flush()
  h.requests[3]!.resolve({ data: '123' })
  await first
  assert.equal(h.requests.length, 4)
  assert.equal(view.resultData.value.id, 2)
  assert.equal(h.store.balance.value, 123)
})

test('snapshot API makes one list request and reuses symbol and status adapters', async () => {
  let calls = 0
  const bindings = {
    exports: {} as Record<string, any>,
    require: (name: string) => {
      if (name === './backendAdapters') return adapters
      if (name === './request') return { default: { instance: { get: async () => {
        calls++
        return { data: { orders: [
          { id: 1, symbol: 'BTC-USDT', status: 'opened', stake_amount: '10', payout_rate: '0.8' },
          { id: 2, symbol: 'BTC-USDT', status: 'refunded', stake_amount: '10', payout_rate: '0.8' },
          { id: 3, symbol: 'ETH-USDT', status: 'opened', stake_amount: '10', payout_rate: '0.8' },
        ] } }
      } } } }
      return {}
    },
  }
  const source = readFileSync(new URL('../src/api/second.ts', import.meta.url), 'utf8')
  const code = ts.transpileModule(source, { compilerOptions: {
    target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS,
  } }).outputText
  new Function(...Object.keys(bindings), code)(...Object.values(bindings))
  const result = await bindings.exports.fetchSecondOrders({ symbol: 'BTC/USDT' })
  assert.equal(calls, 1)
  assert.deepEqual(result.data.data.map((o: { id: number; status: string }) => [o.id, o.status]), [[1, 'OPEN'], [2, 'REFUNDED']])
})
