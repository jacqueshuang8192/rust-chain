<template>
  <div class="space-y-6">
    <h2 class="text-2xl font-bold flex items-center gap-2">
      <Icon icon="mdi:arrow-down-circle-outline" class="text-primary" />
      {{ t('wallet.deposit') }}
    </h2>

    <div class="bg-card border border-border rounded-xl shadow-sm overflow-hidden">
      <div class="flex gap-1 border-b border-border px-3 pt-3 overflow-x-auto">
        <button
          v-for="tab in rechargeTabs"
          :key="tab.value"
          @click="activeRechargeTab = tab.value"
          class="px-4 py-3 text-sm font-medium border-b-2 transition-colors whitespace-nowrap"
          :class="activeRechargeTab === tab.value ? 'border-primary text-primary' : 'border-transparent text-muted-foreground hover:text-foreground'"
        >
          {{ t(tab.labelKey) }}
        </button>
      </div>

      <!-- Deposit Form -->
      <div v-if="activeRechargeTab === 'normal'" class="p-6">
        <TimedDepositPanel v-if="userStore.isLoggedIn" :identity="depositIdentity" />
      </div>

      <div v-else class="p-6 space-y-5">
        <div class="flex items-center justify-between gap-3">
          <h3 class="font-bold">{{ t('wallet.quick_deposit') }}</h3>
          <span v-if="quickConfig?.enabled" class="text-xs px-2 py-1 rounded bg-primary/10 text-primary">
            {{ quickConfig.token.toUpperCase() }} · {{ quickConfig.network }}
          </span>
        </div>

        <div v-if="quickConfig?.enabled" class="space-y-4">
          <label class="block text-sm font-medium text-muted-foreground">
            {{ t('wallet.amount') }} ({{ quickConfig.currency.toUpperCase() }})
            <input
              v-model="quickAmount"
              type="number"
              min="0"
              step="0.01"
              class="mt-2 w-full px-3 py-2 rounded border border-border bg-background text-foreground outline-none focus:border-primary"
            />
          </label>
          <div class="flex flex-wrap gap-2 text-xs text-muted-foreground">
            <span>{{ t('wallet.min') }} {{ formatQuickAmount(quickConfig.min_amount) }} {{ quickConfig.currency.toUpperCase() }}</span>
            <span v-if="quickConfig.max_amount">{{ t('wallet.max') }} {{ formatQuickAmount(quickConfig.max_amount) }} {{ quickConfig.currency.toUpperCase() }}</span>
          </div>
          <button
            @click="submitQuickRecharge"
            :disabled="quickRechargeLoading || !quickAmount"
            class="w-full py-2.5 rounded bg-primary text-primary-foreground font-medium hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <span v-if="quickRechargeLoading" class="inline-flex items-center gap-2">
              <Icon icon="mdi:loading" class="animate-spin" />
              {{ t('wallet.creating_order') }}
            </span>
            <span v-else>{{ t('wallet.create_payment') }}</span>
          </button>

          <div v-if="quickOrder" class="p-4 rounded border border-border bg-muted/20 space-y-3">
            <div class="text-xs text-muted-foreground">{{ t('wallet.order') }} {{ quickOrder.order_id }}</div>
            <div class="grid grid-cols-2 gap-3 text-sm">
              <div>
                <div class="text-muted-foreground text-xs">{{ t('wallet.pay') }}</div>
                <div class="font-medium">{{ formatQuickAmount(quickOrder.fiat_amount) }} {{ quickOrder.currency.toUpperCase() }}</div>
              </div>
              <div>
                <div class="text-muted-foreground text-xs">{{ t('wallet.receive') }}</div>
                <div class="font-medium">{{ quickOrder.actual_amount ? formatQuickAmount(quickOrder.actual_amount) : '-' }} {{ quickOrder.token.toUpperCase() }}</div>
              </div>
            </div>
            <button
              v-if="quickOrder.payment_url"
              @click="openQuickPayment"
              class="w-full py-2 rounded border border-primary/40 text-primary hover:bg-primary/10 transition-colors"
            >
              {{ t('wallet.open_payment') }}
            </button>
          </div>
        </div>

        <div v-else class="text-sm text-muted-foreground">
          {{ t('wallet.quick_unavailable') }}
        </div>
      </div>
    </div>

    <div v-if="quickConfig?.enabled" class="bg-card border border-border rounded-xl p-6 shadow-sm">
      <div class="flex items-center justify-between mb-4">
        <h3 class="font-bold">{{ t('wallet.quick_orders_title') }}</h3>
        <button class="text-xs text-primary font-bold hover:underline" :disabled="quickOrdersLoading" @click="loadQuickOrders">
          {{ t('wallet.records_refresh') }}
        </button>
      </div>
      <div v-if="quickOrdersLoading" class="py-6 text-center text-sm text-muted-foreground">{{ t('common.loading') }}</div>
      <div v-else-if="quickOrders.length === 0" class="py-6 text-center text-sm text-muted-foreground">{{ t('wallet.quick_orders_empty') }}</div>
      <div v-else class="space-y-2">
        <div v-for="order in quickOrders" :key="order.id" class="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border bg-muted/20 px-4 py-3 text-sm">
          <span class="font-mono text-xs text-muted-foreground">{{ order.order_id }}</span>
          <span class="font-mono">{{ formatQuickAmount(order.fiat_amount) }} {{ order.currency.toUpperCase() }}</span>
          <span class="font-mono text-muted-foreground">{{ order.actual_amount ? formatQuickAmount(order.actual_amount) : '-' }} {{ order.token.toUpperCase() }}</span>
          <span class="rounded-full px-3 py-1 text-xs font-bold" :class="quickOrderStatusClass(order.status)">{{ quickOrderStatusText(order.status) }}</span>
          <span class="text-xs text-muted-foreground">{{ formatQuickOrderTime(order.created_at) }}</span>
        </div>
      </div>
    </div>

    <div class="bg-card border border-border rounded-xl p-6 shadow-sm">
      <h3 class="font-bold mb-4">{{ t('wallet.tips') }}</h3>
      <ul class="list-disc pl-5 space-y-2 text-sm text-muted-foreground">
          <li>{{ t('wallet.tip_confirmations') }}</li>
          <li>{{ t('wallet.tip_security') }}</li>
          <li>{{ t('wallet.tip_address') }}</li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import TimedDepositPanel from '@/components/TimedDepositPanel.vue'
