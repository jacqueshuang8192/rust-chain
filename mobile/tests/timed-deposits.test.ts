import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { compileScript, parse } from '@vue/compiler-sfc'
import ts from 'typescript'
import * as Vue from 'vue'
import * as depositIcons from '../src/core/depositIcons.ts'
import * as contract from '../src/core/timedDeposit.ts'
import type { DepositOption, DepositOrder } from '../src/core/timedDeposit.ts'

const option: DepositOption = { asset_symbol: 'ETH', network: 'eth', display_name: 'Ethereum', precision_scale: 18, min_deposit_amount: '0', deposit_fee: '0', required_confirmations: 12 }
const order = (amount = '100', overrides: Partial<DepositOrder> = {}): DepositOrder => ({ id: 1, order_no: 'CD1', user_id: 9, asset_symbol: 'ETH', network: 'eth', address: `0x${'1'.repeat(40)}`, amount, fee_amount: '0', paid_amount: null, required_confirmations: 12, status: 'pending', tx_hash: null, created_at: Date.now(), expires_at: Date.now() + 1800000, paid_at: null, cancelled_at: null, server_time: Date.now(), ...overrides })
interface State {
  amount: Vue.Ref<string>; order: Vue.Ref<DepositOrder | null>; qr: Vue.Ref<string>; payable: Vue.ComputedRef<boolean>; remaining: Vue.ComputedRef<number>; now: Vue.Ref<number>; error: Vue.Ref<string>; history: Vue.Ref<DepositOrder[]>;
  assetChoices: Vue.ComputedRef<{ value: string; logoUrl?: string; fallbackLogoUrl?: string }[]>;
  selection: Vue.Ref<string>; option: Vue.ComputedRef<DepositOption | undefined>; networkChoices: Vue.ComputedRef<{ value: string }[]>; chooseAsset(symbol: string): void; chooseNetwork(key: string): void; newDeposit(): void;
  submit(): Promise<void>; refresh(): Promise<void>; load(): Promise<void>; selectHistory(order: DepositOrder): void; cancelOrder(): Promise<void>; askCancel(): void; confirmingCancel: Vue.Ref<boolean>; cancelError: Vue.Ref<string>
}
const flush = async () => { for (let i = 0; i < 5; i += 1) { await new Promise(resolve => setImmediate(resolve)); await Vue.nextTick() } }
function fixture(packageName: 'mobile' | 'pc', initial: DepositOrder[] = []) {
  const writes: { amount: string; key: string }[] = []
  let fail = false
  let create = async (_option: DepositOption, amount: string, key: string) => { writes.push({ amount, key }); if (fail) throw new Error('response lost'); return order(amount) }
  let cancellations = 0
  let cancel = async (_id: number) => { cancellations += 1; return order('100', { status: 'cancelled', cancelled_at: Date.now() }) }
  const walletApi = { fetchDepositAssets: async (): Promise<{ symbol: string; name?: string; logoUrl?: string }[]> => [] }
  const api = {
    cancelTimedDepositOrder: (id: number) => cancel(id),
    fetchTimedDepositOptions: async () => [option],
    fetchTimedDepositOrders: async () => ({ items: initial, total: initial.length }),
    fetchTimedDepositOrder: async () => order(),
    createTimedDepositOrder: (...args: [DepositOption, string, string]) => create(...args),
  }
  const source = readFileSync(new URL(`../../${packageName}/src/components/TimedDepositPanel.vue`, import.meta.url), 'utf8')
  const compiled = compileScript(parse(source).descriptor, { id: 'timed-deposit-behavior' })
  const output = ts.transpileModule(compiled.content, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } }).outputText
  const cleanup: (() => void)[] = []; const timers = new Set<number>(); const memory = new Map<string, string>()
  const modules: Record<string, unknown> = { './DepositSelectionSheet.vue': {}, '../api/wallet': walletApi, '../core/depositIcons': depositIcons, vue: { ...Vue, onUnmounted: (fn: () => void) => cleanup.push(fn) }, 'vue-i18n': { useI18n: () => ({ t: (key: string) => key, locale: Vue.ref('zh-CN') }) }, '../core/modalDialog': { useModalDialog: () => ({ trapFocus() {} }) }, qrcode: { toDataURL: async () => 'data:image/png;base64,test' }, '../api/timedDeposits': api, '../core/timedDeposit': contract }
  const exports: { default?: { setup(props: object, context: object): State } } = {}
  new Function('require', 'exports', 'sessionStorage', 'setInterval', 'clearInterval', output)(
    (name: string) => { assert.ok(Object.hasOwn(modules, name), name); return modules[name] }, exports,
    { getItem: (key: string) => memory.get(key) ?? null, setItem: (key: string, value: string) => memory.set(key, value), removeItem: (key: string) => memory.delete(key) },
    () => { const id = timers.size + 1; timers.add(id); return id }, (id: number) => timers.delete(id),
  )
  const props = Vue.reactive({ identity: 'user:9', asset: 'ETH', network: 'eth', orderId: '' })
  const scope = Vue.effectScope()
  const state = scope.run(() => exports.default!.setup(props, { expose() {}, emit() {} }))!
  return { state, writes, props, timers, memory, api, walletApi, get cancellations() { return cancellations }, cancel(fn: typeof cancel) { cancel = fn }, fail(value: boolean) { fail = value }, create(fn: typeof create) { create = fn }, stop() { cleanup.forEach(fn => fn()); scope.stop() } }
}

