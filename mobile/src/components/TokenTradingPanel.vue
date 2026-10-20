<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useSessionStore } from '@/stores/session'
import SecondsChartWorkspace from '@/components/SecondsChartWorkspace.vue'
import { confirmToken, fetchTokenHoldings, purchaseToken, quoteToken } from '@/api/tokenIssuance'
import { parseTokenQuote, tokenDecimal, tokenReturn, type TokenHolding, type TokenQuote, type TokenQuoteIntent } from '@/core/tokenIssuance'
import { tokenIssuanceMessages } from '@/core/tokenIssuanceMessages'

const props = defineProps<{ projectId?: number; symbol?: string; quoteSymbol?: string; pairSymbol?: string; stage?: string }>()
const { t, locale } = useI18n({ useScope: 'local', messages: tokenIssuanceMessages })
const router = useRouter()
const session = useSessionStore()
const owner = computed(() => session.isAuthenticated ? session.identityScope : '')
const holdings = ref<TokenHolding[]>([])
const quantity = ref('')
const before = ref<number | undefined>()
const next = ref<number | null>(null)
const busy = ref(false)
const loading = ref(false)
const error = ref('')
const notice = ref('')
const recoveryBlocked = ref(false)
const clock = ref(Date.now())
const quote = ref<TokenQuote | null>(null)
type Pending = { mode?: 'market'; project: number; symbol: string; quoteSymbol: string; intent: TokenQuoteIntent; quote?: TokenQuote; uncertain: boolean }
const pending = ref<Pending | null>(null)
let generation = 0
let disposed = false
const visible = computed(() => holdings.value.filter(h => !props.projectId || h.holding.project_id === props.projectId))
const expired = computed(() => !!quote.value && clock.value >= quote.value.expires_at)
const date = (value: number | null) => value ? new Date(value).toLocaleString(locale.value) : '—'
const persistenceKey = () => `token-trade:${owner.value}:${props.projectId || 'all'}`
function save(value: Pending | null) {
  try { if (value) sessionStorage.setItem(persistenceKey(), JSON.stringify(value)); else sessionStorage.removeItem(persistenceKey()) }
  catch { throw new Error(t('token.unavailableStorage')) }
  pending.value = value
}
function valid(g: number) { return !disposed && generation === g }
async function load() {
  if (!owner.value || loading.value) return
  const g = generation; loading.value = true
  try { const rows = await fetchTokenHoldings(props.projectId, before.value); if (valid(g)) { holdings.value = rows.holdings; next.value = rows.next; error.value = '' } }
  catch { if (valid(g)) error.value = t('token.failure') }
  finally { if (valid(g)) loading.value = false }
}
async function buy() {
  if (busy.value || pending.value || !owner.value || recoveryBlocked.value || props.stage !== 'purchase') return
  try {
    const amount = tokenDecimal(quantity.value.trim())
    if (!/[1-9]/.test(amount)) throw new Error(t('token.failure'))
    save({ mode: 'market', project: props.projectId!, symbol: props.symbol!, quoteSymbol: props.quoteSymbol || '', intent: { side: 'buy', quantity: amount, idempotency_key: crypto.randomUUID() }, uncertain: true })
    await executePurchase()
  } catch (e) { error.value = e instanceof Error ? e.message : t('token.failure') }
}
async function executePurchase() {
  const p = pending.value
  if (!p || p.mode !== 'market' || p.intent.side !== 'buy' || busy.value) return
  const g = generation
  try { save({ ...p, uncertain: true }) } catch { error.value = t('token.unavailableStorage'); return }
  busy.value = true; error.value = ''; notice.value = ''
  try {
    const receipt = await purchaseToken(p.symbol, p.project, { quantity: p.intent.quantity, idempotency_key: p.intent.idempotency_key })
    if (!valid(g)) return
    save(null); quantity.value = ''
    notice.value = `${t('token.success')} · ${t('token.gross')}: ${receipt.gross} ${p.quoteSymbol}`
    await load()
  } catch (e) {
    if (!valid(g)) return
    const status = (e as { response?: { status?: number } }).response?.status
    if (status && [400, 403, 404, 409, 422].includes(status)) {
      try { save(null) } catch { recoveryBlocked.value = true }
    }
    error.value = pending.value ? t('token.purchasePending') : t('token.failure')
  } finally { if (valid(g)) busy.value = false }
}
async function requestQuote(row: TokenHolding) {
  if (busy.value || pending.value || !owner.value || recoveryBlocked.value) return
  try {
    const intent: TokenQuoteIntent = { side: 'sell', holding_id: row.holding.id, idempotency_key: crypto.randomUUID() }
    save({ project: row.holding.project_id, symbol: row.symbol, quoteSymbol: row.quote_symbol, intent, uncertain: false })
    await retrieveQuote()
  } catch (e) { error.value = e instanceof Error ? e.message : t('token.failure') }
}
async function retrieveQuote() {
  if (!pending.value || busy.value) return
  const p = pending.value; const g = generation; busy.value = true; error.value = ''; notice.value = ''
  try {
    const q = await quoteToken(p.symbol, p.project, p.intent)
    if (!valid(g)) return
    quote.value = q
    save({ ...p, quote: q })
    if (q.status === 'executed' && q.receipt_json) { notice.value = t('token.success'); save(null); quote.value = null; await load() }
  } catch { if (valid(g)) error.value = t('token.failure') }
  finally { if (valid(g)) busy.value = false }
}
async function confirm() {
  if (!quote.value || !pending.value || busy.value) return
  const g = generation; const q = quote.value
  try { save({ ...pending.value, uncertain: true }) } catch (e) { error.value = String(e); return }
  busy.value = true; error.value = ''
  try {
    const receipt = await confirmToken(q)
    if (!valid(g)) return
    notice.value = `${t('token.success')} · ${receipt.side === 'sell' ? t('token.net') : t('token.gross')}: ${receipt.side === 'sell' ? receipt.net : receipt.gross} ${pending.value?.quoteSymbol || ''}`
    save(null); quote.value = null; quantity.value = ''; await load()
  } catch (e) {
    if (!valid(g)) return
    const status = (e as { response?: { status?: number } }).response?.status
    if (status && [400, 403, 404, 409, 422].includes(status) && pending.value) save({ ...pending.value, uncertain: false })
    error.value = pending.value?.uncertain ? t('token.pending') : t('token.failure')
  } finally { if (valid(g)) busy.value = false }
}
function cancel() {
  if (busy.value || pending.value?.uncertain) return
  try { save(null); quote.value = null; error.value = '' } catch (e) { error.value = String(e) }
}
function page(cursor?: number) { if (busy.value || loading.value) return; before.value = cursor; void load() }
function openSpot() { if (props.pairSymbol) void router.push({ name: 'trade', params: { symbol: props.pairSymbol } }) }
function login() { void router.push({ name: 'login', query: { redirect: router.currentRoute.value.fullPath } }) }
watch(() => [owner.value, props.projectId], () => {
  generation++; before.value = undefined; next.value = null; holdings.value = []; quote.value = null; pending.value = null; error.value = ''; notice.value = ''; recoveryBlocked.value = false; loading.value = false; busy.value = false
  if (!owner.value) return
  try {
    const raw = sessionStorage.getItem(persistenceKey())
    if (raw) {
      const p = JSON.parse(raw) as Pending
      if (!Number.isSafeInteger(p.project) || p.project <= 0 || (props.projectId && p.project !== props.projectId) || !p.symbol || typeof p.intent?.idempotency_key !== 'string' || !p.intent.idempotency_key || typeof p.uncertain !== 'boolean' || (p.mode !== undefined && p.mode !== 'market')) throw new Error('Invalid pending trade')
      if (p.mode === 'market' && (p.intent.side !== 'buy' || !/[1-9]/.test(tokenDecimal(p.intent.quantity)))) throw new Error('Invalid pending purchase')
      if (p.quote) quote.value = parseTokenQuote(p.quote, p.project, p.intent)
      pending.value = p
      notice.value = t('token.recovered')
    }
  } catch { recoveryBlocked.value = true; error.value = t('token.pending') }
  void load()
}, { immediate: true })
const timer = setInterval(() => { clock.value = Date.now(); if (!busy.value && !pending.value) void load() }, 10_000)
onBeforeUnmount(() => { disposed = true; generation++; clearInterval(timer) })
</script>
<template>
  <section class="token-surface token-trading">
    <button class="token-control" v-if="stage === 'listed' && pairSymbol" type="button" @click="openSpot">{{ t('token.spot') }}</button>
    <button class="token-control" v-if="!owner" type="button" @click="login">{{ t('token.login') }}</button>
    <template v-if="owner">
      <p v-if="error" role="alert">{{ error }}</p><p v-if="notice" role="status">{{ notice }}</p>
      <form class="token-buy-form" v-if="stage === 'purchase' && !pending" @submit.prevent="buy()">
        <label>{{ t('token.quantity') }} · {{ symbol }}<input class="token-control" v-model="quantity" inputmode="decimal" :disabled="busy" required /></label>
        <button class="token-control" type="submit" :disabled="busy || recoveryBlocked || !quantity">{{ t('token.buy') }}</button>
      </form>
      <section v-if="pending?.mode === 'market'" class="token-confirm" aria-live="polite">
        <h3>{{ pending.symbol }} · {{ t('token.buy') }}</h3>
        <p>{{ t('token.quantity') }}: {{ pending.intent.side === 'buy' ? pending.intent.quantity : '' }} {{ pending.symbol }}</p>
        <p>{{ t(busy ? 'token.processing' : 'token.purchasePending') }}</p>
        <button class="token-control" type="button" :disabled="busy" @click="executePurchase">{{ t(busy ? 'token.processing' : 'token.retry') }}</button>
      </section>
      <section v-else-if="pending" class="token-confirm" aria-live="polite">
        <h3>{{ pending.symbol }} · {{ t(pending.intent.side === 'sell' ? 'token.sell' : 'token.quote') }}</h3>
        <template v-if="quote">
          <dl class="token-data"><div v-for="key in (pending.intent.side === 'sell' ? ['quantity', 'price', 'gross', 'fee', 'net'] as const : ['quantity', 'price', 'gross'] as const)" :key="key"><dt>{{ t(`token.${key}`) }}</dt><dd>{{ quote[key] }} {{ key === 'quantity' ? pending.symbol : pending.quoteSymbol }}</dd></div></dl>
          <p>{{ t('token.observed') }}: {{ date(quote.observed_at) }}</p><p>{{ t('token.expires') }}: {{ date(quote.expires_at) }}</p>
          <p v-if="pending.intent.side === 'sell'">{{ t('token.saleNote') }}</p>
          <p v-if="pending.uncertain">{{ t('token.pending') }}</p><p v-else-if="expired">{{ t('token.expired') }}</p>
          <button class="token-control" type="button" :disabled="busy || (expired && !pending.uncertain)" @click="confirm">{{ t(busy ? 'token.processing' : pending.uncertain ? 'token.retry' : 'token.confirm') }}</button>
        </template>
        <button class="token-control" v-else type="button" :disabled="busy" @click="retrieveQuote">{{ t(busy ? 'token.processing' : 'token.retryQuote') }}</button>
        <button class="token-control" type="button" :disabled="busy || pending.uncertain" @click="cancel">{{ t('token.cancel') }}</button>
      </section>
    </template>
    <div v-if="pairSymbol && ['purchase', 'listed'].includes(stage || '')" class="token-chart">
      <SecondsChartWorkspace :symbol="pairSymbol" initially-expanded />
    </div>
    <template v-if="owner">
      <h3 class="token-holdings-title">{{ t('token.holdings') }}</h3>
      <p v-if="loading && !holdings.length">{{ t('token.loading') }}</p><p v-else-if="!visible.length && !error">{{ t('token.empty') }}</p>
      <article v-for="row in visible" :key="row.holding.id" class="token-holding">
        <header><strong>{{ row.symbol }} <small>#{{ row.holding.id }}</small></strong><span :class="row.eligible ? 'is-eligible' : ''">{{ t(row.holding.status === 'sold' ? 'token.sold' : row.eligible ? 'token.eligible' : 'token.locked') }}</span></header>
        <div v-if="row.holding.status === 'held'" class="token-holding-summary">
          <div><span>{{ t('token.value') }}</span><strong>{{ row.market_value ?? '—' }} <small>{{ row.quote_symbol }}</small></strong></div>
          <div :class="row.profit === null ? '' : row.profit.startsWith('-') ? 'is-negative' : 'is-positive'"><span>{{ t('token.returnLabel') }}</span><strong>{{ tokenReturn(row.profit, row.holding.cost) ?? '—' }}</strong><small>{{ row.profit ?? '—' }} {{ row.quote_symbol }}</small></div>
        </div>
        <dl class="token-data"><div><dt>{{ t('token.quantity') }}</dt><dd>{{ row.holding.quantity }} {{ row.symbol }}</dd></div><div><dt>{{ t('token.cost') }}</dt><dd>{{ row.holding.cost }} {{ row.quote_symbol }}</dd></div></dl>
        <details class="token-holding-details"><summary>{{ t('token.details') }}</summary><dl class="token-data">
          <div><dt>{{ t('token.acquired') }}</dt><dd>{{ date(row.holding.acquired_at) }}</dd></div><div><dt>{{ t('token.ruleTime') }}</dt><dd>{{ row.unlock_at ? date(row.unlock_at) : t(`token.${row.holding.policy_json.lock.kind}`, { count: 'count' in row.holding.policy_json.lock ? row.holding.policy_json.lock.count : '' }) }}</dd></div>
        </dl></details>
        <button class="token-control" v-if="row.holding.status === 'held'" type="button" :disabled="busy || recoveryBlocked || !!pending || !row.eligible || !row.market_price" @click="requestQuote(row)">{{ t('token.sell') }}</button>
      </article>
      <nav><button class="token-control" v-if="before" type="button" :disabled="busy || loading" @click="page()">{{ t('token.firstPage') }}</button><button class="token-control" v-if="next" type="button" :disabled="busy || loading" @click="page(next)">{{ t('token.nextPage') }}</button></nav>
    </template>
  </section>
</template>
