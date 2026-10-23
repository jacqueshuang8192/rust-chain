<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { toDataURL } from 'qrcode'
import { createTimedDepositOrder, fetchTimedDepositOptions, fetchTimedDepositOrder, fetchTimedDepositOrders } from '../api/timedDeposits'
import { validateDepositDraft, type DepositOption, type DepositOrder } from '../core/timedDeposit'
const props = withDefaults(defineProps<{ identity: string; asset?: string; network?: string }>(), { asset: '', network: '' })
const { t } = useI18n()
const options = ref<DepositOption[]>([])
const selection = ref('')
const amount = ref('')
const order = ref<DepositOrder | null>(null)
const history = ref<DepositOrder[]>([])
const total = ref(0)
const offset = ref(0)
const error = ref('')
const busy = ref(false)
const loading = ref(false)
const qr = ref('')
const now = ref(Date.now())
const clockOffset = ref(0)
const copied = ref(false)
let generation = 0
let controller: AbortController | null = null
let polling = false
let destroyed = false
let qrGeneration = 0
const optionKey = (value: DepositOption) => `${value.asset_symbol}:${value.network}`
const option = computed(() => options.value.find(value => optionKey(value) === selection.value))
const pair = computed(() => ({ asset: option.value?.asset_symbol || props.asset, network: option.value?.network || props.network }))
const remaining = computed(() => Math.max(0, Math.ceil(((order.value?.expires_at || 0) - now.value - clockOffset.value) / 1000)))
const payable = computed(() => order.value?.status === 'pending' && remaining.value > 0)
const countdown = computed(() => `${Math.floor(remaining.value / 60).toString().padStart(2, '0')}:${(remaining.value % 60).toString().padStart(2, '0')}`)
const validDraft = computed(() => { try { return option.value ? validateDepositDraft(amount.value, option.value) : '' } catch { return '' } })
function begin() { controller?.abort(); controller = new AbortController(); generation += 1; return { version: generation, signal: controller.signal } }
function current(version: number) { return !destroyed && version === generation && Boolean(props.identity) }
function message(cause: unknown): string {
  const raw = (cause as { response?: { data?: { message?: string; error?: { message?: string } } } })?.response?.data
  const detail = raw?.message || raw?.error?.message || ''
  if (detail.includes('all addresses')) return t('timedDeposit.full')
  if (detail.includes('unavailable') || detail.includes('not configured')) return t('timedDeposit.unavailable')
  return t('timedDeposit.failed')
}
function setOrder(value: DepositOrder, syncClock = true) { order.value = value; if (syncClock) clockOffset.value = value.server_time - Date.now(); now.value = Date.now() }
function selectHistory(value: DepositOrder) { begin(); setOrder(value, false); void refresh() }
async function readHistory(version: number, signal: AbortSignal, restore: boolean) {
  const page = await fetchTimedDepositOrders(pair.value.asset, pair.value.network, offset.value, signal)
  if (!current(version)) return
  history.value = page.items; total.value = page.total
  if (restore) { const pending = page.items.find(item => item.status === 'pending' || item.status === 'review'); if (pending) setOrder(pending) }
}
async function load() {
  const { version, signal } = begin()
  order.value = null; history.value = []; options.value = []; selection.value = ''; amount.value = ''; error.value = ''; offset.value = 0; busy.value = false
  if (!props.identity) { loading.value = false; return }
  loading.value = true
  try {
    const available = await fetchTimedDepositOptions(signal)
    if (!current(version)) return
    options.value = available.filter(item => (!props.asset || item.asset_symbol === props.asset.toUpperCase()) && (!props.network || item.network === props.network))
    selection.value = options.value[0] ? optionKey(options.value[0]) : ''
    await readHistory(version, signal, true)
  } catch (cause) { if (current(version)) error.value = message(cause) }
  finally { if (current(version)) loading.value = false }
}
async function changeSelection() {
  const { version, signal } = begin(); order.value = null; amount.value = ''; error.value = ''; offset.value = 0; history.value = []; loading.value = true
  try { await readHistory(version, signal, true) } catch (cause) { if (current(version)) error.value = message(cause) }
  finally { if (current(version)) loading.value = false }
}
async function submit() {
  if (busy.value || !option.value || !validDraft.value || !props.identity || payable.value) return
  const chosen = option.value; const exact = validDraft.value; const { version, signal } = begin()
  busy.value = true; error.value = ''
  const storageKey = `timed-deposit:${props.identity}:${optionKey(chosen)}:${exact}`
  try {
    let key = sessionStorage.getItem(storageKey)
    if (!key) { key = crypto.randomUUID(); sessionStorage.setItem(storageKey, key) }
    const created = await createTimedDepositOrder(chosen, exact, key, signal)
    if (!current(version)) return
    setOrder(created); sessionStorage.removeItem(storageKey)
    await readHistory(version, signal, false)
  } catch (cause) { if (current(version)) error.value = message(cause) }
  finally { if (current(version)) busy.value = false }
}
async function refresh() {
  if (loading.value || busy.value || polling || !props.identity || destroyed) return
  const version = generation; const signal = controller?.signal
  if (!signal) return
  polling = true
  try {
    if (order.value) { const value = await fetchTimedDepositOrder(order.value.id, signal); if (current(version)) { setOrder(value); history.value = history.value.map(item => item.id === value.id ? value : item) } }
    if (current(version)) error.value = ''
  } catch (cause) { if (current(version)) error.value = message(cause) }
  finally { polling = false }
}
async function page(delta: number) {
  if (busy.value || loading.value) return
  offset.value = Math.max(0, offset.value + delta)
  const { version, signal } = begin(); loading.value = true
  try { await readHistory(version, signal, false) } catch (cause) { if (current(version)) error.value = message(cause) }
  finally { if (current(version)) loading.value = false }
}
async function copy() {
  if (!payable.value || !order.value) return
  try { await navigator.clipboard.writeText(order.value.address); copied.value = true } catch { error.value = t('timedDeposit.copyFailed') }
}
function status(value: DepositOrder) { return t(`timedDeposit.${value.status === 'pending' && value.expires_at <= now.value + clockOffset.value ? 'expired' : value.status}`) }
watch(() => [props.identity, props.asset, props.network], () => { void load() }, { immediate: true })
watch(() => [order.value?.address, payable.value], async () => {
  const version = ++qrGeneration; qr.value = ''; copied.value = false
  if (!order.value || !payable.value) return
  try { const image = await toDataURL(order.value.address, { width: 220, margin: 1 }); if (!destroyed && version === qrGeneration) qr.value = image }
  catch { if (!destroyed && version === qrGeneration) error.value = t('timedDeposit.qrFailed') }
})
const clock = setInterval(() => { now.value = Date.now() }, 1000)
const poll = setInterval(() => { void refresh() }, 5000)
onUnmounted(() => { destroyed = true; generation += 1; qrGeneration += 1; controller?.abort(); clearInterval(clock); clearInterval(poll) })
</script>
<template>
  <section class="timed-deposit" :aria-label="t('timedDeposit.title')">
    <button type="button" :disabled="busy || loading" @click="load">{{ t('timedDeposit.refresh') }}</button>
    <p class="timed-deposit__notice">{{ t('timedDeposit.instructions') }}</p>
    <p v-if="error" role="alert" class="timed-deposit__error">{{ error }}</p>
    <p v-if="loading" role="status">{{ t('timedDeposit.loading') }}</p>
    <form v-if="!payable && order?.status !== 'review'" class="timed-deposit__form" @submit.prevent="submit">
      <label>{{ t('timedDeposit.assetNetwork') }}
        <select v-model="selection" :disabled="loading || busy || options.length === 0" @change="changeSelection">
          <option v-for="item in options" :key="optionKey(item)" :value="optionKey(item)">{{ item.asset_symbol }} · {{ item.display_name }}</option>
        </select>
      </label>
      <p v-if="!loading && !options.length">{{ t('timedDeposit.unavailable') }}</p>
      <label>{{ t('timedDeposit.amount') }} <span v-if="option">({{ option.asset_symbol }})</span>
        <input v-model="amount" inputmode="decimal" type="text" maxlength="64" :disabled="busy || !option" :placeholder="t('timedDeposit.amountPlaceholder')" />
      </label>
      <p v-if="option">{{ t('timedDeposit.minimum') }} {{ option.min_deposit_amount }} {{ option.asset_symbol }} · {{ t('timedDeposit.fee') }} {{ option.deposit_fee }} {{ option.asset_symbol }}</p>
      <p v-if="amount && !validDraft" role="status">{{ t('timedDeposit.invalid') }}</p>
      <button type="submit" :disabled="busy || loading || !validDraft">{{ busy ? t('timedDeposit.creating') : t('timedDeposit.create') }}</button>
    </form>
    <section v-if="order" class="timed-deposit__order" :aria-label="t('timedDeposit.order')">
      <strong>{{ order.order_no }}</strong>
      <p>{{ status(order) }} <b v-if="payable" role="timer">{{ countdown }}</b></p>
      <dl>
        <div><dt>{{ t('timedDeposit.payExactly') }}</dt><dd>{{ order.amount }} {{ order.asset_symbol }}</dd></div>
        <div><dt>{{ t('timedDeposit.assetNetwork') }}</dt><dd>{{ order.asset_symbol }} · {{ order.network }}</dd></div>
        <div><dt>{{ t('timedDeposit.fee') }}</dt><dd>{{ order.fee_amount }} {{ order.asset_symbol }}</dd></div>
        <div v-if="order.paid_amount !== null"><dt>{{ t('timedDeposit.received') }}</dt><dd>{{ order.paid_amount }} {{ order.asset_symbol }}</dd></div>
        <div><dt>{{ t('timedDeposit.address') }}</dt><dd>{{ order.address }}</dd></div>
        <div v-if="order.tx_hash"><dt>{{ t('timedDeposit.transaction') }}</dt><dd>{{ order.tx_hash }}</dd></div>
      </dl>
      <img v-if="payable && qr" :src="qr" width="220" height="220" :alt="t('timedDeposit.qr')" />
      <button v-if="payable" type="button" @click="copy">{{ copied ? t('timedDeposit.copied') : t('timedDeposit.copy') }}</button>
      <p v-if="!payable && order.status !== 'credited'">{{ t('timedDeposit.stopped') }}</p>
      <p>{{ t('timedDeposit.confirmations', { count: order.required_confirmations }) }}</p>
      <button type="button" :disabled="busy || loading" @click="refresh">{{ t('timedDeposit.refresh') }}</button>
    </section>
    <section class="timed-deposit__history" :aria-label="t('timedDeposit.history')">
      <h3>{{ t('timedDeposit.history') }}</h3>
      <p v-if="!history.length && !loading">{{ t('timedDeposit.empty') }}</p>
      <button v-for="item in history" :key="item.id" type="button" :disabled="busy || loading" @click="selectHistory(item)">
        <span>{{ item.order_no }}</span><strong>{{ item.amount }} {{ item.asset_symbol }} · {{ item.network }}</strong><span>{{ status(item) }}</span>
      </button>
      <div class="timed-deposit__pages"><button type="button" :disabled="offset === 0 || loading" @click="page(-20)">{{ t('timedDeposit.previous') }}</button><button type="button" :disabled="offset + history.length >= total || loading" @click="page(20)">{{ t('timedDeposit.next') }}</button></div>
    </section>
  </section>
