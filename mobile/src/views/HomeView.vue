<script setup lang="ts">
import '@/styles/home-numeric.css'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import {
  ArrowDownLeft,
  CandlestickChart,
  ChevronRight,
  CircleDollarSign,
  Eye,
  EyeOff,
  Grid2X2,
  Landmark,
  Layers3,
  LockKeyhole,
  LogIn,
  Repeat2,
  Rocket,
  ScanQrCode,
  Search,
  UserPlus,
  Zap,
} from 'lucide-vue-next'
import AssetMark from '@/components/AssetMark.vue'
import guestHeroDark from '@/assets/home/market-hero-dark.jpg'
import guestHeroLight from '@/assets/home/market-hero-light.jpg'
import portfolioIllustration from '@/assets/home/portfolio-wallet.png'
import { fetchHomeCarousel, type HomeCarouselSlide } from '@/api/homeCarousel'
import { fetchMarginWallets } from '@/api/trading'
import {
  createTodayReturnRequestLifecycle,
  fetchTodayReturn,
  fetchWalletAccounts,
  type TodayReturn,
} from '@/api/wallet'
import { formatPercent, formatPrice } from '@/core/format'
import { decimalAdd, decimalRoundHalfUp, decimalSign, normalizeDecimalText, type DecimalText } from '@/core/decimal'
import { formatTokenAmount } from '@/core/tokenIssuance'
import { estimateWalletAsset } from '@/core/walletValuation'
import { walletTotal } from '@/core/walletAmounts'
import { formatFinancialAmount } from '@/core/financialDisplay'
import { resolveTodayReturnPresentation } from '@/core/todayReturnPresentation'
import { useMarketStore } from '@/stores/market'
import { useMarketFavoritesStore } from '@/stores/marketFavorites'
import { useNavigationStore } from '@/stores/navigation'
import { useSessionStore } from '@/stores/session'
import { useThemeStore } from '@/stores/theme'
import type { WalletAccount } from '@/core/types'

const router = useRouter()
const marketStore = useMarketStore()
const marketFavorites = useMarketFavoritesStore()
const navigation = useNavigationStore()
const session = useSessionStore()
const theme = useThemeStore()
const { locale, t } = useI18n()

type HomeTab = 'favorites' | 'mainstream' | 'popular' | 'gainers' | 'newCoins'
type TradeMode = 'spot' | 'contract'

const activeTab = ref<HomeTab>('popular')
const assetVisible = ref(true)
const spotAccounts = ref<WalletAccount[]>([])
const marginAccounts = ref<WalletAccount[]>([])
const assetEstimateReady = ref(false)
const valuationClock = ref(Date.now())
const assetsLoading = ref(false)
const todayReturn = ref<TodayReturn | null>(null)
const todayReturnState = ref<'idle' | 'loading' | 'complete' | 'partial' | 'error'>('idle')
const carouselSlides = ref<HomeCarouselSlide[]>([])
const carouselState = ref<'idle' | 'loading' | 'ready' | 'empty' | 'error'>('idle')
const carouselIndex = ref(0)
const carouselRef = ref<HTMLElement | null>(null)
const carouselImageErrors = ref<Set<string>>(new Set())
const carouselHovering = ref(false)
const carouselFocused = ref(false)
const carouselReducedMotion = ref(false)
const carouselHidden = ref(false)
const carouselTouching = ref(false)
const todayReturnRequestLifecycle = createTodayReturnRequestLifecycle({
  sessionKey: () => session.token,
  fetchTodayReturn,
})
let viewActive = true
let carouselTimer: number | undefined
let carouselTouchStart: { x: number; y: number } | null = null
let carouselSuppressClickUntil = 0
let carouselRequestVersion = 0
let assetRequestVersion = 0
let reducedMotionMediaQuery: MediaQueryList | null = null

