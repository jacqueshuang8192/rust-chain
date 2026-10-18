<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ArrowRightToLine, ZoomIn, ZoomOut } from 'lucide-vue-next'
import { resolveMarketChartPriceFormat, type NormalizedMarketChartPoint } from '@/core/marketChart'
import type { MarketMovingAverages } from '@/core/marketIndicators'

const props = defineProps<{
  points: NormalizedMarketChartPoint[]
  averages: MarketMovingAverages
  time: number | null
}>()
defineEmits<{ zoom: [factor: number]; latest: []; averages: [visible: boolean]; volume: [visible: boolean] }>()
const { t, locale } = useI18n()
const point = computed(() => props.points.find((row) => row.time === props.time) ?? props.points.at(-1))
const priceFormat = computed(() => resolveMarketChartPriceFormat(props.points))
function price(value?: number): string {
  if (value === undefined) return '--'
  if (priceFormat.value.type === 'custom') return priceFormat.value.formatter(value)
  return value.toLocaleString(locale.value, {
    minimumFractionDigits: priceFormat.value.precision, maximumFractionDigits: priceFormat.value.precision,
  })
}
const timestamp = computed(() => point.value
  ? new Date(point.value.time * 1000).toLocaleString(locale.value, {
    month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false, timeZone: 'UTC',
  }) + ' UTC' : '--')
const averageRows = computed(() => ([5, 10, 20] as const).map((period) => ({
  period, value: props.averages[`ma${period}`].find((row) => row.time === point.value?.time)?.value,
})))
</script>

<template>
  <div class="chart-inspector">
    <div class="chart-inspector__toolbar">
      <label><input type="checkbox" checked @change="$emit('averages', ($event.target as HTMLInputElement).checked)" />MA</label>
      <label><input type="checkbox" checked @change="$emit('volume', ($event.target as HTMLInputElement).checked)" />{{ t('marketDetail.candleVolume') }}</label>
      <div class="chart-inspector__tools">
        <button type="button" :disabled="!point" :title="t('marketDetail.zoomOut')" :aria-label="t('marketDetail.zoomOut')" @click="$emit('zoom', 1.4)"><ZoomOut :size="18" /></button>
        <button type="button" :disabled="!point" :title="t('marketDetail.zoomIn')" :aria-label="t('marketDetail.zoomIn')" @click="$emit('zoom', 1 / 1.4)"><ZoomIn :size="18" /></button>
        <button type="button" :disabled="!point" :title="t('marketDetail.returnLatest')" :aria-label="t('marketDetail.returnLatest')" @click="$emit('latest')"><ArrowRightToLine :size="18" /></button>
      </div>
    </div>
    <div class="chart-inspector__readout" :data-inspection-time="point?.time">
      <time>{{ timestamp }}</time>
      <span>{{ t('marketDetail.candleVolume') }} {{ point?.volume.toLocaleString(locale) ?? '--' }}</span>
      <dl>
        <div v-for="key in (['open', 'high', 'low', 'close'] as const)" :key="key">
          <dt>{{ t(`marketDetail.candle${key[0]!.toUpperCase()}${key.slice(1)}`) }}</dt><dd>{{ price(point?.[key]) }}</dd>
        </div>
      </dl>
      <div class="chart-inspector__averages">
        <span v-for="row in averageRows" :key="row.period" :class="`ma-${row.period}`">MA{{ row.period }} {{ price(row.value) }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chart-inspector { color: var(--muted-strong); font-size: 11px; font-variant-numeric: tabular-nums; padding: 0 8px 6px; }
.chart-inspector__toolbar, .chart-inspector__tools { display: flex; align-items: center; gap: 4px; }
.chart-inspector__toolbar { justify-content: space-between; border-bottom: 1px solid var(--line); }
.chart-inspector label { display: flex; align-items: center; gap: 4px; min-height: 44px; cursor: pointer; }
.chart-inspector input { accent-color: var(--focus); }
.chart-inspector button { display: grid; place-items: center; width: 44px; height: 44px; padding: 0; border: 0; background: transparent; color: var(--text); }
.chart-inspector button:disabled { opacity: .4; }
.chart-inspector button:focus-visible, .chart-inspector input:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
.chart-inspector__readout { padding-top: 6px; min-height: 94px; }
.chart-inspector__readout > span { float: right; max-width: 48%; overflow-wrap: anywhere; }
.chart-inspector dl { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 6px; margin: 6px 0; }
.chart-inspector dt { color: var(--muted); }
.chart-inspector dd { margin: 2px 0 0; color: var(--text); overflow-wrap: anywhere; }
.chart-inspector__averages { display: flex; flex-wrap: wrap; gap: 4px 12px; overflow-wrap: anywhere; }
.ma-5 { color: var(--yellow, var(--signal-yellow, var(--positive))); }
.ma-10 { color: var(--coral, var(--signal-coral, var(--negative))); }
.ma-20 { color: var(--cyan, var(--signal-blue, var(--muted))); }
</style>
