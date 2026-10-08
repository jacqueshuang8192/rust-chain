import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import en from '../src/i18n/messages/en.ts'
import zhCN from '../src/i18n/messages/zh-CN.ts'

const homeSource = readFileSync(new URL('../src/views/HomeView.vue', import.meta.url), 'utf8')

test('访客 Hero 首屏只渲染当前主题图片', () => {
  assert.match(homeSource, /const theme = useThemeStore\(\)/)
  assert.match(homeSource, /const guestHeroImage = computed\(\(\) => theme\.isDark \? guestHeroDark : guestHeroLight\)/)
  assert.equal((homeSource.match(/class="home-guest-hero__image"/g) || []).length, 1)
  assert.doesNotMatch(homeSource, /home-guest-hero__image--(?:light|dark)/)
})

test('首页搜索与快捷入口使用原型短文案且不缩短产品页标签', () => {
  assert.equal(zhCN.home.searchPlaceholder, '搜索币种、产品或功能')
  assert.equal(zhCN.home.newCoinsShortcut, '新币')
  assert.equal(zhCN.home.secondsShortcut, '秒合约')
  assert.equal(zhCN.products.newCoins, '新币认购')
  assert.equal(zhCN.products.prediction, '预测市场')

  assert.equal(en.home.searchPlaceholder, 'Search coins, products, or features')
  assert.equal(en.home.newCoinsShortcut, 'New coins')
  assert.equal(en.home.secondsShortcut, 'Seconds')
  assert.equal(en.products.newCoins, 'New coins')
  assert.equal(en.products.prediction, 'Prediction markets')

  const shortcutSection = homeSource.slice(
    homeSource.indexOf('<section class="shortcut-section"'),
    homeSource.indexOf('</section>', homeSource.indexOf('<section class="shortcut-section"')),
  )
  assert.match(shortcutSection, /t\('home\.newCoinsShortcut'\)/)
  assert.match(shortcutSection, /data-home-shortcut="seconds"/)
  assert.match(shortcutSection, /router\.push\(\{ name: 'seconds' \}\)/)
  assert.match(shortcutSection, /<Zap :size="19"/)
  assert.match(shortcutSection, /t\('home\.secondsShortcut'\)/)
  assert.doesNotMatch(shortcutSection, /name: 'prediction'|<Gauge|predictionShortcut/)
  assert.doesNotMatch(shortcutSection, /t\('products\.(?:newCoins|prediction)'\)/)
})

test('首页轮播使用后台接口并只暴露内部目标', () => {
  assert.equal(typeof zhCN.home.carouselLabel, 'string')
  assert.equal(typeof en.home.carouselLabel, 'string')
  assert.match(homeSource, /fetchHomeCarousel\(\)/)
  assert.match(homeSource, /const activeCarouselSlide = computed/)
  assert.match(homeSource, /v-if="activeCarouselSlide"/)
  assert.match(homeSource, /name: 'news-detail'/)
  assert.match(homeSource, /name: 'news'/)
  assert.match(homeSource, /name: 'referrals'/)
  assert.doesNotMatch(homeSource, /buildHomeMarketBrief|marketBrief/)
})

test('首页轮播支持自动播放、触摸、键盘、焦点和 reduced-motion 暂停', () => {
  assert.match(homeSource, /window\.setInterval\(nextCarousel, 5_500\)/)
  assert.match(homeSource, /@touchstart\.passive="onCarouselTouchStart"/)
  assert.match(homeSource, /@touchend\.passive="onCarouselTouchEnd"/)
  assert.match(homeSource, /@keydown="onCarouselKeydown"/)
  assert.match(homeSource, /prefers-reduced-motion: reduce/)
  assert.match(homeSource, /document\.addEventListener\('visibilitychange', updateCarouselVisibility\)/)
  assert.match(homeSource, /stopCarouselTimer\(\)/)
})