const tabs = computed(() => [
  { key: 'favorites' as const, label: t('home.favorites') },
  { key: 'mainstream' as const, label: t('home.mainstream') },
  { key: 'popular' as const, label: t('home.popular') },
  { key: 'gainers' as const, label: t('home.gainers') },
  { key: 'newCoins' as const, label: t('products.newCoins') },
])

const visibleTickers = computed(() => {
  const rows = activeTab.value === 'favorites'
    ? marketStore.tickers.filter((ticker) => marketFavorites.isFavorite(ticker.symbol))
    : [...marketStore.topTickers]
  if (activeTab.value === 'gainers') return rows.sort((left, right) => right.changePercent - left.changePercent)
  if (activeTab.value === 'popular' || activeTab.value === 'mainstream') return rows.sort((left, right) => right.volume - left.volume)
  if (activeTab.value === 'newCoins') return rows.reverse()
  return rows
})
const marketRowsUnavailable = computed(() => marketStore.tickers.length === 0)
const guestHeroImage = computed(() => theme.isDark ? guestHeroDark : guestHeroLight)
const activeCarouselSlide = computed(() => carouselSlides.value[carouselIndex.value] || null)
const carouselHasMultipleSlides = computed(() => carouselSlides.value.length > 1)
const carouselPaused = computed(() => (
  carouselReducedMotion.value
  || carouselHovering.value
  || carouselFocused.value
  || carouselHidden.value
  || carouselTouching.value
))

const assetEstimates = computed(() => [...spotAccounts.value, ...marginAccounts.value].map(account => {
  const amount = walletTotal(account)
  if (decimalSign(amount) === 0) return amount
  const source = spotAccounts.value.find(row => row.assetId === account.assetId) || account
  return estimateWalletAsset(account.symbol, amount, source,
    marketStore.tickerFor(`${account.symbol}/USDT`)?.lastPriceText, valuationClock.value)
}))
const totalAssetEstimate = computed(() => assetEstimates.value.reduce<DecimalText>(
  (total, value) => value === null ? total : decimalAdd(total, value), normalizeDecimalText('0'),
))
const assetEstimateComplete = computed(() => assetEstimates.value.every(value => value !== null))

const displayedAssetAmount = computed(() => {
  if (!session.isAuthenticated || !assetEstimateReady.value || !assetEstimateComplete.value) return '--'
  const rounded = decimalRoundHalfUp(totalAssetEstimate.value, 4)
  return formatTokenAmount(decimalSign(rounded) === 0 ? totalAssetEstimate.value : rounded)
})

const todayReturnPresentation = computed(() => resolveTodayReturnPresentation({
  visible: assetVisible.value,
  state: todayReturnState.value,
  value: todayReturn.value,
  locale: locale.value === 'en' ? 'en-US' : 'zh-CN',
  amountMask: '••••',
  detailMask: '••••',
  messages: {
    loading: t('home.todayReturnLoading'),
    partial: (assets) => t('home.todayReturnPartial', { assets }),
    partialUnknown: t('home.todayReturnPartialUnknown'),
    error: t('home.todayReturnUnavailable'),
  },
}))
const displayedTodayReturnAmount = computed(() => (
  assetVisible.value && todayReturnState.value === 'complete' && todayReturn.value?.status === 'complete'
    ? formatSignedReturnAmount(todayReturn.value.amount)
    : todayReturnPresentation.value.amount
))
const displayedTodayReturnDetail = computed(() => todayReturnPresentation.value.detail)
const todayReturnTone = computed(() => todayReturnPresentation.value.tone)

function formatSignedReturnAmount(value: DecimalText | null | undefined): string {
  const decimal = value ?? normalizeDecimalText('0')
  const formatted = formatFinancialAmount(decimal, locale.value === 'en' ? 'en-US' : 'zh-CN', {
    assetSymbol: 'USDT',
  })
  return `${decimalSign(decimal) > 0 && !formatted.startsWith('<') ? '+' : ''}${formatted}`
}

function openMarket(symbol: string): void {
  void router.push({ name: 'market-detail', params: { symbol: symbol.replace('/', '_') } })
}