for (const packageName of ['mobile', 'pc'] as const) {
  test(`${packageName}: order creation requires explicit exact token input, retries retain identity`, async () => {
    const f = fixture(packageName)
    try {
      await flush(); assert.equal(f.writes.length, 0); assert.equal(f.state.order.value?.id ?? null, null)
      f.state.amount.value = '9007199254740993.000000000000000001'
      f.fail(true); await f.state.submit(); assert.equal(f.writes.length, 1)
      assert.equal(f.writes[0].amount, '9007199254740993.000000000000000001')
      f.fail(false); await f.state.submit(); assert.equal(f.writes[0].key, f.writes[1].key)
      assert.equal(f.state.order.value?.amount, '9007199254740993.000000000000000001')
      assert.equal([...f.memory.keys()].filter(key => key.startsWith('timed-deposit:')).length, 0); assert.equal(f.state.payable.value, true)
      await f.state.submit(); assert.equal(f.writes.length, 2)
      f.state.now.value = f.state.order.value!.expires_at + 10; await Vue.nextTick()
      assert.equal(f.state.payable.value, false); assert.equal(f.state.qr.value, '')
    } finally { f.stop(); assert.equal(f.timers.size, 0) }
  })
  test(`${packageName}: refresh restores pending order and session change rejects late creation`, async () => {
    const f = fixture(packageName, [order('50')])
    try { await flush(); assert.equal(f.state.order.value?.amount, '50'); assert.equal(f.writes.length, 0) } finally { f.stop() }
    const pending = fixture(packageName)
    try {
      await flush(); let resolve!: (value: DepositOrder) => void
      pending.create(() => new Promise(done => { resolve = done }))
      pending.state.amount.value = '50'; const writing = pending.state.submit()
      pending.props.identity = ''; await Vue.nextTick(); resolve(order('50')); await writing; await flush()
      assert.equal(pending.state.order.value, null); assert.equal(pending.state.qr.value, ''); assert.equal(pending.state.history.value.length, 0)
    } finally { pending.stop() }
  })
}

test('deposit contract preserves decimals and rejects numeric money, excess precision and unsafe IDs', () => {
  assert.equal(contract.exactDepositAmount('1e-18'), '0.000000000000000001')
  assert.equal(contract.exactDepositAmount('100.0000000000000000000'), '100')
  assert.throws(() => contract.exactDepositAmount(100))
  assert.throws(() => contract.exactDepositAmount('0.0000000000000000001'))
  assert.throws(() => contract.validateDepositDraft('0.1', { ...option, deposit_fee: '0.1' }))
  assert.throws(() => contract.mapDepositOrder({ ...order(), id: Number.MAX_SAFE_INTEGER + 1 }))
  assert.throws(() => contract.mapDepositOrder({ ...order(), amount: 100 }))
  assert.equal(contract.mapDepositOrder(order('1', { status: 'cancelled', cancelled_at: 1000 })).cancelled_at, 1000)
  assert.throws(() => contract.mapDepositOrder({ ...order(), cancelled_at: undefined }))
  assert.equal(contract.mapDepositOrder(order('9007199254740993.000000000000000001')).amount, '9007199254740993.000000000000000001')
})

