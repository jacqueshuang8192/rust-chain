<script setup lang="ts">
import { ref } from 'vue'
import { History } from 'lucide-vue-next'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import LoginRequiredState from '@/components/LoginRequiredState.vue'
import PageHeader from '@/components/PageHeader.vue'
import DepositOrderHistory from '@/components/DepositOrderHistory.vue'
import TimedDepositPanel from '@/components/TimedDepositPanel.vue'
import { useSessionStore } from '@/stores/session'
withDefaults(defineProps<{ asset?: string; network?: string; id?: string; history?: boolean }>(), { asset: '', network: '', id: '', history: false })
const depositPanel = ref<InstanceType<typeof TimedDepositPanel> | null>(null)
const router = useRouter()
const session = useSessionStore()
const { t } = useI18n()
</script>
<template>
  <main class="page page--plain pencil-page wallet-pencil-page deposit-detail-pencil" data-pencil-source="w5htG TCN5A">
    <PageHeader :back="true" :eyebrow="t('assets.deposit')" :fallback="id ? { name: 'deposit-orders' } : history ? { name: 'deposit-asset' } : { name: 'assets' }" :pencil="true" :subtitle="t('deposit.networkWarning')" :title="t(id ? 'timedDeposit.order' : history ? 'timedDeposit.history' : 'timedDeposit.title')">
      <template v-if="session.isAuthenticated && !history" #actions>
        <button class="icon-button" type="button" :aria-label="t('timedDeposit.history')" :title="t('timedDeposit.history')" :disabled="depositPanel?.busy" @click="router.push({ name: 'deposit-orders' })">
          <History :size="22" aria-hidden="true" />
        </button>
      </template>
    </PageHeader>
    <div class="page-content deposit-detail">
      <LoginRequiredState class="wallet-login-prompt" v-if="!session.isAuthenticated" :description="t('deposit.detailLoginDescription')" />
      <DepositOrderHistory v-else-if="history" :identity="session.identityScope" @select="router.push({ name: 'deposit-order', params: { id: $event } })" @create="router.push({ name: 'deposit-asset' })" />
      <TimedDepositPanel v-else ref="depositPanel" :identity="session.identityScope" :asset="asset" :network="network" :order-id="id" />
    </div>
  </main>
</template>
<style scoped>
.wallet-pencil-page {
  background: var(--page);
}

.deposit-detail {
  display: grid;
  gap: 14px;
  padding: 6px 20px calc(20px + env(safe-area-inset-bottom));
}

@media (max-width: 340px) {
  .deposit-detail {
    padding-inline: 16px;
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

</style>