import { useUserStore } from '@/stores/user'
import { readAuthSessionScope } from '@/utils/authStorage'
import { Icon } from '@iconify/vue'
import {
    createQuickRechargeOrder,
    fetchQuickRechargeConfig,
    fetchQuickRechargeOrders,
    type QuickRechargeConfig,
    type QuickRechargeOrder,
    type QuickRechargeReturnTarget,
} from '@/api/wallet'
import { useToast } from 'vue-toastification'
import { useI18n } from 'vue-i18n'

const toast = useToast()
const { t } = useI18n()
const userStore = useUserStore()
const depositIdentity = computed(() => {
    if (!userStore.token) return ''
    try { return readAuthSessionScope() }
    catch { return '' }
})
const quickConfig = ref<QuickRechargeConfig | null>(null)
const quickAmount = ref('')
const quickOrder = ref<QuickRechargeOrder | null>(null)

const quickRechargeLoading = ref(false)

type QuickRechargeBridgeWindow = Window & {
    __TAURI__?: unknown
    __TAURI_INTERNALS__?: unknown
    Capacitor?: { getPlatform?: () => string }
    cordova?: unknown
}
type RechargeTab = 'normal' | 'quick'

const activeRechargeTab = ref<RechargeTab>('normal')
const rechargeTabs: Array<{ value: RechargeTab; labelKey: string }> = [
    { value: 'normal', labelKey: 'wallet.normal_deposit' },
    { value: 'quick', labelKey: 'wallet.quick_deposit' },
]

