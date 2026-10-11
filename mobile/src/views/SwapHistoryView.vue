<script setup lang="ts">
import { computed, onBeforeUnmount, shallowRef, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ArrowRight, CircleAlert, History, LoaderCircle, RefreshCw } from 'lucide-vue-next'
import AssetMark from '@/components/AssetMark.vue'
import LoginRequiredState from '@/components/LoginRequiredState.vue'
import PageHeader from '@/components/PageHeader.vue'
import { apiErrorMessage } from '@/api/client'
import { fetchConvertOrdersPage, type ConvertOrder } from '@/api/swap'
import { formatDateTime, formatExactAmount } from '@/core/format'
import { convertStatusPresentation } from '@/core/financialEnumPresentation'
import { createHistoryPaginationController, type HistoryPaginationState } from '@/core/historyPagination'
import { useSessionStore } from '@/stores/session'

const { t } = useI18n()
const session = useSessionStore()
const history = shallowRef<HistoryPaginationState<ConvertOrder>>()
const pagination = createHistoryPaginationController({
  sessionKey: () => session.isAuthenticated ? `${session.generation}:${session.token}` : '',
  fetchPage: fetchConvertOrdersPage,
  pageSize: 20,
  identity: (order) => String(order.id),
  onChange: (value) => { history.value = value },
})
const state = computed(() => history.value ?? pagination.snapshot())
const error = computed(() => state.value.initialError === null ? '' : apiErrorMessage(state.value.initialError, t('swap.historyLoadFailed')))
const appendError = computed(() => state.value.appendError === null ? '' : apiErrorMessage(state.value.appendError, t('swap.historyLoadFailed')))
function statusLabel(order: ConvertOrder): string {
  const presentation = convertStatusPresentation(order.status)
  return t(presentation.translationKey, { source: presentation.source || '--' })
}
watch(() => [session.token, session.generation] as const, () => {
  pagination.reset()
  if (session.isAuthenticated) void pagination.loadInitial()
}, { immediate: true })
onBeforeUnmount(() => pagination.stop())
</script>

<template>
  <main class="page page--plain pencil-page swap-history-page">
    <PageHeader :back="true" :pencil="true" :fallback="{ name: 'swap' }" :title="t('swap.history')">
      <template #actions>
        <button class="icon-button" type="button" :aria-label="t('swap.historyRefresh')"
          :disabled="!session.isAuthenticated || state.loading || state.loadingMore" @click="pagination.loadInitial">
          <RefreshCw :size="18" aria-hidden="true" />
        </button>
      </template>
    </PageHeader>
    <div class="swap-history-content">
      <LoginRequiredState v-if="!session.isAuthenticated" :description="t('swap.historyLoginDescription')" />
      <template v-else>
        <div v-if="state.loading" class="swap-history-state" role="status"><LoaderCircle :size="22" class="spin" /><span>{{ t('swap.historyLoading') }}</span></div>
        <div v-else-if="error" class="swap-history-state" role="alert">
          <CircleAlert :size="22" /><span>{{ error }}</span><button type="button" @click="pagination.loadInitial">{{ t('common.retry') }}</button>
        </div>
        <div v-else-if="!state.items.length" class="swap-history-state" role="status"><History :size="26" /><span>{{ t('swap.emptyHistory') }}</span></div>
        <div v-else role="list">
          <article v-for="order in state.items" :key="order.id" class="swap-history-record" role="listitem">
            <header><time>{{ formatDateTime(order.createdAt) }}</time><span :class="`is-${convertStatusPresentation(order.status).tone}`">{{ statusLabel(order) }}</span></header>
            <div class="swap-history-assets">
              <div class="swap-history-asset">
                <AssetMark :symbol="order.fromAssetSymbol || t('swap.asset')" :src="order.fromAssetLogoUrl" :size="32" />
                <span>{{ t('swap.pay') }}<strong>{{ order.fromAssetSymbol || t('swap.asset') }}</strong></span>
                <b class="numeric" :title="order.fromAmount">{{ formatExactAmount(order.fromAmount) }}</b>
              </div>
              <ArrowRight :size="18" aria-hidden="true" />
              <div class="swap-history-asset">
                <AssetMark :symbol="order.toAssetSymbol || t('swap.asset')" :src="order.toAssetLogoUrl" :size="32" />
                <span>{{ t('swap.receive') }}<strong>{{ order.toAssetSymbol || t('swap.asset') }}</strong></span>
                <b class="numeric" :title="order.toAmount">{{ formatExactAmount(order.toAmount) }}</b>
              </div>
            </div>
            <dl><div><dt>{{ t('swap.referenceRate') }}</dt><dd>{{ formatExactAmount(order.rate) }}</dd></div>
              <div><dt>{{ t('common.fee') }}</dt><dd>{{ formatExactAmount(order.feeAmount) }} {{ order.fromAssetSymbol }}</dd></div></dl>
          </article>
        </div>
        <div v-if="appendError" class="swap-history-state" role="alert"><span>{{ appendError }}</span><button type="button" :disabled="state.loadingMore" @click="pagination.retryLoadMore">{{ t('common.retry') }}</button></div>
        <button v-else-if="!state.loading && !error && state.hasMore && state.items.length" class="swap-history-more" type="button" :disabled="state.loadingMore" :aria-busy="state.loadingMore" @click="pagination.loadMore">
          <LoaderCircle v-if="state.loadingMore" :size="18" class="spin" />{{ t(state.loadingMore ? 'common.loading' : 'common.loadMore') }}
        </button>
      </template>
    </div>
  </main>
