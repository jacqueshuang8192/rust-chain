<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import AssetMark from './AssetMark.vue'
import { walletTotal, walletFrozen } from '@/core/walletAmounts'
import { formatFinancialAmount } from '@/core/financialDisplay'
import type { WalletAccount } from '@/core/types'
const props = defineProps<{ wallet: WalletAccount }>()
const { t, locale } = useI18n()
const amount = (value: string | undefined) => formatFinancialAmount(value, locale.value, { assetSymbol: props.wallet.symbol })
</script>
<template>
  <article class="spot-holding-card">
    <header>
      <div class="spot-holding-identity"><AssetMark :symbol="wallet.symbol" :src="wallet.logoUrl" :size="32" /><strong>{{ wallet.symbol }}</strong></div>
      <div class="spot-holding-total"><span>{{ t('trade.holdingQuantity') }}</span><strong class="numeric" :title="walletTotal(wallet)">{{ amount(walletTotal(wallet)) }}</strong></div>
    </header>
    <dl>
      <div><dt>{{ t('trade.available') }}</dt><dd class="numeric" :title="wallet.availableText">{{ amount(wallet.availableText) }}</dd></div>
      <div><dt>{{ t('trade.frozenLocked') }}</dt><dd class="numeric" :title="walletFrozen(wallet)">{{ amount(walletFrozen(wallet)) }}</dd></div>
    </dl>
  </article>
</template>
<style scoped>
.spot-holding-card { min-width: 0; padding: 14px; border: 1px solid var(--line); border-radius: 12px; background: var(--surface-2); }
.spot-holding-card header { display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 12px; align-items: center; }
.spot-holding-identity { display: flex; align-items: center; gap: 8px; font-size: 14px; }
.spot-holding-total { min-width: 0; text-align: right; }
.spot-holding-total span, .spot-holding-card dt { color: var(--muted); font-size: 11px; line-height: 1.5; }
.spot-holding-total strong { display: block; font-size: 16px; line-height: 1.5; overflow-wrap: anywhere; }
.spot-holding-card dl { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; margin: 12px 0 0; padding-top: 12px; border-top: 1px solid var(--line); }
.spot-holding-card dd { margin: 4px 0 0; font-size: 12px; line-height: 1.5; overflow-wrap: anywhere; }

</style>
