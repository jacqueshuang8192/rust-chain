import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import ts from 'typescript'
import * as Vue from 'vue'
import { decimalAdd, decimalSign, normalizeDecimalText as n } from '../src/core/decimal.ts'
import { walletTotal } from '../src/core/walletAmounts.ts'
import { estimateWalletAsset } from '../src/core/walletValuation.ts'
import { formatAmount } from '../src/core/format.ts'
import type { WalletAccount } from '../src/core/types.ts'
const flush = async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); await Vue.nextTick() }
const account = (symbol: string, available: string, locked = '0'): WalletAccount => ({
  assetId: symbol === 'USDT' ? 1 : 2, symbol, available: Number(available), frozen: 0, locked: Number(locked),
  availableText: n(available), frozenText: n('0'), lockedText: n(locked),
})
function harness() {
  const source = readFileSync(new URL('../src/views/HomeView.vue', import.meta.url), 'utf8')
  const computed = source.slice(source.indexOf('const assetEstimates ='), source.indexOf('const todayReturnPresentation ='))
  const load = source.slice(source.indexOf('async function loadAssetEstimate()'), source.indexOf('async function loadTodayReturn()'))
  const start = source.indexOf('watch(() => session.token, () => {\n  assetRequestVersion++')
  const lifecycle = source.slice(start, source.indexOf('watch(() => session.token', start + 10))
  const requests: { resolve(value: WalletAccount[]): void; reject(error: Error): void }[] = []
  const session = Vue.reactive({ token: 'a', isAuthenticated: true })
  const prices = Vue.reactive<Record<string, ReturnType<typeof n>>>({})
  let tick = () => {}, now = 1000
  const bindings = { ref: Vue.ref, computed: Vue.computed, watch: Vue.watch, session, marketStore: { tickerFor: (symbol: string) => prices[symbol] ? { lastPriceText: prices[symbol] } : undefined },
    decimalAdd, decimalSign, normalizeDecimalText: n, walletTotal, estimateWalletAsset, formatAmount,
    fetchWalletAccounts: () => new Promise<WalletAccount[]>((resolve, reject) => requests.push({ resolve, reject })),
    fetchMarginWallets: async () => ({ wallets: [account('USDT', '100')] }),
    document: { visibilityState: 'visible' }, Date: { now: () => now },
    setInterval: (fn: () => void) => { tick = fn; return 1 },
  }
  const code = ts.transpileModule(`let viewActive = true, assetRequestVersion = 0;
    const spotAccounts=ref([]),marginAccounts=ref([]),assetEstimateReady=ref(false),assetsLoading=ref(false),valuationClock=ref(Date.now());
    ${computed}\n${load}\n${lifecycle}
    return { displayedAssetAmount, totalAssetEstimate, loadAssetEstimate, stop(){viewActive=false;assetRequestVersion++} }`, {
    compilerOptions: { target: ts.ScriptTarget.ES2022 },
  }).outputText
  const scope = Vue.effectScope()
  const state = scope.run(() => new Function(...Object.keys(bindings), code)(...Object.values(bindings))) as {
    displayedAssetAmount: Vue.ComputedRef<string>; totalAssetEstimate: Vue.ComputedRef<string>; loadAssetEstimate(): Promise<void>; stop(): void
  }
  return { session, prices, requests, state, tick(time: number) { now = time; tick() }, stop() { state.stop(); scope.stop() } }
}
test('Home values pre-listing locked holdings without a spot listing and refreshes expiry/recovery', async () => {
  const h = harness()
  try {
    const token = { ...account('HIPPO', '0', '2500'), issuanceValuation: { priceUsdt: '0.4', expiresAt: 2000 } }
    h.requests[0]!.resolve([account('USDT', '1000'), token]); await flush()
    assert.equal(h.state.displayedAssetAmount.value, '2,100')
    assert.equal(h.prices['HIPPO/USDT'], undefined)
    h.tick(2001); assert.equal(h.state.displayedAssetAmount.value, '--')
    h.requests[1]!.resolve([account('USDT', '1000'), { ...token, issuanceValuation: { priceUsdt: '0.8', expiresAt: 4000 } }]); await flush()
    assert.equal(h.state.displayedAssetAmount.value, '3,100')
    h.tick(2500); assert.equal(h.state.displayedAssetAmount.value, '3,100', 'background reads do not flash empty')
    h.requests[2]!.reject(new Error('offline')); await flush(); assert.equal(h.state.displayedAssetAmount.value, '--')
    h.tick(3000); h.requests[3]!.resolve([account('USDT', '0'), account('UNPRICED', '0')]); await flush()
    assert.equal(h.state.displayedAssetAmount.value, '100', 'zero unpriced accounts do not hide known funds')
  } finally { h.stop() }
})
test('Home total rejects stale sessions and never replaces missing issuance prices with a ticker', async () => {
  const h = harness()
  h.session.token = 'b'; await flush()
  h.prices['HIPPO/USDT'] = n('999')
  h.requests[1]!.resolve([{ ...account('HIPPO', '1'), issuanceValuation: { priceUsdt: null, expiresAt: 2000 } }]); await flush()
  h.requests[0]!.resolve([account('USDT', '999999')]); await flush(); assert.equal(h.state.displayedAssetAmount.value, '--')
  h.session.token = ''; h.session.isAuthenticated = false; await flush(); assert.equal(h.state.totalAssetEstimate.value, '0')
  h.session.token = 'c'; h.session.isAuthenticated = true; await flush(); h.stop()
  h.requests[2]!.resolve([account('USDT', '99')]); await flush(); assert.equal(h.state.displayedAssetAmount.value, '--')
})