</template>
<style scoped>
.timed-deposit { display: grid; gap: 18px; color: inherit; min-width: 0; }
.timed-deposit p { margin: 0; line-height: 1.6; font-size: 13px; }
.timed-deposit__notice { opacity: .8; }
.timed-deposit__error { color: var(--negative, #d34848); }
.timed-deposit__form, .timed-deposit__order { display: grid; gap: 12px; min-width: 0; }
.timed-deposit label { display: grid; gap: 8px; font-size: 14px; }
.timed-deposit input, .timed-deposit select { width: 100%; min-width: 0; min-height: 46px; padding: 10px 12px; border: 1px solid var(--line, #9ca3af); border-radius: 8px; background: var(--surface, transparent); color: inherit; font: inherit; }
.timed-deposit input:focus-visible, .timed-deposit select:focus-visible, .timed-deposit button:focus-visible { outline: 2px solid var(--focus, #10b981); outline-offset: 3px; }
.timed-deposit button { min-height: 44px; padding: 10px 14px; border-radius: 8px; background: var(--surface-2, rgba(128,128,128,.12)); border: 1px solid var(--line, #9ca3af); color: inherit; cursor: pointer; }
.timed-deposit button[type=submit] { background: #087b52; color: white; border-color: #087b52; }
.timed-deposit button:disabled { opacity: .5; cursor: not-allowed; }
.timed-deposit dl { margin: 0; display: grid; gap: 10px; }
.timed-deposit dl > div { display: grid; grid-template-columns: minmax(80px, 1fr) minmax(0, 2fr); gap: 12px; font-size: 13px; }
.timed-deposit dt { opacity: .7; }
.timed-deposit dd { margin: 0; overflow-wrap: anywhere; }
.timed-deposit__order > strong { overflow-wrap: anywhere; font-size: 13px; }
.timed-deposit__order img { justify-self: center; max-width: 100%; }
.timed-deposit__history { display: grid; gap: 10px; }
.timed-deposit__history > button { display: grid; gap: 5px; text-align: left; overflow-wrap: anywhere; }
.timed-deposit__history > button span { font-size: 12px; }
.timed-deposit__pages { display: flex; justify-content: space-between; gap: 12px; }
</style>
