<script setup lang="ts">
import { onBeforeUnmount, ref, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ChartCandlestick, ChevronDown, ChevronUp } from 'lucide-vue-next'
import MobileMarketChart from '@/components/MobileMarketChart.vue'
import { fetchKlines } from '@/api/market'
import { MARKET_KLINE_INTERVALS, type MarketKlineInterval } from '@/api/marketSocketProtocol'
import { publicMarketWebSocketUrl } from '@/config/app'
import { createSecondsChartSession, type SecondsChartSnapshot } from '@/core/secondsChartSession'

const props = defineProps<{ symbol: string }>()
const { t } = useI18n()
const expanded = ref(false)
const interval = ref<MarketKlineInterval>('1m')
const snapshot = shallowRef<SecondsChartSnapshot>({ points: [], loading: false, failed: false })
const session = createSecondsChartSession({
  getUrl: publicMarketWebSocketUrl,
  load: fetchKlines,
  onChange: (value) => { snapshot.value = value },
})
function reload(): void {
  if (expanded.value && props.symbol) void session.open(props.symbol, interval.value)
}
watch(() => [expanded.value, props.symbol, interval.value], () => {
  if (expanded.value && props.symbol) reload()
  else session.stop()
}, { flush: 'sync' })
onBeforeUnmount(() => session.stop())
</script>

<template>
  <section class="seconds-chart-workspace" :aria-label="t('marketDetail.chartWorkstation')">
    <div v-show="!expanded"><slot /></div>
    <button class="seconds-chart-workspace__toggle" type="button" :disabled="!symbol"
      :aria-expanded="expanded" aria-controls="seconds-chart-workspace-panel" @click="expanded = !expanded">
      <ChartCandlestick :size="18" aria-hidden="true" />
      <span>{{ t(expanded ? 'marketDetail.collapseChart' : 'marketDetail.expandChart') }}</span>
      <ChevronUp v-if="expanded" :size="18" aria-hidden="true" /><ChevronDown v-else :size="18" aria-hidden="true" />
    </button>
    <div v-if="expanded" id="seconds-chart-workspace-panel">
      <div class="seconds-chart-workspace__periods" role="group" :aria-label="t('marketDetail.chartInterval')">
        <button v-for="value in MARKET_KLINE_INTERVALS" :key="value" type="button"
          :aria-pressed="interval === value" @click="interval = value">{{ t(`marketDetail.intervals.${value}`) }}</button>
      </div>
      <div v-if="snapshot.failed" class="seconds-chart-workspace__error" role="status">
        <span>{{ t('marketDetail.chartLoadFailed') }}</span>
        <button type="button" @click="reload">{{ t('common.retry') }}</button>
      </div>
      <div class="seconds-chart-workspace__chart">
        <MobileMarketChart inspection :symbol="symbol" :interval="interval" :points="snapshot.points" :loading="snapshot.loading" />
      </div>
    </div>
  </section>
</template>

<style scoped>
.seconds-chart-workspace { min-width: 0; width: 100%; background: var(--surface); border-block: 1px solid var(--line); }
.seconds-chart-workspace button { color: var(--text); background: transparent; border: 0; min-height: 44px; }
.seconds-chart-workspace button:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
.seconds-chart-workspace button:disabled { opacity: .4; }
.seconds-chart-workspace__toggle { display: flex; align-items: center; justify-content: space-between; gap: 8px; width: 100%; padding: 0 12px; font-size: 12px; }
.seconds-chart-workspace__toggle span { flex: 1; text-align: left; }
.seconds-chart-workspace__periods { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); border-bottom: 1px solid var(--line); }
.seconds-chart-workspace__periods button { font-size: 12px; padding: 0 2px; }
.seconds-chart-workspace__periods button[aria-pressed='true'] { color: var(--text); background: var(--surface-2); box-shadow: inset 0 -2px var(--focus); font-weight: 700; }
.seconds-chart-workspace__chart { height: 480px; height: clamp(420px, 65dvh, 640px); min-width: 0; }
.seconds-chart-workspace__error { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 0 12px; color: var(--negative); font-size: 12px; }
</style>
