// Local browser fixture: real component and API adapter, no external API traffic.
import { createApp, h } from 'vue'
import Recharge from '../../src/views/User/Recharge.vue'
import { createPinia } from 'pinia'
import { useUserStore } from '../../src/stores/user'
import i18n from '../../src/i18n'
import request from '../../src/api/request'
import Toast from 'vue-toastification'
const client = request.instance
import '../../src/style.css'
i18n.global.locale.value = 'zh'
const pinia = createPinia()
useUserStore(pinia).setAuthSession({ token: 'opaque-local-fixture-access', refreshToken: 'local-fixture-refresh' })
window.fetch = async () => Response.json({prefix:'mdi',icons:{}})
const option = { asset_symbol: 'ETH', network: 'eth', display_name: 'Ethereum', precision_scale: 18, min_deposit_amount: '0.001', deposit_fee: '0', required_confirmations: 12 }
const fixture = { orders: [], posts: [], expire() { this.orders[0].expires_at = Date.now() - 1000; this.orders[0].status = 'expired' }, review() { this.orders[0].status = 'review' } }
window.depositFixture = fixture
client.defaults.adapter = async (config) => {
  const path = new URL(config.url, location.origin).pathname
  let data
  if (path.endsWith('/quick-recharge/config')) data = {enabled:false}
  else if (path.endsWith('/quick-recharge/orders')) data = {orders:[]}
  else if (path.endsWith('/deposit-options')) data = [option]
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
createApp({ render: () => h('main', { style: 'padding:32px;max-width:900px;margin:auto;color:#14231d;background:white;min-height:100vh' }, [h(Recharge)]) }).use(pinia).use(i18n).use(Toast).mount('#app')
