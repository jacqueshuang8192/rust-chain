<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { fetchTimedDepositOrders } from '../api/timedDeposits'
import type { DepositOrder } from '../core/timedDeposit'
const props = defineProps<{ identity: string }>()
const emit = defineEmits<{ select: [id: number]; create: [] }>()
const { t, locale } = useI18n()
const items = ref<DepositOrder[]>([])
const total = ref(0)
const offset = ref(0)
const busy = ref(false)
const error = ref('')
const page = computed(() => Math.floor(offset.value / 20) + 1)
let controller: AbortController | null = null
let generation = 0
let destroyed = false
async function load(nextOffset = offset.value) {
  controller?.abort(); controller = new AbortController()
  const version = ++generation
  if (!props.identity) { items.value = []; total.value = 0; busy.value = false; return }
  busy.value = true; error.value = ''
  try {
    const result = await fetchTimedDepositOrders('', '', nextOffset, controller.signal)
    if (destroyed || version !== generation || !props.identity) return
    items.value = result.items; total.value = result.total; offset.value = nextOffset
  } catch {
    if (!destroyed && version === generation && props.identity) error.value = t('timedDeposit.historyFailed')
  } finally { if (!destroyed && version === generation) busy.value = false }
}
function move(direction: -1 | 1) {
  if (busy.value) return
  const next = offset.value + direction * 20
  if (next >= 0 && next < total.value) void load(next)
}
function date(value: number) { return new Intl.DateTimeFormat(locale.value === 'en' ? 'en-US' : 'zh-CN', { dateStyle: 'medium', timeStyle: 'short' }).format(value) }
watch(() => props.identity, () => { items.value = []; total.value = 0; offset.value = 0; error.value = ''; void load(0) }, { immediate: true })
onUnmounted(() => { destroyed = true; generation += 1; controller?.abort() })
</script>
<template>
  <section class="deposit-history" :aria-label="t('timedDeposit.history')" :aria-busy="busy">
    <div class="deposit-history__toolbar"><p>{{ t('timedDeposit.historyHint') }}</p><button type="button" :disabled="busy" @click="load()">{{ t('common.refresh') }}</button></div>
    <p v-if="error" role="alert" class="deposit-history__error">{{ error }}</p>
    <p v-if="busy" role="status">{{ t('timedDeposit.loading') }}</p>
    <div v-if="!busy && !error && !items.length" class="deposit-history__empty"><h2>{{ t('timedDeposit.empty') }}</h2><button type="button" @click="emit('create')">{{ t('timedDeposit.newDeposit') }}</button></div>
    <ol v-if="items.length" class="deposit-history__list">
      <li v-for="item in items" :key="item.id">
        <button type="button" :aria-label="`${t('timedDeposit.details')} ${item.order_no}`" @click="emit('select', item.id)">
          <span class="deposit-history__top"><strong>{{ item.amount }} <small>{{ item.asset_symbol }}</small></strong><span :class="`deposit-history__state is-${item.status}`">{{ t(`timedDeposit.${item.status}`) }}</span></span>
          <span class="deposit-history__meta"><span>{{ item.network }}</span><time :datetime="new Date(item.created_at).toISOString()">{{ date(item.created_at) }}</time></span>
          <span class="deposit-history__number">{{ item.order_no }} <span aria-hidden="true">→</span></span>
        </button>
      </li>
    </ol>
    <nav v-if="total" class="deposit-history__pagination" :aria-label="t('timedDeposit.history')"><p>{{ t('timedDeposit.page', { page, total }) }}</p><div><button type="button" :disabled="busy || offset === 0" @click="move(-1)">{{ t('timedDeposit.previous') }}</button><button type="button" :disabled="busy || offset + 20 >= total" @click="move(1)">{{ t('timedDeposit.next') }}</button></div></nav>
  </section>
</template>
<style>
.deposit-history { display: grid; gap: 24px; min-width: 0; color: var(--text); }
.deposit-history p { margin: 0; font-size: 14px; line-height: 1.6; }
.deposit-history__toolbar { display: grid; gap: 12px; grid-template-columns: minmax(0, 1fr) auto; align-items: center; color: var(--muted-strong); }
.deposit-history button { min-height: 44px; padding: 10px 14px; border: 0; border-radius: 6px; background: var(--surface-2); color: inherit; font: inherit; font-size: 14px; cursor: pointer; }
.deposit-history button:focus-visible { outline: 2px solid var(--focus); outline-offset: 3px; }
.deposit-history button:disabled { opacity: .5; cursor: not-allowed; }
.deposit-history__list { margin: 0; padding: 0; list-style: none; }
.deposit-history__list li { border-bottom: 1px solid var(--hairline); }
.deposit-history__list button { width: 100%; text-align: left; padding: 22px 0; background: transparent; display: grid; gap: 12px; border-radius: 0; }
.deposit-history__top { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
.deposit-history__top strong { font-size: 22px; line-height: 1.3; font-variant-numeric: tabular-nums; overflow-wrap: anywhere; min-width: 0; }
.deposit-history__top small { font-size: 14px; font-weight: 500; }
.deposit-history__state { font-size: 12px; line-height: 1.6; padding: 4px 7px; background: var(--surface-2); flex-shrink: 0; border-radius: 4px; max-width: 42%; }
.deposit-history__state.is-credited { color: var(--positive); }
.deposit-history__meta, .deposit-history__number { display: flex; justify-content: space-between; gap: 12px; color: var(--muted-strong); font-size: 12px; overflow-wrap: anywhere; }
.deposit-history__number { font-size: 11px; }
.deposit-history__empty { padding-block: 50px; text-align: center; }
.deposit-history__empty h2 { font-size: 20px; margin: 0 0 22px; }
.deposit-history__pagination { display: grid; gap: 16px; text-align: center; }
.deposit-history__pagination > div { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.deposit-history__error { color: var(--negative); }
</style>
