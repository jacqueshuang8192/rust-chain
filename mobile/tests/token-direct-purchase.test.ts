import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import ts from 'typescript'
import * as Vue from 'vue'
import { compileScript, parse } from '@vue/compiler-sfc'
import * as contract from '../src/core/tokenIssuance.ts'

const flush = async () => { for (let i=0;i<12;i++) await Promise.resolve(); await Vue.nextTick() }
function deferred<T>() { let resolve!: (value:T)=>void; let reject!: (error:unknown)=>void; const promise=new Promise<T>((a,b)=>{resolve=a;reject=b});return {promise,resolve,reject} }
function harness(app: 'mobile' | 'pc', storage = new Map<string,string>()) {
 const filename=app==='mobile'?'../src/components/TokenTradingPanel.vue':'../../pc/src/components/TokenTradingPanel.vue'
 const {descriptor}=parse(readFileSync(new URL(filename,import.meta.url),'utf8'))
 const compiled=compileScript(descriptor,{id:'direct-purchase-test'}).content
 const source=ts.transpileModule(compiled,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText
 const session=Vue.reactive({isAuthenticated:true,identityScope:'user:42',token:'.'+btoa(JSON.stringify({sub:'user:42'}))+'.'})
 const props=Vue.reactive({projectId:7,symbol:'KHB',quoteSymbol:'USDT',pairSymbol:'KHBUSDT',stage:'purchase'})
 const writes:Array<{intent:contract.TokenPurchaseIntent}&ReturnType<typeof deferred<contract.TokenReceipt>>>=[]
 const quotes:contract.TokenQuoteIntent[]=[];let unmount=()=>{};let failStorage=false
 const modules:Record<string,unknown>={
  vue:{...Vue,onBeforeUnmount:(fn:()=>void)=>{unmount=fn}},
  'vue-i18n':{useI18n:()=>({t:(key:string)=>key,locale:Vue.ref('en')})},'vue-router':{useRouter:()=>({push(){},currentRoute:{value:{fullPath:'/'}}})},
  '@/stores/session':{useSessionStore:()=>session},'@/stores/user':{useUserStore:()=>session},
  '@/components/SecondsChartWorkspace.vue':{},'@/components/chart/MarketChart.vue':{},
  './TokenQuoteBalance.vue': {}, '@/core/tokenIssuanceMessages':{},'@/core/tokenIssuance':contract,
  '@/api/tokenIssuance':{
   fetchTokenHoldings:async()=>({holdings:[],next:null}),
   purchaseToken:(_symbol:string,_project:number,intent:contract.TokenPurchaseIntent)=>{const write={...deferred<contract.TokenReceipt>(),intent:{...intent}};writes.push(write);return write.promise},
   quoteToken:async(_symbol:string,_project:number,intent:contract.TokenQuoteIntent)=>{quotes.push(intent);throw new Error('quote unavailable')},
   confirmToken:async()=>{throw new Error('not expected')},
  },
 }
 const exports:Record<string,any>={}
 new Function('require','exports','sessionStorage','setInterval','clearInterval',source)(
  (name:string)=>{assert.ok(name in modules,name);return modules[name]},exports,
  {getItem:(k:string)=>storage.get(k),setItem:(k:string,v:string)=>{if(failStorage)throw new Error('storage blocked');storage.set(k,v)},removeItem:(k:string)=>{if(failStorage)throw new Error('storage blocked');storage.delete(k)}},()=>1,()=>{},
 )
 const scope=Vue.effectScope()
 const state=scope.run(()=>exports.default.setup(props,{expose(){},emit(){}})) as {
  quantity:Vue.Ref<string>;pending:Vue.Ref<{mode?:string;intent:contract.TokenQuoteIntent;uncertain:boolean}|null>;
  busy:Vue.Ref<boolean>;notice:Vue.Ref<string>;buy():Promise<void>;executePurchase():Promise<void>;cancel():void;retrieveQuote():Promise<void>
 }
 return {state,props,session,writes,quotes,storage,blockStorage:()=>{failStorage=true},stop:()=>{unmount();scope.stop()}}
}
const receipt:contract.TokenReceipt={quote_id:'trade',project_id:7,side:'buy',quantity:'2',price:'0.4',gross:'0.8',fee:'0',net:'0.8',holding_id:9,executed_at:1800000000000}
for(const app of ['mobile','pc'] as const){
 test(`${app}: one click buys once, original key survives lost response, remount and listing`,async()=>{
  const h=harness(app);await flush();h.state.quantity.value='2'
  const first=h.state.buy();await h.state.buy()
  assert.equal(h.writes.length,1);assert.equal(h.quotes.length,0)
  assert.equal(h.state.pending.value?.uncertain,true)
  h.writes[0]!.reject(new Error('response lost'));await first
  const original=h.writes[0]!.intent;h.state.cancel();assert.ok(h.state.pending.value)
  h.stop();const recovered=harness(app,h.storage);await flush();recovered.props.stage='listed'
  const retry=recovered.state.executePurchase();assert.deepEqual(recovered.writes[0]!.intent,original)
  recovered.writes[0]!.resolve(receipt);await retry
  assert.equal(recovered.state.pending.value,null);assert.equal(recovered.storage.size,0)
  await recovered.state.buy();assert.equal(recovered.writes.length,1);recovered.stop()
 })
 test(`${app}: storage failure sends no order and old account completion cannot affect new account`,async()=>{
  const h=harness(app);await flush();h.state.quantity.value='2';h.blockStorage();await h.state.buy();assert.equal(h.writes.length,0);h.stop()
  const changed=harness(app);await flush();changed.state.quantity.value='2';const pending=changed.state.buy()
  changed.session.identityScope='user:43';changed.session.token='.'+btoa(JSON.stringify({sub:'user:43'}))+'.';await flush()
  changed.writes[0]!.resolve(receipt);await pending
  assert.equal(changed.state.pending.value,null);assert.equal(changed.state.notice.value,'');assert.equal(changed.storage.size,1);changed.stop()
 })
 test(`${app}: definitive rejection clears the attempted buy while unknown failures retain it`,async()=>{
  const h=harness(app);await flush();h.state.quantity.value='2';const pending=h.state.buy()
  h.writes[0]!.reject({response:{status:409}});await pending
  assert.equal(h.state.pending.value,null);assert.equal(h.storage.size,0);h.stop()
 })
}

for (const app of ['mobile','pc'] as const) test(`${app}: legacy pending buy quotes remain recoverable`,async()=>{
 const intent={side:'buy',quantity:'2',idempotency_key:'legacy-buy'}
 const storage=new Map([['token-trade:user:42:7',JSON.stringify({project:7,symbol:'KHB',quoteSymbol:'USDT',intent,uncertain:false})]])
 const h=harness(app,storage);await flush();await h.state.retrieveQuote()
 assert.deepEqual(h.quotes,[intent]);assert.equal(h.writes.length,0);assert.ok(h.state.pending.value);h.stop()
})
