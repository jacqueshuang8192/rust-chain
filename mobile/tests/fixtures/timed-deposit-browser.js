// Local browser fixture: real component and API adapter, no external API traffic.
import { createApp, h } from 'vue'
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
const fixture = { orders: [], posts: [], expire() { this.orders[0].expires_at = Date.now() - 1000; this.orders[0].status = 'expired' }, review() { this.orders[0].status = 'review' } }
window.depositFixture = fixture
client.defaults.adapter = async (config) => {
  const path = new URL(config.url, location.origin).pathname
  let data
  if (path.endsWith('/deposit-options')) data = [option]
  else if (config.method === 'post' && path.endsWith('/deposit-orders')) {
    const body = JSON.parse(config.data); fixture.posts.push(body)
    const now = Date.now()
    data = { ...option, id: 1, user_id: 1, order_no: 'CD-local-9007199254740993', address: '0x1111111111111111111111111111111111111111', amount: body.amount, fee_amount: '0', paid_amount: null, status: 'pending', tx_hash: null, created_at: now, expires_at: now + 1800000, paid_at: null, server_time: now }
    fixture.orders = [data]
  } else if (path.endsWith('/deposit-orders')) data = { items: fixture.orders.map(o => ({ ...o, server_time: Date.now() })), total: fixture.orders.length }
  else if (path.endsWith('/deposit-orders/1')) data = { ...fixture.orders[0], server_time: Date.now() }
  else throw new Error('Unexpected local fixture request: ' + path)
  return { data, status: 200, statusText: 'OK', headers: {}, config }
}
createApp({ render: () => h('main', { class: 'page app-stage theme-light', style: 'padding:20px;max-width:720px;margin:auto' }, [h('h1', { style: 'font-size:24px;margin:0 0 20px' }, '链上充值'), h(TimedDepositPanel, { identity: 'fixture-user', asset: 'ETH', network: 'eth' })]) }).use(i18n).mount('#app')
