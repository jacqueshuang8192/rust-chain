<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { TokenPolicy } from '@/core/tokenIssuance'
import { tokenIssuanceMessages } from '@/core/tokenIssuanceMessages'
defineProps<{ policy: TokenPolicy; quoteSymbol?: string }>()
const { t, locale } = useI18n({ useScope: 'local', messages: tokenIssuanceMessages })
const nodes = ['subscription', 'distribution', 'purchase', 'listing'] as const
const date = (at: number) => new Date(at).toLocaleString(locale.value, { year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false })
</script>
<template>
  <section class="token-surface token-rules">
    <h2>{{ t('token.rules') }}</h2>
    <dl class="token-data">
      <div><dt>{{ t('token.seed') }}</dt><dd>{{ policy.purchase_initial_price }} {{ quoteSymbol }}</dd></div>
      <div v-for="node in nodes" :key="node"><dt>{{ t('token.plannedTime', { stage: t(`token.${node}`) }) }}</dt><dd><time>{{ date(policy[node].at) }}</time></dd></div>
      <div><dt>{{ t('token.lock') }}</dt><dd>{{ t(`token.${policy.lock.kind}`, { count: 'count' in policy.lock ? policy.lock.count : '' }) }}<template v-if="policy.lock.kind === 'fixed'"> · {{ date(policy.lock.at) }}</template></dd></div>
      <div><dt>{{ t('token.fee') }}</dt><dd>{{ policy.fee.kind === 'none' ? t('token.feeNone') : policy.fee.kind === 'fixed' ? t('token.feeFixed', { value: `${policy.fee.amount} ${quoteSymbol || ''}` }) : t(policy.fee.kind === 'profit' ? 'token.feeProfit' : 'token.feeGross', { value: policy.fee.rate }) }}</dd></div>
    </dl>
  </section>
</template>
