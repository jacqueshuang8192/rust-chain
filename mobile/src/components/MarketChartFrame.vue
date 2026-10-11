<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Maximize2, Minimize2 } from 'lucide-vue-next'
import { MARKET_KLINE_INTERVALS, type MarketKlineInterval } from '@/api/marketSocketProtocol'
import { useModalDialog } from '@/core/modalDialog'
import { createChartOrientation } from '@/core/chartOrientation'
import { bindRotatedChartInput } from '@/core/chartRotatedInput'
import { useThemeStore } from '@/stores/theme'

const props = defineProps<{ symbol: string; interval: string }>()
defineEmits<{ interval: [value: MarketKlineInterval] }>()
const { t } = useI18n()
const theme = useThemeStore()
const expanded = ref(false)
const host = ref<HTMLElement | null>(null)
const panel = ref<HTMLElement | null>(null)
const size = ref({ width: 0, height: 0 })
const rotated = computed(() => expanded.value && size.value.width < size.value.height)
const { trapFocus } = useModalDialog(expanded, panel)
let scroll = { x: 0, y: 0 }
let alive = true
let orientation: ReturnType<typeof createChartOrientation> | undefined
let stopInput = () => {}

function measure(): void {
  size.value = { width: window.visualViewport?.width ?? window.innerWidth,
    height: window.visualViewport?.height ?? window.innerHeight }
}

function close(): void {
  if (!expanded.value) return
  expanded.value = false
  orientation?.exit()
  void nextTick(() => { if (alive && !expanded.value) window.scrollTo(scroll.x, scroll.y) })
}

async function toggle(): Promise<void> {
  if (expanded.value) return close()
  scroll = { x: window.scrollX, y: window.scrollY }
  measure()
  expanded.value = true
  await nextTick()
  if (alive && expanded.value && host.value) await orientation?.enter(host.value)
}

function keydown(event: KeyboardEvent): void {
  if (expanded.value) trapFocus(event, close)
}

watch(() => props.symbol, close)
onMounted(() => {
  orientation = createChartOrientation({
    fullscreenElement: () => document.fullscreenElement,
    exitFullscreen: () => document.exitFullscreen(),
    orientation: screen.orientation,
    onFullscreenChange(listener) {
      document.addEventListener('fullscreenchange', listener)
      return () => document.removeEventListener('fullscreenchange', listener)
    },
  }, close)
  if (host.value) stopInput = bindRotatedChartInput(host.value, () => rotated.value)
  window.addEventListener('resize', measure)
  window.visualViewport?.addEventListener('resize', measure)
  document.addEventListener('keydown', keydown)
})
onBeforeUnmount(() => {
  alive = false
  orientation?.dispose()
  stopInput()
  window.removeEventListener('resize', measure)
  window.visualViewport?.removeEventListener('resize', measure)
  document.removeEventListener('keydown', keydown)
})
</script>

<template>
  <Teleport to="body" :disabled="!expanded">
    <div ref="host" class="market-chart-host" :class="{ 'is-expanded': expanded, 'app-stage': expanded, [`theme-${theme.theme}`]: expanded }"
      :style="expanded ? { '--chart-width': `${size.width}px`, '--chart-height': `${size.height}px` } : undefined"
      :data-chart-mode="expanded ? 'expanded' : 'inline'">
      <section ref="panel" class="market-chart-frame" :class="{ 'is-rotated': rotated }"
        :role="expanded ? 'dialog' : 'region'" :aria-modal="expanded || undefined"
        :aria-label="`${t('marketDetail.chartWorkstation')} ${symbol}`">
        <header class="market-chart-frame__toolbar">
          <strong v-if="expanded" class="market-chart-frame__symbol">{{ symbol }}</strong>
          <nav :aria-label="t('marketDetail.chartInterval')">
            <button v-for="value in MARKET_KLINE_INTERVALS" :key="value" type="button"
              :aria-pressed="interval === value" @click="$emit('interval', value)">{{ t(`marketDetail.intervals.${value}`) }}</button>
          </nav>
          <slot name="tools" />
          <button type="button" class="market-chart-frame__toggle" data-dialog-initial :aria-pressed="expanded"
            :aria-label="t(expanded ? 'marketDetail.collapseChart' : 'marketDetail.expandChart')"
            :title="t(expanded ? 'marketDetail.collapseChart' : 'marketDetail.expandChart')" @click="toggle">
            <Minimize2 v-if="expanded" :size="18" /><Maximize2 v-else :size="18" />
          </button>
        </header>
        <div class="market-chart-frame__canvas"><slot /></div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.market-chart-host, .market-chart-frame { width: 100%; height: 100%; min-width: 0; min-height: 0; }
.market-chart-frame { display: grid; grid-template-rows: 44px minmax(0, 1fr); background: var(--surface); color: var(--text); }
.market-chart-frame__toolbar { display: flex; align-items: center; gap: 8px; padding: 0 8px; border-bottom: 1px solid var(--line); min-width: 0; }
.market-chart-frame__toolbar nav { display: flex; flex: 1; min-width: 0; overflow-x: auto; scrollbar-width: none; }
.market-chart-frame__toolbar :deep(button) { height: 44px; min-width: 44px; border: 0; box-shadow: none; background: transparent; color: var(--muted-strong); padding: 0 4px; font-size: 12px; }
.market-chart-frame__toolbar :deep(.chart-action-rail) { display: flex; flex: none; }
.market-chart-frame__toolbar button[aria-pressed='true'] { color: var(--text); font-weight: 700; box-shadow: inset 0 -2px var(--focus); }
.market-chart-frame__toolbar button:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
.market-chart-frame__toolbar .market-chart-frame__toggle { display: grid; place-items: center; padding: 0; width: 44px; flex: none; box-shadow: none; color: var(--text); }
.market-chart-frame__symbol { font-size: 13px; white-space: nowrap; }
.market-chart-frame__canvas { min-width: 0; min-height: 0; overflow: hidden; }
.market-chart-host.is-expanded { position: fixed; inset: 0; width: var(--chart-width); height: var(--chart-height); min-height: 0; margin: 0; padding: 0; z-index: var(--layer-overlay, 200); background: var(--surface); overflow: hidden; touch-action: none; }
.is-expanded .market-chart-frame { box-sizing: border-box; padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left); }
.is-expanded .market-chart-frame.is-rotated { position: absolute; left: var(--chart-width); top: 0; width: var(--chart-height); height: var(--chart-width); transform-origin: 0 0; transform: rotate(90deg); padding: env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left) env(safe-area-inset-top); }
.is-expanded :deep(.chart-inspector) { display: flex; align-items: center; flex-wrap: wrap; gap: 0 12px; }
.is-expanded :deep(.chart-inspector__toolbar) { flex: none; border: 0; }
.is-expanded :deep(.chart-inspector__readout) { flex: 1; min-width: 300px; min-height: 0; padding-top: 0; }
.is-expanded :deep(.chart-inspector__readout dl) { margin: 2px 0; }
.is-expanded :deep(.chart-inspector__readout dd) { margin: 0; }
@media (max-width: 340px) {
  .market-chart-frame__toolbar { gap: 0; padding-inline: 4px; }
}
</style>
