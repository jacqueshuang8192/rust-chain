<script setup lang="ts">
import { computed, ref } from 'vue'
import { Check, ChevronDown, Search, X } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import { useModalDialog } from '@/core/modalDialog'
import AssetMark from './AssetMark.vue'

const props = defineProps<{
  label: string
  value: string
  selected: string
  choices: { value: string; label: string; description?: string; symbol?: string; logoUrl?: string; fallbackLogoUrl?: string }[]
  disabled?: boolean
}>()
const emit = defineEmits<{ select: [value: string] }>()
const { t } = useI18n()
const activeChoice = computed(() => props.choices.find(item => item.value === props.selected))
const open = ref(false)
const query = ref('')
const dialog = ref<HTMLElement | null>(null)
const { trapFocus } = useModalDialog(open, dialog)
const filtered = computed(() => {
  const term = query.value.trim().toLowerCase()
  return props.choices.filter(item => `${item.label} ${item.description || ''}`.toLowerCase().includes(term))
})
function show() { if (!props.disabled) { query.value = ''; open.value = true } }
function close() { open.value = false }
function choose(value: string) { if (props.disabled) return; close(); emit('select', value) }
</script>

<template>
  <button class="deposit-choice" type="button" :disabled="disabled" aria-haspopup="dialog" :aria-expanded="open" @click="show">
    <AssetMark v-if="activeChoice?.symbol" :symbol="activeChoice.symbol" :src="activeChoice.logoUrl" :fallback-src="activeChoice.fallbackLogoUrl" :size="36" />
    <span><small>{{ label }}</small><strong>{{ value || '—' }}</strong></span>
    <ChevronDown :size="20" aria-hidden="true" />
  </button>
  <Teleport to="body">
    <div v-if="open" class="pencil-page deposit-picker-overlay" @click.self="close">
      <section ref="dialog" class="deposit-picker" role="dialog" aria-modal="true" :aria-label="label" tabindex="-1" @keydown="trapFocus($event, close)">
        <header><h2>{{ label }}</h2><button type="button" :aria-label="t('common.close')" @click="close"><X :size="22" aria-hidden="true" /></button></header>
        <label class="deposit-picker-search"><Search :size="20" aria-hidden="true" /><input v-model="query" data-dialog-initial type="search" :aria-label="t('timedDeposit.searchOptions')" :placeholder="t('timedDeposit.searchOptions')" /></label>
        <div class="deposit-picker-options">
          <button v-for="item in filtered" :key="item.value" type="button" :aria-pressed="item.value === selected" :disabled="disabled" @click="choose(item.value)">
            <AssetMark v-if="item.symbol" :symbol="item.symbol" :src="item.logoUrl" :fallback-src="item.fallbackLogoUrl" :size="36" />
            <span><strong>{{ item.label }}</strong><small v-if="item.description">{{ item.description }}</small></span>
            <Check v-if="item.value === selected" :size="22" aria-hidden="true" />
          </button>
          <p v-if="!filtered.length" role="status">{{ t('timedDeposit.noMatches') }}</p>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style>
.timed-deposit button.deposit-choice { display: flex; align-items: center; justify-content: space-between; gap: 16px; min-height: 80px; text-align: left; padding: 16px; border-radius: 0; }
.deposit-choice > span:not(.asset-mark) { flex: 1; display: grid; gap: 6px; min-width: 0; }
.deposit-choice small { color: var(--muted-strong); font-size: 13px; }
.deposit-choice strong { font-size: 17px; font-weight: 600; overflow-wrap: anywhere; }
.deposit-choice > svg { flex-shrink: 0; }
.pencil-page.deposit-picker-overlay { position: fixed; inset: 0; z-index: calc(var(--layer-overlay, 80) + 2); background: var(--overlay, #0008); display: flex; align-items: end; justify-content: center; }
.deposit-picker { box-sizing: border-box; width: min(100%, 448px); max-height: calc(90dvh - env(safe-area-inset-top)); padding: 20px 20px calc(20px + env(safe-area-inset-bottom)); background: var(--surface-elevated); color: var(--text); border-radius: 24px 24px 0 0; display: flex; flex-direction: column; gap: 20px; animation: deposit-sheet-enter 180ms ease-out; }
.deposit-picker header { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.deposit-picker h2 { margin: 0; font-size: 22px; }
.deposit-picker header button { display: grid; place-items: center; width: 44px; height: 44px; flex-shrink: 0; padding: 0; border: 0; border-radius: 50%; background: var(--surface-2); color: var(--text); }
.deposit-picker-search { display: flex; align-items: center; gap: 10px; padding: 0 14px; background: var(--surface-2); min-height: 52px; flex-shrink: 0; }
.deposit-picker-search input { width: 100%; min-width: 0; min-height: 52px; background: transparent; border: 0; outline: 0; color: var(--text); font: inherit; font-size: 16px; }
.deposit-picker-search input:focus, .deposit-picker-search input:focus-visible { outline: 0; box-shadow: none; } /* taste-skill-ignore: remove inherited shadow; focus belongs to the container. */
.deposit-picker-options { overflow-y: auto; overscroll-behavior: contain; min-height: 0; display: grid; gap: 8px; }
.deposit-picker-options button { width: 100%; min-height: 64px; padding: 14px; display: flex; align-items: center; justify-content: space-between; gap: 16px; text-align: left; background: var(--surface-2); color: var(--text); border: 0; font: inherit; }
.deposit-picker-options button[aria-pressed='true'] { background: var(--accent-soft); }
.deposit-picker-options button > span:not(.asset-mark) { flex: 1; display: grid; gap: 6px; min-width: 0; overflow-wrap: anywhere; }
.deposit-picker-options svg { flex-shrink: 0; color: var(--positive); }
.deposit-picker-options small { color: var(--muted-strong); font-size: 13px; line-height: 1.5; }
.deposit-picker-options p { color: var(--muted-strong); padding: 16px 0; }
.deposit-picker button:focus-visible, .deposit-picker-search:focus-within { outline: 2px solid var(--focus); outline-offset: 2px; }
.deposit-picker button { cursor: pointer; }
.deposit-picker button:active { opacity: .75; }
@keyframes deposit-sheet-enter { from { transform: translateY(24px); opacity: 0; } to { transform: translateY(0); opacity: 1; } }
@media (prefers-reduced-motion: reduce) { .deposit-picker { animation: none; } }
</style>