</template>

<style scoped>
.swap-history-content { padding: 0 16px calc(24px + env(safe-area-inset-bottom)); }
.swap-history-record { border-bottom: 1px solid var(--line); padding: 16px 0; }
.swap-history-record header { display: flex; align-items: center; justify-content: space-between; gap: 8px; color: var(--muted-strong); font-size: 11px; }
.swap-history-record .is-positive { color: var(--positive); }
.swap-history-record .is-negative { color: var(--negative); }
.swap-history-assets { display: grid; grid-template-columns: minmax(0, 1fr) 18px minmax(0, 1fr); align-items: center; gap: 10px; margin-top: 14px; }
.swap-history-assets > svg { color: var(--muted-strong); }
.swap-history-asset { display: grid; grid-template-columns: 32px minmax(0, 1fr); align-items: center; gap: 8px; min-width: 0; }
.swap-history-asset > span { color: var(--muted-strong); font-size: 11px; }
.swap-history-asset strong { display: block; color: var(--text); font-size: 14px; overflow-wrap: anywhere; }
.swap-history-asset > b { grid-column: 1 / -1; color: var(--text); font-size: 14px; font-weight: 600; overflow-wrap: anywhere; }
.swap-history-record dl { display: grid; gap: 4px; margin: 12px 0 0; font-size: 11px; color: var(--muted-strong); }
.swap-history-record dl > div { display: flex; gap: 12px; justify-content: space-between; }
.swap-history-record dd { margin: 0; max-width: 70%; overflow-wrap: anywhere; text-align: right; }
.swap-history-state { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 12px; min-height: 180px; color: var(--muted-strong); text-align: center; font-size: 13px; }
.swap-history-state button, .swap-history-more { min-height: 44px; background: transparent; color: var(--text); border: 1px solid var(--line); border-radius: 6px; padding: 0 16px; }
.swap-history-more { display: flex; align-items: center; justify-content: center; gap: 8px; width: 100%; margin-top: 16px; }
.swap-history-content button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
.spin { animation: swap-history-spin .8s linear infinite; }
@keyframes swap-history-spin { to { transform: rotate(360deg); } }
@media (prefers-reduced-motion: reduce) { .spin { animation: none; } }
</style>
