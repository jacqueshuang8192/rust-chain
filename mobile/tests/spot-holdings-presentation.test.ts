import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import ts from 'typescript'
import { computed, ref, reactive } from 'vue'
import { walletTotal, walletFrozen } from '../src/core/walletAmounts.ts'
import { formatFinancialAmount } from '../src/core/financialDisplay.ts'
import { estimateWalletAsset } from '../src/core/walletValuation.ts'
import { normalizeDecimalText as n } from '../src/core/decimal.ts'
import { hasPositiveTradeBalance } from '../src/core/tradeFinancial.ts'

const wallet = (symbol: string) => ({assetId:1,symbol,available:2,frozen:1,locked:1,availableText:n('2'),frozenText:n('1'),lockedText:n('1')})
test('spot holdings excludes USDT without removing it from the order wallet collection', () => {
  const source=readFileSync(new URL('../src/views/TradeView.vue',import.meta.url),'utf8')
  const code=source.slice(source.indexOf('const spotVisibleBalances ='),source.indexOf('const selectedProduct ='))
  const wallets=ref([wallet('USDT'),wallet('BTC'),wallet('ETH')])
  const run=new Function('computed','spotWallets','baseAsset','quoteAsset','hasBalance',`${code};return spotVisibleBalances`)
  const rows=run(computed,wallets,ref('BTC'),ref('USDT'),hasPositiveTradeBalance)
  assert.deepEqual(rows.value.map((r: {symbol:string})=>r.symbol),['BTC'])
  assert.equal(wallets.value[0]!.symbol,'USDT')
})
test('holding values use exact totals and authoritative valuation; expiry clears price and unmount clears timer', () => {
  const source=readFileSync(new URL('../src/components/SpotHoldingCard.vue',import.meta.url),'utf8')
  const body=source.split('<script setup lang="ts">')[1]!.split('</script>')[0]!.replace(/^import .*$/gm,'')
  const props=reactive({wallet:{...wallet('BTC'),issuanceValuation:{priceUsdt:'10',expiresAt:2000}}})
  let tick=()=>{}, stop=()=>{}, cleared=false, now=1000
  const bindings={computed,ref,defineProps:()=>props,useI18n:()=>({t:(x:string)=>x,locale:ref('en')}),useMarketStore:()=>({tickerFor:()=>({lastPriceText:n('999')})}),walletTotal,walletFrozen,formatFinancialAmount,estimateWalletAsset,normalizeDecimalText:n,
    Date:{now:()=>now},setInterval:(fn:()=>void)=>{tick=fn;return 1},clearInterval:()=>{cleared=true},onBeforeUnmount:(fn:()=>void)=>{stop=fn}}
  const js=ts.transpileModule(`${body};return {value,price,priceText}`,{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText
  const state=new Function(...Object.keys(bindings),js)(...Object.values(bindings))
  assert.equal(state.value.value,'40')
  assert.equal(state.price.value,'10')
  now=2001;tick()
  assert.equal(state.price.value,null)
  assert.equal(state.value.value,null)
  assert.equal(state.priceText.value,'—')
  props.wallet.issuanceValuation={priceUsdt:'20',expiresAt:3000}
  assert.equal(state.value.value,'80')
  stop();assert.equal(cleared,true)
})
