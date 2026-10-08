// Isolated actual-app fixture: no backend requests or real account credentials.
import { client, clearAuthTokens, persistAuthTokens } from '/src/api/client.ts'
import { normalizeMobileLocale, setAppLocale } from '/src/i18n/index.ts'
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
const total = params.has('long') ? '9007199254740993.01' : '24806.32'
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
  if (path === '/home/carousel') {
    if (params.has('error')) throw new Error('Local carousel failure')
    data = { slides }
  } else if (path === '/wallet/accounts') {
    data = { accounts: [{ asset_id: 1, symbol: 'USDT', precision_scale: 18, available: total, frozen: '0', locked: '0' }] }
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
history.replaceState(null, '', `${location.pathname}${location.search}#/`)
await import('/src/main.ts')