function openTrade(mode: TradeMode = 'spot', symbol = 'BTC/USDT'): void {
  const routeSymbol = symbol.replace('/', '_')
  navigation.rememberTradeSymbol(routeSymbol)
  navigation.rememberTradeMode(mode)
  void router.replace({
    name: 'trade',
    params: { symbol: routeSymbol },
    query: mode === 'contract' ? { mode } : undefined,
  })
}

function openLogin(): void {
  void router.push({ name: 'login', query: { redirect: '/' } })
}

function openRegister(): void {
  void router.push({ name: 'register', query: { redirect: '/' } })
}

function selectCarousel(index: number): void {
  if (!carouselSlides.value.length) return
  carouselIndex.value = (index + carouselSlides.value.length) % carouselSlides.value.length
  restartCarouselTimer()
}

function nextCarousel(): void {
  selectCarousel(carouselIndex.value + 1)
}

function previousCarousel(): void {
  selectCarousel(carouselIndex.value - 1)
}

function openCarouselTarget(slide: HomeCarouselSlide): void {
  if (Date.now() < carouselSuppressClickUntil) return
  if (slide.target.type === 'news_detail' && slide.target.id) {
    void router.push({ name: 'news-detail', params: { id: String(slide.target.id) } })
  } else if (slide.target.type === 'news_center') {
    void router.push({ name: 'news' })
  } else if (slide.target.type === 'referrals') {
    void router.push({ name: 'referrals' })
  }
}

function markCarouselImageError(id: string): void {
  carouselImageErrors.value = new Set(carouselImageErrors.value).add(id)
}

function stopCarouselTimer(): void {
  if (carouselTimer !== undefined) {
    window.clearInterval(carouselTimer)
    carouselTimer = undefined
  }
}

function restartCarouselTimer(): void {
  stopCarouselTimer()
  if (!viewActive || carouselPaused.value || !carouselHasMultipleSlides.value) return
  carouselTimer = window.setInterval(nextCarousel, 5_500)
}

function updateCarouselReducedMotion(): void {
  carouselReducedMotion.value = typeof window !== 'undefined'
    && window.matchMedia('(prefers-reduced-motion: reduce)').matches
  restartCarouselTimer()
}

function updateCarouselVisibility(): void {
  carouselHidden.value = document.hidden
  restartCarouselTimer()
}

function onCarouselKeydown(event: KeyboardEvent): void {
  if (!carouselHasMultipleSlides.value) return
  if (event.key === 'ArrowRight') {
    event.preventDefault()
    nextCarousel()
  } else if (event.key === 'ArrowLeft') {
    event.preventDefault()
    previousCarousel()
  }
}

function onCarouselFocusOut(event: FocusEvent): void {
  const nextTarget = event.relatedTarget
  if (!(nextTarget instanceof Node) || !carouselRef.value?.contains(nextTarget)) {
    carouselFocused.value = false
    restartCarouselTimer()
  }
}

function onCarouselTouchStart(event: TouchEvent): void {
  const touch = event.touches.length === 1 ? event.touches[0] : undefined
  carouselTouchStart = touch ? { x: touch.clientX, y: touch.clientY } : null
  carouselTouching.value = Boolean(touch)
  carouselSuppressClickUntil = 0
  restartCarouselTimer()
}

function onCarouselTouchCancel(): void {
  carouselTouchStart = null
  carouselTouching.value = false
  restartCarouselTimer()
}

function onCarouselTouchEnd(event: TouchEvent): void {
  const start = carouselTouchStart
  const touch = event.changedTouches[0]
  onCarouselTouchCancel()
  if (!start || !touch) return
  const dx = touch.clientX - start.x
  const dy = touch.clientY - start.y
  if (Math.max(Math.abs(dx), Math.abs(dy)) > 12) carouselSuppressClickUntil = Date.now() + 500
  if (Math.abs(dx) < 40 || Math.abs(dx) <= Math.abs(dy) || !carouselHasMultipleSlides.value) return
  if (dx < 0) nextCarousel()
  else previousCarousel()
}