test('PC recharge uses the persisted login generation for opaque access tokens', () => {
  const source = readFileSync(new URL('../../pc/src/views/User/Recharge.vue', import.meta.url), 'utf8')
  const compiled = compileScript(parse(source).descriptor, { id: 'pc-recharge-session' })
  const output = ts.transpileModule(compiled.content, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } }).outputText
  const user = Vue.reactive({ token: 'opaque-sa-token', isLoggedIn: true })
  let sessionScope = 'login-generation-one'
  const modules: Record<string, unknown> = {
    vue: { ...Vue, onMounted() {} },
    '@/components/TimedDepositPanel.vue': {}, '@/stores/user': { useUserStore: () => user },
    '@/utils/authStorage': { readAuthSessionScope: () => sessionScope },
    '@iconify/vue': {}, '@/api/wallet': {}, 'vue-toastification': { useToast: () => ({}) },
    'vue-i18n': { useI18n: () => ({ t: (key: string) => key }) },
  }
  const exports: { default?: { setup(props: object, context: object): { depositIdentity: Vue.ComputedRef<string> } } } = {}
  new Function('require', 'exports', output)((name: string) => { assert.ok(Object.hasOwn(modules, name), name); return modules[name] }, exports)
  const state = exports.default!.setup({}, { expose() {} })
  assert.equal(state.depositIdentity.value, sessionScope)
  user.token = ''; assert.equal(state.depositIdentity.value, '')
  sessionScope = 'login-generation-two'; user.token = 'another-opaque-token'
  assert.equal(state.depositIdentity.value, sessionScope)
})


test('mobile: cancellation needs confirmation, hides QR, and ignores duplicate submission', async () => {
  const f = fixture('mobile', [order()])
  try {
    await flush(); await f.state.cancelOrder(); assert.equal(f.cancellations, 0)
    f.state.askCancel(); assert.equal(f.state.confirmingCancel.value, true)
    await Promise.all([f.state.cancelOrder(), f.state.cancelOrder()]); await flush()
    assert.equal(f.cancellations, 1); assert.equal(f.state.order.value?.status, 'cancelled')
    assert.equal(f.state.payable.value, false); assert.equal(f.state.qr.value, '')
    assert.equal(f.state.confirmingCancel.value, false)
  } finally { f.stop() }
})

test('mobile: rejected cancellation preserves order with feedback; lost successful reply re-reads state', async () => {
  const f = fixture('mobile', [order()])
  try {
    await flush(); f.cancel(async () => { throw new Error('conflict') }); f.state.askCancel()
    await f.state.cancelOrder(); assert.equal(f.state.order.value?.status, 'pending')
    assert.equal(f.state.cancelError.value, 'timedDeposit.cancelFailed')
    f.api.fetchTimedDepositOrder = async () => order('100', { status: 'cancelled', cancelled_at: Date.now() })
    await f.state.cancelOrder(); await flush()
    assert.equal(f.state.order.value?.status, 'cancelled'); assert.equal(f.state.qr.value, '')
    assert.equal(f.state.confirmingCancel.value, false)
  } finally { f.stop() }
})

test('mobile: late cancellation response cannot leak across sessions, historical detail works without options', async () => {
  const f = fixture('mobile', [order()])
  try {
    await flush(); let resolve!: (value: DepositOrder) => void
    f.cancel(() => new Promise(done => { resolve = done })); f.state.askCancel()
    const pending = f.state.cancelOrder(); f.props.identity = ''; await Vue.nextTick()
    resolve(order('100', { status: 'cancelled', cancelled_at: Date.now() })); await pending; await flush()
    assert.equal(f.state.order.value, null); assert.equal(f.state.confirmingCancel.value, false)
    f.api.fetchTimedDepositOptions = async () => { throw new Error('network unavailable') }
    f.api.fetchTimedDepositOrder = async () => order('50', { id: 8, status: 'cancelled', cancelled_at: Date.now() })
    f.props.identity = 'user:10'; f.props.orderId = '8'; await flush()
    assert.equal(Vue.unref(f.state.order)?.id, 8); assert.equal(f.state.payable.value, false)
    f.props.orderId = 'invalid'; await flush(); assert.equal(f.state.order.value, null)
  } finally { f.stop() }
})


