import assert from 'node:assert/strict'
import test from 'node:test'
import {parseIssuanceValuation, issuancePrice, estimateWalletAsset} from '../src/core/walletValuation.ts'
test('issuance valuation keeps exact prices separate from spot visibility and expires without fallback',()=>{
 const v=parseIssuanceValuation({price_usdt:'0.000000000000000001',expires_at:2000})
 assert.equal(issuancePrice(v,1999),'0.000000000000000001')
 assert.equal(issuancePrice(v,2000),null)
 assert.equal(issuancePrice(undefined,2000),undefined)
 for(const price of [null,0.4,'-1','0','NaN','1e2']) assert.equal(issuancePrice(parseIssuanceValuation({price_usdt:price,expires_at:2000}),1000),null)
 assert.equal(issuancePrice(parseIssuanceValuation({price_usdt:'1',expires_at:'2000'}),1000),null)
})

test('pre-listing totals value all booked buckets once and never add pending token quantities',async()=>{
 const {normalizeDecimalText:n,decimalAdd}=await import('../src/core/decimal.ts')
 const {walletTotal}=await import('../src/core/walletAmounts.ts')
 const token={assetId:7,symbol:'KHB',available:0,frozen:0,locked:2500,availableText:n('0'),frozenText:n('0'),lockedText:n('2500'),issuanceValuation:parseIssuanceValuation({price_usdt:'0.2',expires_at:2000})}
 const quote={assetId:1,symbol:'USDT',available:1000,frozen:100,locked:0,availableText:n('1000'),frozenText:n('100'),lockedText:n('0')}
 const tokenValue=estimateWalletAsset('KHB',walletTotal(token),token,undefined,1000)
 assert.equal(tokenValue,'500')
 assert.equal(decimalAdd(tokenValue!,estimateWalletAsset('USDT',walletTotal(quote),quote,undefined,1000)!),'1600')
 token.issuanceValuation=parseIssuanceValuation({price_usdt:'0.4',expires_at:2000})
 assert.equal(estimateWalletAsset('KHB',walletTotal(token),token,undefined,1000),'1000')
 assert.equal(estimateWalletAsset('KHB',walletTotal(token),token,n('999'),2000),null)
 assert.equal(estimateWalletAsset('BTC',n('2'),undefined,n('50000'),1000),'100000')
 assert.equal(estimateWalletAsset('USDC',n('1'),undefined,undefined,1000),null)
})
