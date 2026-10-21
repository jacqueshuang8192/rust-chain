<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import AssetMark from './AssetMark.vue'
import { walletTotal, walletFrozen } from '@/core/walletAmounts'
import { formatFinancialAmount } from '@/core/financialDisplay'
import { estimateWalletAsset } from '@/core/walletValuation'
import { normalizeDecimalText } from '@/core/decimal'
import { useMarketStore } from '@/stores/market'
import type { WalletAccount } from '@/core/types'
const props = defineProps<{ wallet: WalletAccount }>()
const { t, locale } = useI18n()
const market = useMarketStore()
const clock = ref(Date.now())
const timer = setInterval(() => { clock.value = Date.now() }, 10_000)
onBeforeUnmount(() => clearInterval(timer))
const price = computed(() => estimateWalletAsset(props.wallet.symbol, normalizeDecimalText('1'), props.wallet,
  market.tickerFor(`${props.wallet.symbol}/USDT`)?.lastPriceText, clock.value))
const value = computed(() => estimateWalletAsset(props.wallet.symbol, walletTotal(props.wallet), props.wallet,
  market.tickerFor(`${props.wallet.symbol}/USDT`)?.lastPriceText, clock.value))
const amount = (value: string | null | undefined, asset = props.wallet.symbol) => formatFinancialAmount(value, locale.value, { assetSymbol: asset, unavailable: '—' })
const priceText = computed(() => formatFinancialAmount(price.value, locale.value, { maximumFractionDigits: 8, unavailable: '—' }))
</script>
<template>
  <article class="spot-holding-card">
    <header><AssetMark :symbol="wallet.symbol" :src="wallet.logoUrl" :size="28" /><strong>{{ wallet.symbol }}</strong></header>
    <dl class="spot-holding-metrics">
      <div><dt>{{ t('orders.currencyEquity') }}</dt><dd class="numeric" :title="walletTotal(wallet)">{{ amount(walletTotal(wallet)) }}</dd><small class="numeric">≈ {{ amount(value, 'USDT') }} USDT</small></div>
      <div><dt>{{ t('orders.costPrice') }}</dt><dd>—</dd></div>
      <div><dt>{{ t('orders.latestPrice') }}</dt><dd class="numeric" :title="price ?? undefined">{{ priceText }}</dd><small>USDT</small></div>
    </dl>
    <dl class="spot-holding-balances">
      <div><dt>{{ t('trade.available') }}</dt><dd class="numeric" :title="wallet.availableText">{{ amount(wallet.availableText) }}</dd></div>
      <div><dt>{{ t('trade.frozenLocked') }}</dt><dd class="numeric" :title="walletFrozen(wallet)">{{ amount(walletFrozen(wallet)) }}</dd></div>
    </dl>
  </article>
</template>
<style scoped>
.spot-holding-card { min-width: 0; padding: 16px 0 20px; border-bottom: 1px solid var(--line); }
.spot-holding-card header { display: flex; gap: 10px; align-items: center; font-size: 17px; margin-bottom: 20px; }
.spot-holding-card dl { display: grid; gap: 12px; margin: 0; }
.spot-holding-metrics { grid-template-columns: repeat(3, minmax(0, 1fr)); }
.spot-holding-metrics > div:last-child { text-align: right; }
.spot-holding-card .spot-holding-balances { grid-template-columns: repeat(2, minmax(0, 1fr)); margin-top: 20px; }
.spot-holding-card dt, .spot-holding-card small { color: var(--muted); font-size: 11px; line-height: 1.5; }
.spot-holding-card dt small { display: block; }
.spot-holding-card dd { margin: 5px 0 3px; font-size: 15px; font-weight: 600; line-height: 1.5; overflow-wrap: anywhere; }
.spot-holding-card div { min-width: 0; }
.spot-holding-card small { overflow-wrap: anywhere; }
</style>
