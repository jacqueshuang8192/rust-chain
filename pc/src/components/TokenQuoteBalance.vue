<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { fetchTokenQuoteBalance } from '@/api/tokenIssuance'
import { formatTokenAmount } from '@/core/tokenIssuance'
import { tokenIssuanceMessages } from '@/core/tokenIssuanceMessages'
const props = defineProps<{ owner: string; assetId: number; symbol: string; revision: number }>()
const { t } = useI18n({ useScope: 'local', messages: tokenIssuanceMessages })
const balance = ref<string | null>(null), loading = ref(false), failed = ref(false)
let generation = 0, active = true
const identity = () => `${props.owner}:${props.assetId}:${props.symbol}`
async function load() {
  const g = ++generation, key = identity()
  if (!props.owner) { balance.value = null; loading.value = false; return }
  loading.value = true; failed.value = false
  try {
    const value = await fetchTokenQuoteBalance(props.assetId, props.symbol)
    if (active && g === generation && key === identity()) balance.value = value
  } catch {
    if (active && g === generation && key === identity()) { balance.value = null; failed.value = true }
  } finally { if (active && g === generation) loading.value = false }
}
watch(identity, () => { balance.value = null; void load() }, { immediate: true })
watch(() => props.revision, () => { void load() })
const timer = setInterval(() => {
  if (props.owner && !loading.value && document.visibilityState === 'visible') void load()
}, 10_000)
onBeforeUnmount(() => { active = false; generation++; clearInterval(timer) })
</script>
<template>
  <div v-if="owner" class="token-quote-balance" :aria-busy="loading">
    <span>{{ t('token.availableBalance') }}</span>
    <strong>{{ loading && balance === null ? t('token.processing') : formatTokenAmount(balance) }} <small>{{ symbol }}</small></strong>
    <button v-if="failed" class="token-control" type="button" @click="load">{{ t('token.balanceRetry') }}</button>
  </div>
</template>
<style scoped>
.token-quote-balance { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 8px; padding-block: 10px; font-size: 12px; color: var(--muted); }
strong { min-width: 0; color: var(--text); font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
small { font-size: inherit; font-weight: 400; }
button { min-height: 44px; border: 1px solid var(--line); border-radius: 10px; padding: 8px 12px; color: var(--text); background: var(--surface-2); cursor: pointer; }
</style>
