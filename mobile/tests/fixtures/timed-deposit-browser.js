// Local browser fixture: real component and API adapter, no external API traffic.
import { createApp, h, ref } from 'vue'
import { History } from 'lucide-vue-next'
import { createRouter, createMemoryHistory } from 'vue-router'
import PageHeader from '../../src/components/PageHeader.vue'
import TimedDepositPanel from '../../src/components/TimedDepositPanel.vue'
import { i18n } from '../../src/i18n'
import { client, persistAuthTokens } from '../../src/api/client'
import '../../src/styles/base.css'
import '../../src/styles/prototype-parity.css'
import '../../src/styles/pencil-selected-pages.css'
import '../../src/styles/token-issuance.css'
i18n.global.locale.value = 'zh-CN'
persistAuthTokens('local-fixture-access', 'local-fixture-refresh')
const option = { asset_symbol: 'ETH', network: 'eth', display_name: 'Ethereum', precision_scale: 18, min_deposit_amount: '0.001', deposit_fee: '0', required_confirmations: 12 }
const fixture = { locale(value) { i18n.global.locale.value = value }, orders: [], posts: [], expire() { this.orders[0].expires_at = Date.now() - 1000; this.orders[0].status = 'expired' }, review() { this.orders[0].status = 'review' } }
window.depositFixture = fixture
client.defaults.adapter = async (config) => {
  const path = new URL(config.url, location.origin).pathname
  let data
  if (path.endsWith('/deposit-assets')) data = { assets: [{ symbol: 'USDT', name: 'Tether', logo_url: '/deposit-icons/usdt.png', min_deposit_amount: '0.001', deposit_fee: '0' }] }
  else if (path.endsWith('/deposit-options')) data = [option, { ...option, network: 'base', display_name: 'Base' }, { ...option, asset_symbol: 'SOL', network: 'solana', display_name: 'Solana' }, { ...option, asset_symbol: 'BTC', network: 'btc', display_name: 'Bitcoin' }, { ...option, asset_symbol: 'USDC', precision_scale: 6 }, { ...option, asset_symbol: 'USDT', precision_scale: 6 }, { ...option, asset_symbol: 'USDT', network: 'tron', display_name: 'Tron (TRC20)', precision_scale: 6 }]
  else if (config.method === 'post' && path.endsWith('/deposit-orders')) {
    const body = JSON.parse(config.data); fixture.posts.push(body)
    const now = Date.now()
    data = { ...option, asset_symbol: body.asset_symbol, network: body.network, id: 1, user_id: 1, order_no: 'CD-local-9007199254740993', address: '0x1111111111111111111111111111111111111111', amount: body.amount, fee_amount: '0', paid_amount: null, status: 'pending', tx_hash: null, created_at: now, expires_at: now + 1800000, paid_at: null, cancelled_at: null, server_time: now }
    fixture.orders = [data]
  } else if (config.method === 'post' && path.endsWith('/deposit-orders/1/cancel')) {
    fixture.orders[0].status = 'cancelled'; fixture.orders[0].cancelled_at = Date.now()
    data = { ...fixture.orders[0], server_time: Date.now() }
  } else if (path.endsWith('/deposit-orders')) data = { items: fixture.orders.map(o => ({ ...o, server_time: Date.now() })), total: fixture.orders.length }
  else if (path.endsWith('/deposit-orders/1')) data = { ...fixture.orders[0], server_time: Date.now() }
  else throw new Error('Unexpected local fixture request: ' + path)
  return { data, status: 200, statusText: 'OK', headers: {}, config }
}
const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/', component: {} }, { path: '/assets', name: 'assets', component: {} }, { path: '/assets/deposit/orders', name: 'deposit-orders', component: {} }] })
const depositPanel = ref(null)
createApp({ render: () => h('main', { class: 'page pencil-page wallet-pencil-page', style: 'max-width:448px;margin:auto' }, [
  h(PageHeader, { title: i18n.global.t('timedDeposit.title'), back: true, pencil: true, fallback: { name: 'assets' } }, { actions: () => h('button', { class: 'icon-button', type: 'button', 'aria-label': i18n.global.t('timedDeposit.history'), disabled: depositPanel.value?.busy, onClick: () => router.push({ name: 'deposit-orders' }) }, [h(History, { size: 22 })]) }),
  h('div', { style: 'padding:6px 20px 20px' }, [h(TimedDepositPanel, { ref: depositPanel, identity: 'fixture-user', asset: 'ETH', network: 'eth' })]),
]) }).use(i18n).use(router).mount('#app')
