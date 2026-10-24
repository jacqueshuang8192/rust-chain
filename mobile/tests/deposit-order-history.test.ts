import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { compileScript, parse } from '@vue/compiler-sfc'
import ts from 'typescript'
import * as Vue from 'vue'
import type { DepositOrder, DepositPage } from '../src/core/timedDeposit.ts'

const item = (id: number): DepositOrder => ({ id, order_no: `CD${id}`, user_id: 9, asset_symbol: 'ETH', network: 'eth', address: '0x1111111111111111111111111111111111111111', amount: '1', fee_amount: '0', paid_amount: null, required_confirmations: 12, status: 'cancelled', tx_hash: null, created_at: 1000, expires_at: 1801000, paid_at: null, cancelled_at: 2000, server_time: 3000 })
const flush = async () => { for (let n = 0; n < 4; n++) { await new Promise(resolve => setImmediate(resolve)); await Vue.nextTick() } }
function fixture() {
  const calls: number[] = []; const cleanup: (() => void)[] = []
  let read = async (offset: number): Promise<DepositPage> => ({ items: [item(offset + 1)], total: 25 })
  const compiled = compileScript(parse(readFileSync(new URL('../src/components/DepositOrderHistory.vue', import.meta.url), 'utf8')).descriptor, { id: 'deposit-history' })
  const output = ts.transpileModule(compiled.content, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } }).outputText
  const modules: Record<string, unknown> = { vue: { ...Vue, onUnmounted: (fn: () => void) => cleanup.push(fn) }, 'vue-i18n': { useI18n: () => ({ t: (key: string) => key, locale: Vue.ref('zh-CN') }) }, '../api/timedDeposits': { fetchTimedDepositOrders: (_asset: string, _network: string, offset: number) => { calls.push(offset); return read(offset) } } }
  interface State { items: Vue.Ref<DepositOrder[]>; total: Vue.Ref<number>; offset: Vue.Ref<number>; error: Vue.Ref<string>; load(offset?: number): Promise<void>; move(direction: -1 | 1): void }
  const exports: { default?: { setup(props: object, context: object): State } } = {}
  new Function('require', 'exports', output)((name: string) => { assert.ok(Object.hasOwn(modules, name), name); return modules[name] }, exports)
  const scope = Vue.effectScope(); const props = Vue.reactive({ identity: 'user:9' })
  const state = scope.run(() => exports.default!.setup(props, { expose() {}, emit() {} }))!
  return { state, props, calls, read(fn: typeof read) { read = fn }, stop() { cleanup.forEach(fn => fn()); scope.stop() } }
}

test('history loads independent pages, failed navigation retains offset, and retry recovers', async () => {
  const f = fixture()
  try {
    await flush(); assert.deepEqual(f.calls, [0]); assert.equal(f.state.items.value[0].id, 1)
    f.read(async () => { throw new Error('offline') }); f.state.move(1); await flush()
    assert.equal(f.state.offset.value, 0); assert.equal(f.state.error.value, 'timedDeposit.historyFailed')
    f.read(async offset => ({ items: [item(offset + 1)], total: 25 })); f.state.move(1); await flush()
    assert.equal(f.state.offset.value, 20); assert.equal(f.state.items.value[0].id, 21)
    f.state.move(1); await flush(); assert.equal(f.calls.length, 3)
    f.state.move(-1); await flush(); assert.equal(f.state.offset.value, 0)
  } finally { f.stop() }
})

test('history clears previous user data and rejects stale reads on logout', async () => {
  const f = fixture()
  try {
    await flush(); let resolve!: (page: DepositPage) => void
    f.read(() => new Promise(done => { resolve = done })); const pending = f.state.load()
    f.props.identity = ''; await Vue.nextTick(); resolve({ items: [item(99)], total: 1 }); await pending
    assert.deepEqual(f.state.items.value, []); assert.equal(f.state.total.value, 0)
    const count = f.calls.length; await f.state.load(); assert.equal(f.calls.length, count)
  } finally { f.stop() }
})


test('history ignores an in-flight response after unmount', async () => {
  const f = fixture(); await flush()
  let resolve!: (page: DepositPage) => void
  f.read(() => new Promise(done => { resolve = done }))
  const pending = f.state.load(20); f.stop()
  resolve({ items: [item(99)], total: 100 }); await pending
  assert.equal(f.state.items.value[0].id, 1); assert.equal(f.state.offset.value, 0)
})
