<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { useSessionStore } from '@/stores/session'
import SecondsChartWorkspace from '@/components/SecondsChartWorkspace.vue'
import { confirmToken, fetchTokenHoldings, quoteToken } from '@/api/tokenIssuance'
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
type Pending = { project: number; symbol: string; quoteSymbol: string; intent: TokenQuoteIntent; quote?: TokenQuote; uncertain: boolean }
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
async function requestQuote(row?: TokenHolding) {
  if (busy.value || pending.value || !owner.value || recoveryBlocked.value) return
  try {
    const intent: TokenQuoteIntent = row ? { side: 'sell', holding_id: row.holding.id, idempotency_key: crypto.randomUUID() }
      : { side: 'buy', quantity: tokenDecimal(quantity.value.trim()), idempotency_key: crypto.randomUUID() }
    if (!row && !/[1-9]/.test(quantity.value)) throw new Error(t('token.failure'))
    save({ project: row?.holding.project_id || props.projectId!, symbol: row?.symbol || props.symbol!, quoteSymbol: row?.quote_symbol || props.quoteSymbol || '', intent, uncertain: false })
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
      if (!Number.isSafeInteger(p.project) || !p.symbol || !p.intent?.idempotency_key || typeof p.uncertain !== 'boolean') throw new Error('Invalid pending trade')
      pending.value = p
      if (p.quote) quote.value = parseTokenQuote(p.quote, p.project, p.intent)
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
    <header><h2>{{ t('token.title') }}</h2><button type="button" :disabled="loading || !owner" @click="load">{{ t('token.refresh') }}</button></header>
    <div v-if="pairSymbol && ['purchase', 'listed'].includes(stage || '')" class="token-chart">
      <p>{{ t('token.market') }} · {{ t('token.marketHint') }}</p>
      <SecondsChartWorkspace :symbol="pairSymbol" initially-expanded />
    </div>
    <button v-if="stage === 'listed' && pairSymbol" type="button" @click="openSpot">{{ t('token.spot') }}</button>
    <button v-if="!owner" type="button" @click="login">{{ t('token.login') }}</button>
    <template v-else>
      <p v-if="error" role="alert">{{ error }}</p><p v-if="notice" role="status">{{ notice }}</p>
      <form v-if="stage === 'purchase' && !pending" @submit.prevent="requestQuote()">
        <label>{{ t('token.quantity') }} · {{ symbol }}<input v-model="quantity" inputmode="decimal" :disabled="busy" required /></label>
        <button type="submit" :disabled="busy || recoveryBlocked || !quantity">{{ t('token.quote') }}</button>
      </form>
      <section v-if="pending" class="token-confirm" aria-live="polite">
        <h3>{{ pending.symbol }} · {{ t(pending.intent.side === 'sell' ? 'token.sell' : 'token.quote') }}</h3>
        <template v-if="quote">
          <dl><div v-for="key in (pending.intent.side === 'sell' ? ['quantity', 'price', 'gross', 'fee', 'net'] as const : ['quantity', 'price', 'gross'] as const)" :key="key"><dt>{{ t(`token.${key}`) }}</dt><dd>{{ quote[key] }} {{ key === 'quantity' ? pending.symbol : pending.quoteSymbol }}</dd></div></dl>
          <p>{{ t('token.observed') }}: {{ date(quote.observed_at) }}</p><p>{{ t('token.expires') }}: {{ date(quote.expires_at) }}</p>
          <p v-if="pending.intent.side === 'sell'">{{ t('token.saleNote') }}</p>
          <p v-if="pending.uncertain">{{ t('token.pending') }}</p><p v-else-if="expired">{{ t('token.expired') }}</p>
          <button type="button" :disabled="busy || (expired && !pending.uncertain)" @click="confirm">{{ t(busy ? 'token.processing' : pending.uncertain ? 'token.retry' : 'token.confirm') }}</button>
        </template>
        <button v-else type="button" :disabled="busy" @click="retrieveQuote">{{ t(busy ? 'token.processing' : 'token.retryQuote') }}</button>
        <button type="button" :disabled="busy || pending.uncertain" @click="cancel">{{ t('token.cancel') }}</button>
      </section>
      <h3>{{ t('token.holdings') }}</h3>
      <p v-if="loading && !holdings.length">{{ t('token.loading') }}</p><p v-else-if="!visible.length && !error">{{ t('token.empty') }}</p>
      <article v-for="row in visible" :key="row.holding.id" class="token-holding">
        <header><strong>{{ row.symbol }} · #{{ row.holding.id }}</strong><span>{{ t(row.holding.status === 'sold' ? 'token.sold' : row.eligible ? 'token.eligible' : 'token.locked') }}</span></header>
        <dl><div><dt>{{ t('token.quantity') }}</dt><dd>{{ row.holding.quantity }} {{ row.symbol }}</dd></div><div><dt>{{ t('token.cost') }}</dt><dd>{{ row.holding.cost }} {{ row.quote_symbol }}</dd></div>
          <template v-if="row.holding.status === 'held'"><div><dt>{{ t('token.value') }}</dt><dd>{{ row.market_value ?? '—' }} {{ row.quote_symbol }}</dd></div><div><dt>{{ t('token.profit') }}</dt><dd>{{ row.profit ?? '—' }} / {{ tokenReturn(row.profit, row.holding.cost) ?? '—' }}</dd></div></template>
          <div><dt>{{ t('token.acquired') }}</dt><dd>{{ date(row.holding.acquired_at) }}</dd></div><div><dt>{{ t('token.ruleTime') }}</dt><dd>{{ row.unlock_at ? date(row.unlock_at) : t(`token.${row.holding.policy_json.lock.kind}`, { count: 'count' in row.holding.policy_json.lock ? row.holding.policy_json.lock.count : '' }) }}</dd></div>
        </dl>
        <button v-if="row.holding.status === 'held'" type="button" :disabled="busy || recoveryBlocked || !!pending || !row.eligible || !row.market_price" @click="requestQuote(row)">{{ t('token.sell') }}</button>
      </article>
      <nav><button v-if="before" type="button" :disabled="busy || loading" @click="page()">{{ t('token.firstPage') }}</button><button v-if="next" type="button" :disabled="busy || loading" @click="page(next)">{{ t('token.nextPage') }}</button></nav>
    </template>
  </section>
</template>
