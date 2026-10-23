<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import LoginRequiredState from '@/components/LoginRequiredState.vue'
import PageHeader from '@/components/PageHeader.vue'
import TimedDepositPanel from '@/components/TimedDepositPanel.vue'
import { useSessionStore } from '@/stores/session'
defineProps<{ asset: string; network: string }>()
const session = useSessionStore()
const { t } = useI18n()
</script>
<template>
  <main class="page page--plain pencil-page wallet-pencil-page deposit-detail-pencil" data-pencil-source="w5htG TCN5A">
    <PageHeader :back="true" :eyebrow="t('assets.deposit')" :fallback="{ name: 'deposit-network', params: { asset } }" :pencil="true" :subtitle="t('deposit.networkWarning')" :title="t('timedDeposit.title')" />
    <div class="page-content deposit-detail">
      <LoginRequiredState class="wallet-login-prompt" v-if="!session.isAuthenticated" :description="t('deposit.detailLoginDescription')" />
      <TimedDepositPanel v-else :identity="session.identityScope" :asset="asset" :network="network" />
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

.wallet-login-prompt {
  background: transparent;
  background-image: none;
  border: 0;
  border-top: 1px solid var(--hairline);
  gap: 10px;
  grid-template-columns: 34px minmax(0, 1fr) auto;
  min-height: 72px;
  padding: 10px 0;
}

.wallet-login-prompt :deep(.login-required__icon) {
  background: var(--accent-soft);
  border: 0;
  color: var(--positive);
  height: 34px;
  width: 34px;
}

.wallet-login-prompt :deep(.login-required__copy) {
  gap: 2px;
}

.wallet-login-prompt :deep(.login-required__copy strong) {
  font-size: 13px;
}

.wallet-login-prompt :deep(.login-required__copy p) {
  color: var(--muted);
  font-size: 11px;
  line-height: 1.4;
}

.wallet-login-prompt :deep(.button) {
  border-radius: var(--wallet-pill-radius, 999px);
  min-height: 44px;
  padding-inline: 14px;
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