async function loadCarousel(): Promise<void> {
  const requestVersion = ++carouselRequestVersion
  stopCarouselTimer()
  carouselSlides.value = []
  carouselIndex.value = 0
  carouselState.value = 'loading'
  try {
    const slides = (await fetchHomeCarousel()).filter((slide) => slide.enabled)
    if (!viewActive || requestVersion !== carouselRequestVersion) return
    carouselSlides.value = slides
    carouselIndex.value = Math.min(carouselIndex.value, Math.max(0, slides.length - 1))
    carouselImageErrors.value = new Set()
    carouselState.value = slides.length ? 'ready' : 'empty'
    restartCarouselTimer()
  } catch {
    if (!viewActive || requestVersion !== carouselRequestVersion) return
    carouselSlides.value = []
    carouselIndex.value = 0
    carouselState.value = 'error'
    stopCarouselTimer()
  }
}

async function loadAssetEstimate(): Promise<void> {
  const requestVersion = ++assetRequestVersion
  const token = session.token
  if (!session.isAuthenticated) return
  assetsLoading.value = true
  try {
    const [nextSpotAccounts, marginState] = await Promise.all([
      fetchWalletAccounts(),
      fetchMarginWallets(),
    ])
    if (!viewActive || requestVersion !== assetRequestVersion || session.token !== token) return
    spotAccounts.value = nextSpotAccounts
    marginAccounts.value = marginState.wallets
    valuationClock.value = Date.now()
    assetEstimateReady.value = true
  } catch {
    if (!viewActive || requestVersion !== assetRequestVersion || session.token !== token) return
    spotAccounts.value = []; marginAccounts.value = []; assetEstimateReady.value = false
  } finally {
    if (requestVersion === assetRequestVersion) assetsLoading.value = false
  }
}

async function loadTodayReturn(): Promise<void> {
  if (!session.token) {
    todayReturn.value = null
    todayReturnState.value = 'idle'
    return
  }

  todayReturn.value = null
  todayReturnState.value = 'loading'
  const result = await todayReturnRequestLifecycle.load()
  if (result.state === 'stale') return
  if (result.state === 'guest') {
    todayReturn.value = null
    todayReturnState.value = 'idle'
    return
  }
  if (result.state === 'loaded') {
    todayReturn.value = result.value
    todayReturnState.value = result.value.status
    return
  }
  todayReturn.value = null
  todayReturnState.value = 'error'
}

async function refreshMarkets(force = false): Promise<void> {
  await marketStore.refresh(force)
  if (viewActive) marketStore.startLiveUpdates('home')
}

onMounted(async () => {
  updateCarouselReducedMotion()
  reducedMotionMediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)')
  reducedMotionMediaQuery.addEventListener('change', updateCarouselReducedMotion)
  updateCarouselVisibility()
  document.addEventListener('visibilitychange', updateCarouselVisibility)
  void loadCarousel()
  await refreshMarkets()
})
onUnmounted(() => {
  viewActive = false
  carouselRequestVersion++
  assetRequestVersion++
  clearInterval(assetRefreshTimer)
  stopCarouselTimer()
  reducedMotionMediaQuery?.removeEventListener('change', updateCarouselReducedMotion)
  document.removeEventListener('visibilitychange', updateCarouselVisibility)
  todayReturnRequestLifecycle.stop()
  marketStore.stopLiveUpdates('home')
})
watch(() => session.token, () => {
  assetRequestVersion++; spotAccounts.value = []; marginAccounts.value = []
  assetEstimateReady.value = false; assetsLoading.value = false
  void loadAssetEstimate()
}, { immediate: true })
const assetRefreshTimer = setInterval(() => {
  valuationClock.value = Date.now()
  if (session.isAuthenticated && !assetsLoading.value && document.visibilityState === 'visible') void loadAssetEstimate()
}, 10_000)
watch(() => session.token, () => {
  todayReturnRequestLifecycle.invalidate()
  void loadTodayReturn()
}, { immediate: true })
watch(locale, () => { void loadCarousel() })
watch([carouselSlides, carouselPaused], restartCarouselTimer)
</script>

