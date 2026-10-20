// Isolated actual-app fixture: no backend requests or real account credentials.
import { client, clearAuthTokens, persistAuthTokens } from '/src/api/client.ts'
import { normalizeMobileLocale, setAppLocale } from '/src/i18n/index.ts'
sessionStorage.setItem('hippo_mobile_launch_intro_v1','seen')
const params = new URLSearchParams(location.search)
const english = params.get('locale') === 'en'
const now = Date.now()
const day = Math.floor(now / 86400000) * 86400000
window.__homeRequests = []
window.WebSocket = class extends EventTarget {
  readyState = 0
  send() {}
  close() { this.readyState = 3 }
}
const total = params.has('long') ? '9007199254740993.000000000000000001' : '1000'
const { tokenMultiply } = await import('/src/core/tokenIssuance.ts')
const { decimalSubtract, normalizeDecimalText } = await import('/src/core/decimal.ts')
window.__coinPreview = { balance: total, price: '0.4', failWallet: false, writes: [] }
const policy = { version:2,timezone:'Asia/Hong_Kong',purchase_initial_price:'0.2',listing_reference_price:'0.2',subscription:{at:now-4000,automatic:true},distribution:{at:now-3000,automatic:false},purchase:{at:now-2000,automatic:true},listing:{at:now+86400000,automatic:false},lock:{kind:'on_listing'},fee:{kind:'none'} }
const project = { id:7,asset_id:2,symbol:'HIPPO',name:'HIPPO',lifecycle_status:params.get('stage')||'purchase',total_supply:'1000000',issue_price:'0.2',quote_asset_id:1,quote_asset_symbol:'USDT',reserved_supply:'1000',allocated_supply:'2500',remaining_supply:'996500',listed_at:null,unlock_type:'immediate_on_listing',unlock_fee_enabled:false,post_listing_purchase_enabled:false,post_listing_pair_id:7,status:'active',issuance_policy:policy }
const holding = {holding:{id:1,project_id:7,quantity:params.has('long')?'9007199254740993.000000000000000001':'2500.000000000000000000',cost:'500.000000000000000000',acquired_at:now-1000,status:'held',sold_at:null,policy_json:policy},symbol:'HIPPO',quote_symbol:'USDT',pair_id:7,pair_symbol:null,lifecycle_status:project.lifecycle_status,eligible:params.has('eligible'),unlock_at:now+86400000,market_price:'0.4',market_value:'1000.000000000000000000',profit:'500.000000000000000000'}
const today = {
  scope: 'realized', reporting_asset: 'USDT', amount: '1204.55', basis_amount: '24800',
  rate: '0.048570564516129032', period_start_at: day, calculated_at: now,
  status: 'complete', missing_price_assets: []
}
const slides = Array.from({ length: Number(params.get('slides') ?? 5) }, (_, position) => ({
  id: `slide-${position}`, position, enabled: true, locale: null,
  title: english ? (position === 0 ? 'Invite friends' : `News ${position + 1}`) : (position === 0 ? '邀请好友' : `资讯 ${position + 1}`),
  description: english ? 'Share your invitation code. View invitation records and campaign rules.' : '分享专属邀请码，邀请好友加入。查看邀请记录，了解活动规则。',
  image_url: new URL('/src/assets/home/market-hero-light.jpg', location.origin).href,
  target: { type: position === 0 ? 'referrals' : 'news_center', id: null }
}))
client.defaults.adapter = async (config) => {
  const path = new URL(config.url, location.origin).pathname.replace(/^.*\/api\/(?:v1|user)/, '')
  window.__homeRequests.push({ path, authorization: config.headers?.get?.('Authorization') ?? null })
  let data = {}
  const state = window.__coinPreview
  if ((config.method || 'get').toLowerCase() !== 'get') {
    state.writes.push({path, body:config.data})
    if (path !== '/new-coins/HIPPO/market-purchases') throw new Error('Preview write blocked')
    const intent=typeof config.data==='string'?JSON.parse(config.data):config.data
    const gross=tokenMultiply(intent.quantity,'0.4')
    state.balance=decimalSubtract(normalizeDecimalText(state.balance),normalizeDecimalText(gross))
    data={idempotency_key:intent.idempotency_key,receipt:{quote_id:'preview-buy',project_id:7,holding_id:2,side:'buy',quantity:intent.quantity,price:'0.4',gross,fee:'0',net:gross,executed_at:Date.now()}}
    return {data,status:200,statusText:'OK',headers:{},config}
  }
  if(path==='/new-coins/HIPPO') data=project
  else if(path==='/new-coins') data={projects:[project]}
  else if(path==='/new-coins/holdings') data={holdings:[holding],next_before_id:null}
  else if(path==='/new-coins/subscriptions') data={subscriptions:[{id:3,project_id:7,quote_asset:1,issue_price:'0.2',quote_amount:'500.000000000000000000',requested_quantity:holding.holding.quantity,allocated_quantity:'2500',settlement_mode:'manual_distribution',frozen_quote_amount:'500',settled_quote_amount:'500',refunded_quote_amount:'0',status:'allocated',idempotency_key:'PREVIEW-SUB-001',created_at:now-5000}]}
  else if(path==='/new-coins/distributions') data={distributions:[]}
  else if(path==='/new-coins/purchases') data={purchases:[]}
  else if(path==='/new-coins/unlocks') data={unlocks:[]}
  else
  if (path === '/home/carousel') {
    if (params.has('error')) throw new Error('Local carousel failure')
    data = { slides }
  } else if (path === '/wallet/accounts') {
    if(state.failWallet) throw new Error('Preview wallet failure')
    data = { accounts: [{ asset_id:1,symbol:'USDT',precision_scale:18,available:state.balance,frozen:'0',locked:'0' },{asset_id:2,symbol:'HIPPO',precision_scale:18,available:'0',frozen:'0',locked:'2500',issuance_valuation:{price_usdt:state.price,expires_at:Date.now()+30000}}] }
  } else if (path === '/margin/wallets') data = { wallets: [], positions: [], cross_accounts: [] }
  else if (path === '/wallet/today-return') data = today
  else if (path === '/wallet/return-history') data = {
    scope: 'realized', reporting_asset: 'USDT', period_days: 1, period_start_at: day,
    calculated_at: now, status: 'complete', missing_prices: [],
    summary: { amount: today.amount, basis_amount: today.basis_amount, rate: today.rate },
    points: [{
      day_start_at: day, valued_at: now, amount: today.amount, basis_amount: today.basis_amount,
      rate: today.rate, cumulative_amount: today.amount, status: 'complete', missing_price_assets: []
    }]
  }
  else if (path === '/markets') data = { markets: [] }
  else if (path === '/market-favorites') data = { favorites: [] }
  else if (path === '/support/conversation') data = { conversation: null }
  return { data, status: 200, statusText: 'OK', headers: {}, config }
}
localStorage.setItem('hippo_mobile_theme', params.get('theme') ?? 'light')
setAppLocale(normalizeMobileLocale(params.get('locale')) ?? 'zh-CN')
if (params.has('member')) persistAuthTokens('local-home-test', 'local-home-refresh')
else clearAuthTokens()
history.replaceState(null, '', `${location.pathname}${location.search}#${params.get('page')==='home'?'/':params.get('page')==='records'?'/products/new-coins/records':'/products/new-coins/HIPPO'}`)
await import('/src/main.ts')