const loadQuickRechargeConfig = async () => {
    try {
        const res = await fetchQuickRechargeConfig()
        if (res.data.code === 0) {
            quickConfig.value = res.data.data
        }
    } catch (e) {
        console.error(e)
    }
}

const submitQuickRecharge = async () => {
    const amount = Number(quickAmount.value)
    if (!Number.isFinite(amount) || amount <= 0) {
        toast.error(t('wallet.invalid_amount'))
        return
    }
    quickRechargeLoading.value = true
    try {
        const res = await createQuickRechargeOrder(quickAmount.value, detectQuickRechargeReturnTarget())
        if (res.data.code === 0) {
            quickOrder.value = res.data.data
            toast.success(t('wallet.payment_created'))
            void loadQuickOrders()
            openQuickPayment()
        }
    } catch (e) {
        console.error(e)
        toast.error(t('wallet.payment_failed'))
    } finally {
        quickRechargeLoading.value = false
    }
}

const detectQuickRechargeReturnTarget = (): QuickRechargeReturnTarget => {
    const bridgeWindow = window as QuickRechargeBridgeWindow
    const userAgent = navigator.userAgent.toLowerCase()
    const capacitorPlatform = bridgeWindow.Capacitor?.getPlatform?.()
    if (capacitorPlatform === 'ios') return 'ios_app'
    if (capacitorPlatform === 'android') return 'android_app'
    if (bridgeWindow.cordova && /iphone|ipad|ipod/.test(userAgent)) return 'ios_app'
    if (bridgeWindow.cordova && /android/.test(userAgent)) return 'android_app'
    if (bridgeWindow.__TAURI__ || bridgeWindow.__TAURI_INTERNALS__) {
        return /macintosh|mac os x/.test(userAgent) ? 'mac_app' : 'pc_app'
    }
    if (/android|iphone|ipad|ipod|mobile/.test(userAgent)) return 'mobile_web'
    return 'desktop_web'
}

const openQuickPayment = () => {
    const paymentUrl = quickOrder.value?.payment_url
    if (paymentUrl) {
        const opened = window.open(paymentUrl, '_blank', 'noopener,noreferrer')
        if (!opened) {
            window.location.assign(paymentUrl)
        }
    }
}

const formatQuickAmount = (value: string | number | null | undefined) => {
    const amount = Number(value)
    if (!Number.isFinite(amount)) return '-'
    return new Intl.NumberFormat(undefined, { maximumFractionDigits: 8 }).format(amount)
}

const quickOrders = ref<QuickRechargeOrder[]>([])
const quickOrdersLoading = ref(false)

async function loadQuickOrders() {
    quickOrdersLoading.value = true
    try {
        const response = await fetchQuickRechargeOrders()
        quickOrders.value = response.data.data
    } catch {
        quickOrders.value = []
    } finally {
        quickOrdersLoading.value = false
    }
}

function quickOrderStatusText(status: string) {
    const key = `wallet.quick_status_${status}`
    const label = t(key)
    return label === key ? status : label
}

function quickOrderStatusClass(status: string) {
    if (status === 'paid' || status === 'credited') return 'bg-green-500/10 text-green-400'
    if (status === 'expired' || status === 'cancelled' || status === 'failed') return 'bg-rose-500/10 text-rose-400'
    return 'bg-blue-500/10 text-blue-400'
}

function formatQuickOrderTime(value: number) {
    if (!value) return '--'
    return new Date(value).toLocaleString()
}

onMounted(() => {
    loadQuickRechargeConfig()
    void loadQuickOrders()
})

</script>

<style scoped>
.animate-fade-in {
    animation: fadeIn 0.3s ease-in-out;
}
@keyframes fadeIn {
    from { opacity: 0; transform: translateY(10px); }
    to { opacity: 1; transform: translateY(0); }
}
</style>
