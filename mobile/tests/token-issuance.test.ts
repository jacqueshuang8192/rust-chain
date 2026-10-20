import assert from 'node:assert/strict'
import test from 'node:test'
import { parseTokenPurchase, parseTokenSubscriptionReceipt, parseTokenPolicy, parseTokenQuote, parseTokenReceipt, tokenReturn, tokenMultiply } from '../src/core/tokenIssuance.ts'
const policy={version:2,timezone:'Asia/Hong_Kong',purchase_initial_price:'0.3',listing_reference_price:'9.99',subscription:{at:1800000000000,automatic:true},distribution:{at:1800000010000,automatic:false},purchase:{at:1800000020000,automatic:true},listing:{at:1800000030000,automatic:false},lock:{kind:'on_listing'},fee:{kind:'profit',rate:'0.2'}}
const intent={side:'buy' as const,quantity:'10',idempotency_key:'unique'}
const quote={id:'quote-1',project_id:7,side:'buy',holding_id:null,quantity:'10.000',price:'0.4',gross:'4',cost:'0',fee:'0',net:'4',observed_at:1800000000000,expires_at:1800000015000,status:'quoted',receipt_json:null}
test('live issuance preserves phase instants, exact math and meaningful unavailable returns',()=>{
 assert.equal(parseTokenPolicy(policy).listing_reference_price,'9.99')
 assert.equal(tokenMultiply('5000','0.2'),'1000')
 assert.equal(tokenMultiply('0.000000000000000001','100'),'0.0000000000000001')
 assert.equal(tokenReturn('500','500'),'100.00%')
 assert.equal(tokenReturn('-50','100'),'-50.00%')
 assert.equal(tokenReturn(null,'100'),null)
 assert.equal(tokenReturn('100','0'),null)
 assert.throws(()=>parseTokenPolicy({...policy,version:3}))
 assert.throws(()=>parseTokenPolicy({...policy,fee:{kind:'unknown'}}))
 assert.throws(()=>parseTokenPolicy({...policy,purchase:{...policy.purchase,at:policy.subscription.at}}))
})
test('quote and final receipt bind project, side, quantity and quote identity',()=>{
 const q=parseTokenQuote(quote,7,intent)
 assert.equal(q.price,'0.4')
 assert.throws(()=>parseTokenQuote({...quote,project_id:8},7,intent))
 assert.throws(()=>parseTokenQuote({...quote,quantity:'11'},7,intent))
 assert.throws(()=>parseTokenQuote({...quote,price:0.4},7,intent))
 assert.throws(()=>parseTokenQuote({...quote,side:'sell',holding_id:2},7,{side:'sell',holding_id:3,idempotency_key:'sell'}))
 const receipt={quote_id:q.id,project_id:7,side:'buy',quantity:'10.000',price:'0.4',gross:'4',fee:'0',net:'4',holding_id:3,executed_at:1800000001000}
 assert.equal(parseTokenReceipt(receipt,q).holding_id,3)
 assert.throws(()=>parseTokenReceipt({...receipt,net:'5'},q))
 assert.throws(()=>parseTokenReceipt({...receipt,quote_id:'other'},q))
})

test('subscription accepts the actual compact receipt and rejects another intent', () => {
 const receipt={ idempotency_key:'subscription-key', status:'pending', lock_position_id:null }
 assert.equal(parseTokenSubscriptionReceipt(receipt,'subscription-key').status,'pending')
 assert.throws(()=>parseTokenSubscriptionReceipt(receipt,'other-key'))
 assert.throws(()=>parseTokenSubscriptionReceipt({...receipt,status:'unknown'},'subscription-key'))
 assert.throws(()=>parseTokenSubscriptionReceipt({...receipt,lock_position_id:5},'subscription-key'))
})

test('direct purchase binds the original key, project and exact quantity without a client price', () => {
 const intent={quantity:'10',idempotency_key:'direct'}
 const receipt={quote_id:'trade',project_id:7,side:'buy',quantity:'10.000',price:'0.4',gross:'4',fee:'0',net:'4',holding_id:3,executed_at:1800000001000}
 const result={idempotency_key:'direct',receipt}
 assert.equal(parseTokenPurchase(result,7,intent).price,'0.4')
 assert.throws(()=>parseTokenPurchase({...result,idempotency_key:'other'},7,intent))
 for(const patch of [{project_id:8},{quantity:'11'},{side:'sell'},{price:0.4},{price:'0'},{fee:'1'},{net:'3'}]) assert.throws(()=>parseTokenPurchase({...result,receipt:{...receipt,...patch}},7,intent))
})
