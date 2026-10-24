<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ChevronRight, LoaderCircle } from 'lucide-vue-next'
import AssetMark from '@/components/AssetMark.vue'
import LoginRequiredState from '@/components/LoginRequiredState.vue'
import PageHeader from '@/components/PageHeader.vue'
import { apiErrorMessage } from '@/api/client'
import { fetchDepositAssets } from '@/api/wallet'
import { fetchTimedDepositOptions } from '@/api/timedDeposits'
import type { DepositOption } from '@/core/timedDeposit'
import { useSessionStore } from '@/stores/session'
import type { DepositAsset } from '@/core/types'

const props = defineProps<{ asset: string }>()
const router = useRouter()
const session = useSessionStore()
const { t } = useI18n()
const networks = ref<DepositOption[]>([])
const selectedAsset = ref<DepositAsset | null>(null)
const error = ref('')
const loading = ref(false)

async function load(): Promise<void> {
  if (!session.isAuthenticated) return
  loading.value = true
  error.value = ''
  try {
    const assets = await fetchDepositAssets()
    selectedAsset.value = assets.find((asset) => asset.symbol === props.asset.toUpperCase()) || null
    networks.value = (await fetchTimedDepositOptions()).filter(item => item.asset_symbol === props.asset.toUpperCase())
  } catch (reason) {
    error.value = apiErrorMessage(reason, t('deposit.networkLoadFailed'))
  } finally {
    loading.value = false
  }
}

function chooseNetwork(network: DepositOption) {
  void router.push({ name: 'deposit-detail', params: { asset: props.asset, network: network.network } })
}

onMounted(() => { void load() })
</script>

<template>
  <main
    class="page page--plain pencil-page wallet-pencil-page deposit-network-pencil"
    data-pencil-source="y4ifR qKfsZ"
  >
    <PageHeader
      :back="true"
      :eyebrow="t('assets.deposit')"
      :fallback="{ name: 'deposit-asset' }"
      :pencil="true"
      :subtitle="t('deposit.networkNoteDescription')"
      :title="t('deposit.selectNetwork')"
    />
    <div class="page-content network-page">
      <LoginRequiredState
        v-if="!session.isAuthenticated"
        class="wallet-login-prompt"
        :description="t('deposit.networkLoginDescription')"
      />
      <template v-else>
        <section class="network-summary">
          <AssetMark
            :symbol="selectedAsset?.symbol || asset.toUpperCase()"
            :src="selectedAsset?.logoUrl"
            :size="36"
          />
          <div>
            <strong>{{ selectedAsset?.symbol || asset.toUpperCase() }}</strong>
            <small>{{ t('assets.fundingAccount') }}</small>
          </div>
        </section>
        <p class="network-warning">{{ t('deposit.networkNoteDescription') }}</p>
        <p v-if="error" class="error-message wallet-feedback" role="alert">{{ error }}</p>
        <div v-if="loading" class="loading-state" role="status" :aria-label="t('common.loading')">
          <LoaderCircle :size="22" class="spin" aria-hidden="true" />
          <span>{{ t('common.loading') }}</span>
        </div>
        <div v-else-if="networks.length" class="network-list">
          <button v-for="network in networks" :key="network.network" type="button" @click="chooseNetwork(network)">
            <span>
              <strong>{{ network.display_name }}</strong>
              <small>
                {{ t('deposit.minimum') }} · {{ network.min_deposit_amount }} {{ asset.toUpperCase() }}
              </small>
            </span>
            <ChevronRight :size="16" aria-hidden="true" />
          </button>
        </div>
        <p v-else class="empty-state">{{ t('deposit.noNetworks') }}</p>
      </template>
    </div>
  </main>
</template>

<style scoped>
.wallet-pencil-page {
  background: var(--page);
}

.network-page {
  display: grid;
  gap: 12px;
  padding: 6px 20px calc(20px + env(safe-area-inset-bottom));
}

.network-summary {
  align-items: center;
  display: flex;
  gap: 12px;
  min-height: 48px;
  padding: 4px 0 8px;
}

.network-summary > div {
  display: grid;
  gap: 3px;
  min-width: 0;
}

.network-summary strong {
  font-size: 14px;
  line-height: 20px;
}

.network-summary small {
  color: var(--muted);
  font-size: 11px;
  line-height: 15px;
}

.network-warning {
  color: var(--negative);
  font-size: 11px;
  font-weight: 500;
  line-height: 1.45;
  margin: 0;
}

.loading-state {
  align-items: center;
  color: var(--muted);
  display: flex;
  font-size: 13px;
  gap: 9px;
  justify-content: center;
  min-height: 104px;
}

.network-list {
  display: grid;
}

.network-list button {
  align-items: center;
  background: transparent;
  border-bottom: 1px solid var(--hairline);
  border-radius: 0;
  color: var(--ink);
  display: grid;
  gap: 12px;
  grid-template-columns: minmax(0, 1fr) 16px;
  height: 56px;
  min-height: 56px;
  padding: 0;
  text-align: left;
  width: 100%;
}

.network-list button:hover {
  background: var(--surface-elevated);
}

.network-list button > span {
  display: grid;
  gap: 3px;
  min-width: 0;
}

.network-list strong {
  font-size: 14px;
  line-height: 20px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.network-list small {
  color: var(--muted);
  font-size: 11px;
  line-height: 15px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.network-list svg {
  color: var(--muted);
  justify-self: end;
}

.wallet-feedback {
  margin: 0;
}

.spin {
  animation: spin .8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

@media (max-width: 340px) {
  .network-page {
    padding-left: 16px;
    padding-right: 16px;
  }

  .network-list button {
    gap: 9px;
  }

  .wallet-login-prompt {
    align-items: center;
    grid-template-columns: 34px minmax(0, 1fr);
  }

  .wallet-login-prompt :deep(.button) {
    grid-column: 2;
    justify-self: start;
  }
}

@media (prefers-reduced-motion: reduce) {
  .spin {
    animation: none;
  }
}
</style>