<template>
  <main class="view home-view prototype-root-view">
    <section class="home-workspace">
      <div class="home-utility-row">
        <button class="home-search" type="button" @click="router.replace({ name: 'markets' })">
          <Search :size="17" aria-hidden="true" />
          <span>{{ t('home.searchPlaceholder') }}</span>
        </button>
        <div class="home-utility-actions action-cluster" role="group" :aria-label="t('home.scan')">
          <button
            class="utility-icon"
            type="button"
            :aria-label="t('home.scan')"
            @click="router.push({ name: 'deposit-asset' })"
          >
            <ScanQrCode :size="18" aria-hidden="true" />
          </button>
        </div>
      </div>
    </section>

    <section v-if="!session.isAuthenticated" class="home-portfolio home-portfolio--guest">
      <article class="home-guest-hero">
        <img class="home-guest-hero__image" :src="guestHeroImage" alt="">
        <span class="home-guest-hero__overlay" aria-hidden="true" />
        <span class="home-guest-hero__accent" aria-hidden="true">
          <span class="home-guest-hero__trail" />
        </span>
        <div class="home-guest-hero__copy">
          <span class="home-guest-hero__eyebrow">{{ t('home.guestAssetAccount') }} <LockKeyhole :size="13" aria-hidden="true" /></span>
          <strong class="home-guest-hero__title">{{ t('home.guestAssetTitle') }}</strong>
          <p>{{ t('home.guestHeroDescription') }}</p>
        </div>
        <div class="home-guest-hero__actions">
          <button class="home-guest-hero__login" type="button" @click="openLogin">
            <LogIn :size="14" aria-hidden="true" />
            {{ t('home.guestHeroLogin') }}
          </button>
          <button class="home-guest-hero__register" type="button" @click="openRegister">
            <UserPlus :size="14" aria-hidden="true" />
            {{ t('home.guestHeroRegister') }}
          </button>
        </div>
      </article>
    </section>

    <section
      v-else
      class="portfolio-overview home-portfolio home-portfolio--member"
      data-portfolio-source="wallet-valuation"
      :data-long-amount="assetVisible && displayedAssetAmount.length > 14"
      :aria-busy="!assetEstimateReady"
    >
        <div class="portfolio-heading">
          <div>
            <div class="balance-label">
              <span>{{ t('home.totalAssetValue') }}</span>
              <button
                class="inline-icon"
                type="button"
                :aria-label="t('home.assetOverview')"
                :aria-pressed="!assetVisible"
                @click="assetVisible = !assetVisible"
              >
                <Eye v-if="assetVisible" :size="14" aria-hidden="true" />
                <EyeOff v-else :size="14" aria-hidden="true" />
              </button>
            </div>
            <strong class="portfolio-balance numeric">
              <span>{{ assetVisible ? displayedAssetAmount : '••••••' }}</span>
              <small> USDT</small>
            </strong>
          </div>
          <div
            class="portfolio-change"
            :data-today-return-status="todayReturnState"
            :aria-busy="todayReturnState === 'loading'"
            aria-live="polite"
          >
            <span>{{ t('rootPrototype.todayReturn') }}</span>
            <strong class="numeric" :class="todayReturnTone" :aria-label="todayReturnPresentation.amount">{{ displayedTodayReturnAmount }}</strong>
            <small class="numeric" :class="todayReturnTone">{{ displayedTodayReturnDetail }}</small>
          </div>
        </div>
        <img
          class="portfolio-illustration"
          :src="portfolioIllustration"
          alt=""
          aria-hidden="true"
          width="120"
          height="104"
          decoding="async"
        />
    </section>

    <section v-if="session.isAuthenticated" class="funding-actions" :aria-label="t('assets.operations')">
      <button class="buy-crypto" type="button" @click="router.push({ name: 'quick-recharge' })">
        <Zap :size="18" aria-hidden="true" />
        {{ t('home.quickDeposit') }}
      </button>
      <button class="deposit-crypto" type="button" @click="router.push({ name: 'deposit-asset' })">
        <ArrowDownLeft :size="18" aria-hidden="true" />
        {{ t('home.onchainDeposit') }}
      </button>
    </section>

    <section class="shortcut-section" :aria-label="t('home.productCenter')">
      <div class="shortcut-grid">
        <button type="button" @click="router.push({ name: 'swap' })"><span><Repeat2 :size="20" /></span><small>{{ t('trade.swap') }}</small></button>
        <button type="button" @click="openTrade('spot')"><span><CandlestickChart :size="20" /></span><small>{{ t('trade.spot') }}</small></button>
        <button type="button" @click="openTrade('contract')"><span><Layers3 :size="20" /></span><small>{{ t('trade.contract') }}</small></button>
        <button type="button" @click="router.push({ name: 'earn' })"><span><Landmark :size="20" /></span><small>{{ t('products.earn') }}</small></button>
        <button type="button" @click="router.push({ name: 'loan' })"><span><CircleDollarSign :size="20" /></span><small>{{ t('products.loan') }}</small></button>
        <button type="button" @click="router.push({ name: 'new-coins' })"><span><Rocket :size="20" /></span><small>{{ t('home.newCoinsShortcut') }}</small></button>
        <button type="button" data-home-shortcut="seconds" @click="router.push({ name: 'seconds' })"><span><Zap :size="19" /></span><small>{{ t('home.secondsShortcut') }}</small></button>
        <button type="button" @click="router.push({ name: 'products' })"><span><Grid2X2 :size="20" /></span><small>{{ t('common.more') }}</small></button>
      </div>
    </section>

    <section
      v-if="activeCarouselSlide"
      ref="carouselRef"
      class="home-carousel"
      aria-roledescription="carousel"
      :aria-label="t('home.carouselLabel')"
      :aria-busy="carouselState === 'loading'"
      tabindex="0"
      @keydown="onCarouselKeydown"
      @mouseenter="carouselHovering = true; restartCarouselTimer()"
      @mouseleave="carouselHovering = false; restartCarouselTimer()"
      @focusin="carouselFocused = true; restartCarouselTimer()"
      @focusout="onCarouselFocusOut"
    >
      <button
        class="home-carousel__slide"
        type="button"
        :aria-label="`${activeCarouselSlide.title}. ${activeCarouselSlide.description}`"
        @click="openCarouselTarget(activeCarouselSlide)"
        @touchstart.passive="onCarouselTouchStart"
        @touchend.passive="onCarouselTouchEnd"
        @touchcancel.passive="onCarouselTouchCancel"
      >
        <span class="home-carousel__copy">
          <strong><span>{{ activeCarouselSlide.title }}</span><ChevronRight :size="18" aria-hidden="true" /></strong>
          <span class="home-carousel__description">{{ activeCarouselSlide.description }}</span>
        </span>
        <img
          v-if="!carouselImageErrors.has(activeCarouselSlide.id)"
          class="home-carousel__image"
          :src="activeCarouselSlide.imageUrl"
          alt=""
          @error="markCarouselImageError(activeCarouselSlide.id)"
        >
        <span v-else class="home-carousel__image-fallback" aria-hidden="true" />
      </button>
      <div v-if="carouselHasMultipleSlides" class="home-carousel__controls">
        <div class="home-carousel__pagination">
          <span v-for="(slide, index) in carouselSlides" :key="slide.id"
            class="home-carousel__dot" :class="{ active: carouselIndex === index }" aria-hidden="true" />
          <input type="range" min="0" :max="carouselSlides.length - 1" step="1" :value="carouselIndex"
            :aria-label="t('home.carouselPagination')"
            :aria-valuetext="t('home.carouselSelect', { index: carouselIndex + 1 })"
            @keydown.stop @input="selectCarousel(Number(($event.target as HTMLInputElement).value))">
        </div>
      </div>
    </section>
    <div v-else-if="carouselState === 'error'" class="home-carousel-error" role="status">
      <span>{{ t('home.carouselUnavailable') }}</span>
      <button type="button" @click="loadCarousel">{{ t('common.retry') }}</button>
    </div>

    <section class="home-market-section">
      <div class="section-heading">
        <div><h2>{{ t('markets.title') }}</h2></div>
        <button class="text-action" type="button" @click="router.replace({ name: 'markets' })">
          {{ t('common.more') }} <ChevronRight :size="15" />
        </button>
      </div>
      <div class="home-market-tabs" role="tablist" :aria-label="t('home.marketSummary')">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          type="button"
          role="tab"
          :aria-selected="activeTab === tab.key"
          :class="{ active: activeTab === tab.key }"
          @click="activeTab = tab.key"
        >
          {{ tab.label }}
        </button>
      </div>
      <div class="home-market-head" aria-hidden="true">
        <span>{{ t('markets.pair') }}</span>
        <span>{{ t('markets.latestPrice') }}</span>
        <span>{{ t('markets.change24h') }}</span>
      </div>
      <div
        class="home-market-list"
        :class="{ 'root-market-reserved': marketRowsUnavailable }"
        :aria-busy="marketRowsUnavailable && !marketStore.error"
        :aria-label="marketRowsUnavailable ? t(marketStore.error ? 'common.marketLoadFailed' : 'common.loading') : undefined"
      >
        <template v-if="marketRowsUnavailable">
          <div v-for="row in 3" :key="`home-market-skeleton-${row}`" class="home-market-skeleton-row" aria-hidden="true">
            <span class="home-market-name">
              <span class="coin-orbit root-skeleton" />
              <span>
                <i class="root-market-skeleton-block root-skeleton" />
                <i class="root-market-skeleton-block compact root-skeleton" />
              </span>
            </span>
            <span class="home-market-price">
              <i class="root-market-skeleton-block root-skeleton" />
              <i class="root-market-skeleton-block compact root-skeleton" />
            </span>
            <span class="market-change root-skeleton" />
          </div>
          <div v-if="marketStore.error" class="root-market-reserved-state" role="alert">
            <span>{{ t('common.marketLoadFailed') }}</span>
            <button type="button" :disabled="marketStore.loading" @click="refreshMarkets(true)">{{ t('common.retry') }}</button>
          </div>
        </template>

        <template v-else>
          <button
            v-for="ticker in visibleTickers.slice(0, 3)"
            :key="ticker.symbol"
            type="button"
            @click="openMarket(ticker.symbol)"
          >
            <span class="home-market-name">
              <span class="coin-orbit">
                <AssetMark :symbol="ticker.base" :src="ticker.iconUrl" :fallback-src="ticker.baseIconUrl" :size="32" />
              </span>
              <span><strong>{{ ticker.base }}</strong><small>/{{ ticker.quote }}</small></span>
            </span>
            <span class="home-market-price">
              <strong class="numeric">{{ formatPrice(ticker.lastPrice) }}</strong>
              <small>{{ ticker.quote }}</small>
            </span>
            <span
              class="market-change numeric"
              :class="ticker.changePercent >= 0 ? 'positive-bg' : 'negative-bg'"
            >
              {{ formatPercent(ticker.changePercent) }}
            </span>
          </button>

          <div v-if="!visibleTickers.length" class="root-market-empty-state" role="status">
            <div class="root-market-empty-icon" aria-hidden="true">
              <Search :size="22" :stroke-width="1.75" />
            </div>
            <strong>{{ t('markets.noResults') }}</strong>
            <span>{{ t('rootPrototype.noMarketHint') }}</span>
          </div>
        </template>
      </div>
    </section>

  </main>
</template>
