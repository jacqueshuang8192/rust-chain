import assert from 'node:assert/strict'
import test from 'node:test'
import {parseIssuanceValuation, issuancePrice} from '../src/core/walletValuation.ts'
test('issuance valuation keeps exact prices separate from spot visibility and expires without fallback',()=>{
 const v=parseIssuanceValuation({price_usdt:'0.000000000000000001',expires_at:2000})
 assert.equal(issuancePrice(v,1999),'0.000000000000000001')
 assert.equal(issuancePrice(v,2000),null)
 assert.equal(issuancePrice(undefined,2000),undefined)
 for(const price of [null,0.4,'-1','0','NaN','1e2']) assert.equal(issuancePrice(parseIssuanceValuation({price_usdt:price,expires_at:2000}),1000),null)
 assert.equal(issuancePrice(parseIssuanceValuation({price_usdt:'1',expires_at:'2000'}),1000),null)
})