test('mobile: asset and network selection stays configured, clears amounts, and preserves unchanged drafts', async () => {
  const f = fixture('mobile')
  try {
    await flush()
    f.api.fetchTimedDepositOptions = async () => [option, { ...option, asset_symbol: 'USDT', precision_scale: 6 }, { ...option, asset_symbol: 'USDT', network: 'tron', display_name: 'Tron', precision_scale: 6 }]
    await f.state.load()
    f.state.amount.value = '1.25'; f.state.chooseAsset('ETH')
    assert.equal(f.state.amount.value, '1.25')
    f.state.chooseAsset('USDT'); await flush()
    assert.equal(f.state.selection.value, 'USDT:eth'); assert.equal(f.state.amount.value, '')
    assert.deepEqual(f.state.networkChoices.value.map(item => item.value), ['USDT:eth', 'USDT:tron'])
    f.state.amount.value = '100'; f.state.chooseNetwork('USDT:tron'); await flush()
    assert.equal(f.state.selection.value, 'USDT:tron'); assert.equal(f.state.amount.value, '')
    f.state.chooseNetwork('ETH:eth'); f.state.chooseAsset('FAKE')
    assert.equal(f.state.selection.value, 'USDT:tron')
    f.state.amount.value = '50'; await f.state.submit()
    f.state.chooseAsset('ETH'); f.state.chooseNetwork('USDT:eth')
    assert.equal(f.state.selection.value, 'USDT:tron')
  } finally { f.stop() }
})

test('mobile: explicit new deposit clears a terminal order and rejects a late old-order poll', async () => {
  const f = fixture('mobile', [order()])
  try {
    await flush(); f.state.newDeposit(); assert.ok(f.state.order.value)
    f.state.askCancel(); await f.state.cancelOrder()
    let resolve!: (value: DepositOrder) => void
    f.api.fetchTimedDepositOrder = () => new Promise(done => { resolve = done })
    const pending = f.state.refresh()
    f.state.newDeposit(); assert.equal(f.state.order.value, null)
    resolve(order()); await pending; await flush()
    assert.equal(f.state.order.value, null); assert.equal(f.state.amount.value, '')
  } finally { f.stop() }
})


test('mobile: rapid selection changes ignore the previous pair history response', async () => {
  const f = fixture('mobile')
  try {
    await flush()
    f.api.fetchTimedDepositOptions = async () => [option, { ...option, asset_symbol: 'USDT' }]
    await f.state.load()
    let resolve!: (value: { items: DepositOrder[]; total: number }) => void
    f.api.fetchTimedDepositOrders = () => new Promise(done => { resolve = done })
    f.state.chooseAsset('USDT')
    const finishOld = resolve
    f.state.chooseAsset('ETH')
    finishOld({ items: [order('50', { asset_symbol: 'USDT' })], total: 1 })
    resolve({ items: [], total: 0 }); await flush()
    assert.equal(f.state.selection.value, 'ETH:eth'); assert.equal(f.state.order.value, null)
    assert.equal(f.state.history.value.length, 0)
  } finally { f.stop() }
})


test('mobile: current-session asset/network choice survives a reload and rejects obsolete options', async () => {
  const f = fixture('mobile')
  try {
    await flush()
    const usdt = { ...option, asset_symbol: 'USDT', network: 'tron' }
    f.api.fetchTimedDepositOptions = async () => [option, usdt]
    f.props.asset = ''; f.props.network = ''; await flush()
    f.state.chooseAsset('USDT'); await flush()
    await f.state.load()
    assert.equal(f.state.selection.value, 'USDT:tron')
    f.props.identity = 'user:10'; await flush()
    assert.equal(f.state.selection.value, 'ETH:eth')
    f.props.identity = 'user:9'; f.api.fetchTimedDepositOptions = async () => [option]; await flush()
    assert.equal(f.state.selection.value, 'ETH:eth')
  } finally { f.stop() }
})


test('mobile: configured asset logos win and metadata failure cannot disable valid deposit options', async () => {
  const f = fixture('mobile')
  try {
    await flush()
    f.walletApi.fetchDepositAssets = async () => [{ symbol: 'ETH', name: 'Ethereum', logoUrl: '/configured-eth.png' }]
    await f.state.load()
    assert.equal(f.state.assetChoices.value[0].logoUrl, '/configured-eth.png')
    assert.equal(f.state.assetChoices.value[0].fallbackLogoUrl, '/deposit-icons/eth.png')
    f.walletApi.fetchDepositAssets = async () => { throw new Error('metadata unavailable') }
    await f.state.load()
    assert.equal(f.state.selection.value, 'ETH:eth')
    assert.equal(f.state.assetChoices.value[0].logoUrl, undefined)
    assert.equal(f.state.assetChoices.value[0].fallbackLogoUrl, '/deposit-icons/eth.png')
    f.state.amount.value = '1'; await f.state.submit(); assert.equal(f.writes.length, 1)
  } finally { f.stop() }
})
