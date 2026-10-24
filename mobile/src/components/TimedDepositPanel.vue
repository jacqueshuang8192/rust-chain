<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { toDataURL } from 'qrcode'
import { cancelTimedDepositOrder, createTimedDepositOrder, fetchTimedDepositOptions, fetchTimedDepositOrder, fetchTimedDepositOrders } from '../api/timedDeposits'
import { validateDepositDraft, type DepositOption, type DepositOrder } from '../core/timedDeposit'
import { useModalDialog } from '../core/modalDialog'
const props = withDefaults(defineProps<{ identity: string; asset?: string; network?: string; orderId?: string }>(), { orderId: '', asset: '', network: '' })
const emit = defineEmits<{ history: [] }>()
const { t, locale } = useI18n()
const options = ref<DepositOption[]>([])
const selection = ref('')
const amount = ref('')
const order = ref<DepositOrder | null>(null)
const history = ref<DepositOrder[]>([])
const error = ref('')
const busy = ref(false)
const loading = ref(false)
const qr = ref('')
const now = ref(Date.now())
const clockOffset = ref(0)
const copied = ref(false)
const confirmingCancel = ref(false)
const cancelError = ref('')
const dialog = ref<HTMLElement | null>(null)
const { trapFocus } = useModalDialog(confirmingCancel, dialog)
function closeCancel() { if (!busy.value) confirmingCancel.value = false }
function askCancel() { if (payable.value && !busy.value) { cancelError.value = ''; confirmingCancel.value = true } }
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
async function readHistory(version: number, signal: AbortSignal, restore: boolean) {
  const page = await fetchTimedDepositOrders(pair.value.asset, pair.value.network, 0, signal)
  if (!current(version)) return
  history.value = page.items
  if (restore) { const pending = page.items.find(item => item.status === 'pending' || item.status === 'review'); if (pending) setOrder(pending) }
}
async function load() {
  const { version, signal } = begin()
  confirmingCancel.value = false; cancelError.value = ''; order.value = null; history.value = []; options.value = []; selection.value = ''; amount.value = ''; error.value = ''; busy.value = false
  if (!props.identity) { loading.value = false; return }
  loading.value = true
  try {
    if (props.orderId) {
      const id = Number(props.orderId)
      if (!/^\d+$/.test(props.orderId) || !Number.isSafeInteger(id) || id <= 0) throw new Error('Invalid order ID')
      const restored = await fetchTimedDepositOrder(id, signal)
      if (current(version)) setOrder(restored)
      return
    }
    const available = await fetchTimedDepositOptions(signal)
    if (!current(version)) return
    options.value = available.filter(item => (!props.asset || item.asset_symbol === props.asset.toUpperCase()) && (!props.network || item.network === props.network))
    selection.value = options.value[0] ? optionKey(options.value[0]) : ''
    await readHistory(version, signal, true)
  } catch (cause) { if (current(version)) error.value = message(cause) }
  finally { if (current(version)) loading.value = false }
}
async function changeSelection() {
  const { version, signal } = begin(); order.value = null; amount.value = ''; error.value = ''; history.value = []; loading.value = true
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
async function cancelOrder() {
  if (!confirmingCancel.value || !payable.value || busy.value || !order.value || !props.identity) return
  const id = order.value.id; const { version, signal } = begin()
  busy.value = true; cancelError.value = ''; error.value = ''
  try {
    const cancelled = await cancelTimedDepositOrder(id, signal)
    if (!current(version)) return
    setOrder(cancelled); confirmingCancel.value = false
  } catch {
    if (current(version)) {
      cancelError.value = t('timedDeposit.cancelFailed')
      // A lost response may still mean success. Re-read without retrying a mutation.
      try {
        const latest = await fetchTimedDepositOrder(id, signal)
        if (current(version)) { setOrder(latest); if (latest.status === 'cancelled') confirmingCancel.value = false }
      } catch { /* Preserve the dialog and explain the uncertain result. */ }
    }
  } finally { if (current(version)) busy.value = false }
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
async function copy() {
  if (!payable.value || !order.value) return
  try { await navigator.clipboard.writeText(order.value.address); copied.value = true } catch { error.value = t('timedDeposit.copyFailed') }
}
function date(value: number) { return new Intl.DateTimeFormat(locale.value === 'en' ? 'en-US' : 'zh-CN', { dateStyle: 'medium', timeStyle: 'short' }).format(value) }
const details = computed(() => {
  const value = order.value
  if (!value) return []
  const fields = [['order', value.order_no], ['fee', `${value.fee_amount} ${value.asset_symbol}`], ['createdAt', date(value.created_at)], ['expiresAt', date(value.expires_at)]]
  if (value.paid_amount !== null) fields.push(['received', `${value.paid_amount} ${value.asset_symbol}`])
  if (!payable.value) fields.push(['address', value.address])
  if (value.cancelled_at !== null) fields.push(['cancelledAt', date(value.cancelled_at)])
  if (value.tx_hash) fields.push(['transaction', value.tx_hash])
  return fields
})
function status(value: DepositOrder) { return t(`timedDeposit.${value.status === 'pending' && value.expires_at <= now.value + clockOffset.value ? 'expired' : value.status}`) }
watch(() => [props.identity, props.asset, props.network, props.orderId], () => { void load() }, { immediate: true })
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
    <div class="deposit-toolbar"><span>{{ t('timedDeposit.window') }}</span><button type="button" class="deposit-link" :disabled="busy" @click="emit('history')">{{ t('timedDeposit.history') }} →</button></div>
    <p v-if="error" role="alert" class="deposit-error">{{ error }} <button type="button" :disabled="busy || loading" @click="order ? refresh() : load()">{{ t('timedDeposit.refresh') }}</button></p>
    <p v-if="loading" role="status">{{ t('timedDeposit.loading') }}</p>
    <form v-if="!props.orderId && !payable && order?.status !== 'review'" class="deposit-form" @submit.prevent="submit">
      <div class="deposit-form-heading"><h2>{{ t('timedDeposit.amount') }}</h2><p>{{ t('timedDeposit.draftHint') }}</p></div>
      <label>{{ t('timedDeposit.assetNetwork') }}
        <select v-model="selection" :disabled="loading || busy || options.length === 0" @change="changeSelection"><option v-for="item in options" :key="optionKey(item)" :value="optionKey(item)">{{ item.asset_symbol }} · {{ item.display_name }}</option></select>
      </label>
      <p v-if="!loading && !options.length">{{ t('timedDeposit.unavailable') }}</p>
      <label>{{ t('timedDeposit.amount') }}<div class="deposit-amount-field"><input v-model="amount" inputmode="decimal" type="text" maxlength="64" :disabled="busy || !option" :placeholder="t('timedDeposit.amountPlaceholder')" /><span>{{ option?.asset_symbol || '—' }}</span></div></label>
      <div v-if="option" class="deposit-terms"><p>{{ t('timedDeposit.minimum') }}<strong>{{ option.min_deposit_amount }} {{ option.asset_symbol }}</strong></p><p>{{ t('timedDeposit.fee') }}<strong>{{ option.deposit_fee }} {{ option.asset_symbol }}</strong></p></div>
      <p v-if="amount && !validDraft" role="status" class="deposit-error">{{ t('timedDeposit.invalid') }}</p>
      <button class="deposit-primary" type="submit" :disabled="busy || loading || !validDraft">{{ busy ? t('timedDeposit.creating') : t('timedDeposit.create') }}</button>
      <p class="deposit-notice">{{ t('timedDeposit.instructions') }}</p>
    </form>
    <section v-if="order" class="deposit-order" :aria-label="t('timedDeposit.order')">
      <header class="deposit-status"><span :class="`deposit-state is-${payable ? 'pending' : order.status}`">{{ status(order) }}</span><span v-if="payable">{{ t('timedDeposit.remaining') }} <b role="timer">{{ countdown }}</b></span></header>
      <div class="deposit-payment-heading"><p>{{ payable ? t('timedDeposit.payExactly') : t('timedDeposit.amount') }}</p><h2>{{ order.amount }} <span>{{ order.asset_symbol }}</span></h2><p>{{ order.asset_symbol }} · {{ order.network }}</p></div>
      <div v-if="payable" class="deposit-payment">
        <div class="deposit-qr"><img v-if="qr" :src="qr" width="220" height="220" :alt="t('timedDeposit.qr')" /><span v-else>{{ t('timedDeposit.loading') }}</span></div>
        <div class="deposit-destination"><h3>{{ t('timedDeposit.address') }}</h3><p>{{ order.address }}</p><button class="deposit-primary" type="button" @click="copy">{{ copied ? t('timedDeposit.copied') : t('timedDeposit.copy') }}</button><p class="deposit-notice">{{ t('timedDeposit.networkOnly', { network: order.network, asset: order.asset_symbol }) }}</p></div>
      </div>
      <p v-if="!payable && order.status !== 'credited'" class="deposit-notice">{{ t('timedDeposit.stopped') }}</p>
      <dl class="deposit-details"><div v-for="field in details" :key="field[0]"><dt>{{ t(`timedDeposit.${field[0]}`) }}</dt><dd>{{ field[1] }}</dd></div></dl>
      <button v-if="payable" type="button" :disabled="busy" @click="askCancel">{{ t('timedDeposit.cancel') }}</button>
      <footer class="deposit-footer"><p>{{ t('timedDeposit.confirmations', { count: order.required_confirmations }) }}</p><button type="button" :disabled="busy || loading" @click="refresh">{{ t('timedDeposit.refresh') }}</button></footer>
    </section>
    <Teleport to="body">
      <div v-if="confirmingCancel" class="pencil-page deposit-cancel-overlay" @click.self="closeCancel">
        <section ref="dialog" class="deposit-cancel-dialog" role="dialog" aria-modal="true" aria-labelledby="deposit-cancel-title" tabindex="-1" :aria-busy="busy" @keydown="trapFocus($event, closeCancel)">
          <h2 id="deposit-cancel-title">{{ t('timedDeposit.cancelTitle') }}</h2>
          <p>{{ t('timedDeposit.cancelHint') }}</p>
          <p v-if="cancelError" role="alert">{{ cancelError }}</p>
          <button data-dialog-initial type="button" :disabled="busy" @click="closeCancel">{{ t('timedDeposit.keepOrder') }}</button>
          <button type="button" :disabled="busy || !payable" @click="cancelOrder">{{ busy ? t('timedDeposit.cancelling') : t('timedDeposit.confirmCancel') }}</button>
        </section>
      </div>
    </Teleport>
  </section>
</template>
<style>
.timed-deposit { --deposit-ink: var(--text, currentColor); --deposit-muted: var(--muted-strong, #667085); --deposit-surface: var(--surface-2, #f3f5f7); display: grid; gap: 24px; color: var(--deposit-ink); min-width: 0; }
.timed-deposit p { margin: 0; line-height: 1.6; font-size: 14px; }
.timed-deposit h2, .timed-deposit h3 { margin: 0; }
.deposit-toolbar, .deposit-status, .deposit-footer { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
.deposit-toolbar > span, .deposit-notice, .deposit-form-heading p { color: var(--deposit-muted); font-size: 13px; }
.deposit-form, .deposit-order { display: grid; gap: 22px; min-width: 0; }
.deposit-form-heading h2 { font-size: 26px; line-height: 1.2; margin-bottom: 10px; }
.timed-deposit label { display: grid; gap: 10px; font-size: 14px; }
.timed-deposit input, .timed-deposit select { width: 100%; min-width: 0; min-height: 52px; padding: 12px 14px; border: 0; border-radius: 6px; background: var(--deposit-surface); color: inherit; font: inherit; }
.deposit-amount-field { display: flex; align-items: center; background: var(--deposit-surface); border-radius: 6px; padding-right: 16px; gap: 8px; }
.deposit-amount-field input { font-size: 24px; font-variant-numeric: tabular-nums; }
.deposit-amount-field span { font-weight: 600; }
.deposit-amount-field:focus-within, .timed-deposit select:focus-visible, .timed-deposit button:focus-visible { outline: 2px solid var(--focus, #10b981); outline-offset: 3px; }
.deposit-amount-field input:focus { outline: none; }
.timed-deposit button { min-height: 44px; padding: 10px 16px; border-radius: 6px; background: var(--deposit-surface); border: 0; color: inherit; cursor: pointer; font: inherit; font-size: 14px; transition: opacity 120ms, transform 120ms; }
.timed-deposit button:active:not(:disabled) { transform: translateY(1px); }
.timed-deposit button.deposit-primary { min-height: 50px; background: #087b52; color: white; font-weight: 600; }
.timed-deposit button.deposit-link { background: transparent; padding-inline: 0; }
.timed-deposit button:disabled { opacity: .5; cursor: not-allowed; }
.deposit-terms { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
.deposit-terms p { display: grid; gap: 6px; color: var(--deposit-muted); font-size: 13px; }
.deposit-terms strong { color: var(--deposit-ink); font-size: 14px; font-weight: 500; overflow-wrap: anywhere; }
.deposit-status { font-size: 13px; }
.deposit-status b { font-size: 24px; margin-left: 6px; font-variant-numeric: tabular-nums; }
.deposit-state { background: var(--deposit-surface); padding: 7px 10px; border-radius: 4px; }
.deposit-payment-heading h2 { font-size: 32px; line-height: 1.2; overflow-wrap: anywhere; margin: 8px 0; font-variant-numeric: tabular-nums; }
.deposit-payment-heading h2 span { font-size: 18px; font-weight: 500; white-space: nowrap; }
.deposit-payment { display: grid; grid-template-columns: minmax(0, 1fr); gap: 22px; padding: 20px 16px; background: var(--deposit-surface); }
.deposit-qr { justify-self: center; display: grid; place-items: center; background: white; color: #14231d; padding: 10px; width: 240px; max-width: 100%; aspect-ratio: 1; box-sizing: border-box; }
.deposit-qr img { max-width: 100%; height: auto; }
.deposit-destination { display: grid; gap: 14px; min-width: 0; }
.deposit-destination h3 { font-size: 16px; }
.deposit-destination > p { overflow-wrap: anywhere; }
.deposit-details { margin: 0; display: grid; gap: 14px; }
.deposit-details > div { display: grid; grid-template-columns: minmax(80px, 1fr) minmax(0, 2fr); gap: 16px; font-size: 13px; line-height: 1.5; }
.timed-deposit dt { color: var(--deposit-muted); }
.timed-deposit dd { margin: 0; overflow-wrap: anywhere; text-align: right; }
.deposit-error { color: var(--negative, #c43b3b); }
.deposit-footer button { width: 100%; }
@media (prefers-reduced-motion: reduce) { .timed-deposit button { transition: none; } .timed-deposit button:active:not(:disabled) { transform: none; } }
</style>

<style>
.pencil-page.deposit-cancel-overlay { position: fixed; inset: 0; z-index: calc(var(--layer-overlay, 80) + 1); background: var(--overlay, #0008); display: grid; align-items: end; justify-items: center; }
.deposit-cancel-dialog { width: min(100%, 448px); max-height: 90dvh; overflow: auto; padding: 26px 24px calc(24px + env(safe-area-inset-bottom)); background: var(--surface-elevated); color: var(--text); display: grid; gap: 18px; border-radius: 20px 20px 0 0; }
.deposit-cancel-dialog h2 { margin: 0; font-size: 22px; }
.deposit-cancel-dialog p { margin: 0; font-size: 15px; line-height: 1.7; overflow-wrap: anywhere; }
.deposit-cancel-dialog [role='alert'] { color: var(--negative); }
.deposit-cancel-dialog button { min-height: 48px; border: 1px solid var(--line); border-radius: 6px; background: var(--surface); color: var(--text); font: inherit; }
.deposit-cancel-dialog button:last-child { background: #087b52; color: white; border: 0; }
.deposit-cancel-dialog button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
.deposit-cancel-dialog button:disabled { opacity: .5; }
</style>
