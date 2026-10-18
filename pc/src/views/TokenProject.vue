<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import request from '@/api/request'
import { backendApiUrl } from '@/api/backendAdapters'
import { parseTokenPolicy, tokenDecimal, tokenObject, type TokenPolicy } from '@/core/tokenIssuance'
import { tokenIssuanceMessages } from '@/core/tokenIssuanceMessages'
import TokenTradingPanel from '@/components/TokenTradingPanel.vue'
import TokenSubscriptionForm from '@/components/TokenSubscriptionForm.vue'
import TokenIssuanceRules from '@/components/TokenIssuanceRules.vue'
const route = useRoute()
const { t } = useI18n({ useScope: 'local', messages: tokenIssuanceMessages })
const project = ref<{ id: number; symbol: string; quote: string; quoteId: number; price: string; pair?: string; stage: string; policy: TokenPolicy } | null>(null)
const error = ref(''); const loading = ref(false)
let generation = 0
const symbol = computed(() => String(route.params.symbol || ''))
async function load() {
  const g = ++generation; loading.value = true; error.value = ''; project.value = null
  try {
    const r = tokenObject((await request.instance.get(backendApiUrl(`/new-coins/${encodeURIComponent(symbol.value)}`))).data)
    const policy = parseTokenPolicy(r.issuance_policy)
    if (!Number.isSafeInteger(r.id) || !Number.isSafeInteger(r.quote_asset_id) || typeof r.symbol !== 'string' || typeof r.quote_asset_symbol !== 'string' || typeof r.lifecycle_status !== 'string') throw new Error()
    const next = { id: r.id as number, symbol: r.symbol, quote: r.quote_asset_symbol, quoteId: r.quote_asset_id as number, price: tokenDecimal(r.issue_price), stage: r.lifecycle_status, pair: typeof r.post_listing_pair_symbol === 'string' ? r.post_listing_pair_symbol : undefined, policy }
    if (g === generation) project.value = next
  } catch { if (g === generation) error.value = t('token.failure') }
  finally { if (g === generation) loading.value = false }
}
watch(symbol, () => { void load() }, { immediate: true })
</script>
<template>
  <main class="token-project">
    <header><RouterLink to="/launchpad">{{ t('token.all') }}</RouterLink><h1>{{ symbol }}</h1><button :disabled="loading" type="button" @click="load">{{ t('token.refreshProject') }}</button></header>
    <p v-if="loading">{{ t('token.processing') }}</p><p v-if="error" role="alert">{{ error }}</p>
    <template v-if="project">
      <TokenTradingPanel :project-id="project.id" :symbol="project.symbol" :quote-symbol="project.quote" :pair-symbol="project.pair" :stage="project.stage" />
      <TokenSubscriptionForm v-if="project.stage === 'subscription'" :project-id="project.id" :symbol="project.symbol" :quote-symbol="project.quote" :quote-asset-id="project.quoteId" :price="project.price" />
      <TokenIssuanceRules :policy="project.policy" :quote-symbol="project.quote" />
    </template>
  </main>
</template>
<style scoped>
.token-project { --text: hsl(var(--foreground)); --muted: hsl(var(--muted-foreground)); --line: hsl(var(--border)); --surface: hsl(var(--card)); --surface-2: hsl(var(--muted)); --focus: hsl(var(--primary)); width: min(100%, 1000px); margin: 0 auto; padding: 24px 16px; color: var(--text); } header { display: flex; align-items: center; gap: 20px; flex-wrap: wrap; } h1 { flex: 1; font-size: 28px; } button { min-height: 44px; padding: 10px 16px; border: 1px solid var(--line); border-radius: 8px; } p { padding: 16px; }
</style>
