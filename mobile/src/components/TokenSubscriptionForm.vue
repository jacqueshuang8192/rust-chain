<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useSessionStore } from '@/stores/session'
import { subscribeToken, type TokenSubscriptionIntent } from '@/api/tokenIssuance'
import { tokenDecimal, tokenMultiply } from '@/core/tokenIssuance'
import { tokenIssuanceMessages } from '@/core/tokenIssuanceMessages'
const props = withDefaults(defineProps<{ projectId: number; symbol: string; quoteSymbol: string; quoteAssetId: number; price: string; subscriptionOpen?: boolean }>(), { subscriptionOpen: true })
const emit = defineEmits<{ completed: [] }>()
const { t } = useI18n({ useScope: 'local', messages: tokenIssuanceMessages })
const session = useSessionStore()
const owner = computed(() => session.isAuthenticated ? session.identityScope : '')
const recoveryBlocked = ref(false)
const quantity = ref(''); const busy = ref(false); const uncertain = ref(false); const error = ref(''); const success = ref(false)
const intent = ref<TokenSubscriptionIntent | null>(null)
let generation = 0
const storageKey = () => `token-subscription:${owner.value}:${props.projectId}`
function save() { sessionStorage.setItem(storageKey(), JSON.stringify({ intent: intent.value, uncertain: uncertain.value })) }
function review() {
  if (recoveryBlocked.value || !props.subscriptionOpen) return
  try {
    const q = tokenDecimal(quantity.value.trim()); if (!/[1-9]/.test(q)) throw new Error()
    const cost = tokenMultiply(q, props.price); tokenDecimal(cost)
    intent.value = { quantity: q, quote_amount: cost, quote_asset_id: props.quoteAssetId, idempotency_key: crypto.randomUUID() }
    uncertain.value = false; save(); error.value = ''
  } catch { intent.value = null; error.value = t('token.failure') }
}
function cancel() {
  if (busy.value || uncertain.value) return
  sessionStorage.removeItem(storageKey()); intent.value = null; error.value = ''
}
async function submit() {
  if (!intent.value || busy.value || !owner.value || (!props.subscriptionOpen && !uncertain.value)) return
  const g = generation
  try { uncertain.value = true; save() } catch { error.value = t('token.unavailableStorage'); return }
  busy.value = true; error.value = ''
  try {
    await subscribeToken(props.symbol, intent.value)
    if (g !== generation) return
    sessionStorage.removeItem(storageKey()); intent.value = null; uncertain.value = false; success.value = true; quantity.value = ''; emit('completed')
  } catch (e) {
    if (g !== generation) return
    const status = (e as { response?: { status?: number } }).response?.status
    if (status && [400, 403, 404, 409, 422].includes(status)) { uncertain.value = false; save() }
    error.value = t(uncertain.value ? 'token.pending' : 'token.failure')
  } finally { if (g === generation) busy.value = false }
}
watch(() => [owner.value, props.projectId], () => {
  generation++; intent.value = null; uncertain.value = false; busy.value = false; quantity.value = ''; error.value = ''; success.value = false; recoveryBlocked.value = false
  if (!owner.value) return
  try { const raw = sessionStorage.getItem(storageKey()); if (raw) { const p = JSON.parse(raw); if (p.intent?.quote_asset_id !== props.quoteAssetId || typeof p.uncertain !== 'boolean' || typeof p.intent?.idempotency_key !== 'string' || !p.intent.idempotency_key) throw new Error(); tokenDecimal(p.intent.quantity); tokenDecimal(p.intent.quote_amount); intent.value = p.intent; uncertain.value = p.uncertain === true } } catch { recoveryBlocked.value = true; error.value = t('token.pending') }
}, { immediate: true })
onBeforeUnmount(() => { generation++ })
</script>
<template>
  <section v-if="owner && (subscriptionOpen || intent || recoveryBlocked || success)" class="token-surface token-subscribe">
    <h2>{{ t('token.subscribe') }} · {{ symbol }}</h2><p>{{ t('token.fixedPrice') }}: {{ price }} {{ quoteSymbol }}</p>
    <p v-if="error" role="alert">{{ error }}</p><p v-if="success" role="status">{{ t('token.subscribed') }}</p>
    <p v-if="!subscriptionOpen && intent && !uncertain">{{ t('token.subscriptionClosed') }}</p>
    <form v-if="!intent && subscriptionOpen" @submit.prevent="review"><label>{{ t('token.quantity') }}<input v-model="quantity" inputmode="decimal" required /></label><button type="submit" :disabled="recoveryBlocked">{{ t('token.review') }}</button></form>
    <div v-if="intent" aria-live="polite"><p>{{ intent.quantity }} {{ symbol }} · {{ t('token.freeze') }}: {{ intent.quote_amount }} {{ quoteSymbol }}</p><p>{{ t('token.subscriptionNote') }}</p><button :disabled="busy || (!subscriptionOpen && !uncertain)" type="button" @click="submit">{{ t(busy ? 'token.processing' : uncertain ? 'token.retry' : 'token.confirm') }}</button><button :disabled="busy || uncertain" type="button" @click="cancel">{{ t('token.cancel') }}</button></div>
  </section>
</template>
