import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'
import ts from 'typescript'
import { publicApiRequestConfig } from '../src/api/requestAuth.ts'
import { HomeCarouselContractError, parseHomeCarouselResponse } from '../src/core/homeCarouselContract.ts'

test('首页轮播契约映射公开 DTO 并保留启用状态与内部目标类型', () => {
  assert.deepEqual(parseHomeCarouselResponse({
    slides: [{
      id: 'invite',
      position: 0,
      enabled: false,
      locale: 'zh-CN',
      title: '邀请好友',
      description: '邀请好友获得奖励',
      image_url: 'https://cdn.example.test/invite.png',
      target: { type: 'referrals', id: null },
    }],
  }), [{
    id: 'invite',
    position: 0,
    enabled: false,
    locale: 'zh-CN',
    title: '邀请好友',
    description: '邀请好友获得奖励',
    imageUrl: 'https://cdn.example.test/invite.png',
    target: { type: 'referrals' },
  }])
})

test('首页轮播 API 适配器拒绝畸形或未知字段', async () => {
  assert.throws(() => parseHomeCarouselResponse({
    slides: [{ id: 'x', position: 0, unexpected: true }],
  }), HomeCarouselContractError)
})

function compileFunctions(path: string, names: string[], bindings: Record<string, unknown>, tail = '') {
  const source = readFileSync(new URL(path, import.meta.url), 'utf8')
  const functions = names.map((name) => {
    const match = source.match(new RegExp(`(?:export )?(?:async )?function ${name}\\([\\s\\S]*?\\n}`))
    assert.ok(match, `missing production function ${name}`)
    return match[0].replace(/^export /, '')
  }).join('\n')
  const code = ts.transpileModule(functions, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText
  return new Function(...Object.keys(bindings), `${code}\n${tail}\nreturn { ${names.join(',')} };`)(...Object.values(bindings))
}

test('公开轮播请求执行真实适配器、透传语言并禁止私有认证', async () => {
  const requests: Array<{ url: string; config: ReturnType<typeof publicApiRequestConfig> }> = []
  const bindings = {
    parseHomeCarouselResponse, publicApiRequestConfig, currentApiLocale: () => 'en-US',
    requestUrl: (path: string) => `/api/v1${path}`,
    client: { get: async (url: string, config: ReturnType<typeof publicApiRequestConfig>) => {
      requests.push({ url, config })
      return { data: { slides: [] } }
    } },
  }
  const api = compileFunctions('../src/api/homeCarousel.ts', ['fetchHomeCarousel'], bindings) as {
    fetchHomeCarousel(): Promise<unknown[]>
  }
  assert.deepEqual(await api.fetchHomeCarousel(), [])
  assert.equal(requests[0]!.url, '/api/v1/home/carousel')
  assert.deepEqual(requests[0]!.config, publicApiRequestConfig({ params: { locale: 'en-US' } }))
})

function carouselHarness() {
  const value = <T>(value: T) => ({ value })
  let resolve: (slides: unknown[]) => void = () => {}
  const calls: unknown[] = []
  const timers = new Map<number, () => void>()
  let timerId = 0
  const slides = value<unknown[]>([])
  const index = value(0)
  const hidden = value(false)
  const document = { hidden: false }
  const hovering = value(false)
  const reduced = value(false)
  const focused = value(false)
  const touching = value(false)
  const bindings = {
    carouselSlides: slides, carouselIndex: index, carouselState: value('idle'),
    carouselImageErrors: value(new Set()), carouselHidden: hidden, carouselTouching: touching,
    carouselHasMultipleSlides: { get value() { return slides.value.length > 1 } },
    carouselPaused: { get value() { return hidden.value || hovering.value || reduced.value || focused.value || touching.value } },
    carouselRef: value(null),
    document, Date, Node: class {},
    fetchHomeCarousel: () => new Promise<unknown[]>((r) => { resolve = r }),
    router: { push: (route: unknown) => calls.push(route) },
    window: {
      clearInterval: (id: number) => timers.delete(id),
      setInterval: (callback: () => void) => { timers.set(++timerId, callback); return timerId },
    },
  }
  const names = ['loadCarousel', 'selectCarousel', 'nextCarousel', 'previousCarousel', 'stopCarouselTimer',
    'restartCarouselTimer', 'updateCarouselVisibility', 'onCarouselKeydown', 'openCarouselTarget',
    'onCarouselTouchStart', 'onCarouselTouchEnd', 'onCarouselTouchCancel']
  const script = readFileSync(new URL('../src/views/HomeView.vue', import.meta.url), 'utf8')
  const functions = names.map((name) => script.match(new RegExp(`(?:async )?function ${name}\\([\\s\\S]*?\\n}`))![0]).join('\n')
  const code = ts.transpileModule(functions, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText
  // Keep production functions intact; only emulate their host refs and browser timer.
  const harness = new Function(...Object.keys(bindings), `
    let viewActive = true, carouselTimer, carouselRequestVersion = 0, carouselTouchStart = null, carouselSuppressClickUntil = 0;
    ${code}
    return { ${names.join(',')}, stopView() { viewActive = false; carouselRequestVersion++; stopCarouselTimer() } };
  `)(...Object.values(bindings)) as Record<string, (...args: any[]) => any>
  return { harness, slides, index, hidden, document, hovering, reduced, focused, touching, timers, calls,
    get resolveRequest() { return resolve },
    finish: (slides: unknown[]) => resolve(slides), stop: () => { harness.stopView() } }
}

const visibleSlides = [{ id: 'a', enabled: true }, { id: 'b', enabled: true }, { id: 'c', enabled: false }]

test('轮播忽略旧语言和卸载后的响应，空页与单页不启动播放', async () => {
  const h = carouselHarness()
  const old = h.harness.loadCarousel()
  const finishOld = h.resolveRequest
  const latest = h.harness.loadCarousel()
  finishOld(visibleSlides)
  await old
  assert.equal(h.slides.value.length, 0)
  h.finish([{ id: 'only', enabled: true }])
  await latest
  assert.equal(h.slides.value.length, 1)
  assert.equal(h.timers.size, 0)
  const pending = h.harness.loadCarousel()
  h.stop()
  h.finish(visibleSlides)
  await pending
  assert.equal(h.timers.size, 0)
  assert.equal(h.slides.value.length, 0)
})

test('没有播放按钮时仍支持自动播放、隐藏/悬停/焦点/低动态暂停与键盘切页', async () => {
  const h = carouselHarness()
  const loaded = h.harness.loadCarousel()
  h.finish(visibleSlides)
  await loaded
  assert.equal(h.slides.value.length, 2)
  assert.equal(h.timers.size, 1)
  h.timers.values().next().value!()
  assert.equal(h.index.value, 1)
  h.document.hidden = true
  h.harness.updateCarouselVisibility()
  assert.equal(h.hidden.value, true)
  assert.equal(h.timers.size, 0)
  h.document.hidden = false
  h.harness.updateCarouselVisibility()
  assert.equal(h.timers.size, 1)
  for (const pause of [h.hovering, h.reduced, h.focused, h.touching]) {
    pause.value = true
    h.harness.restartCarouselTimer()
    assert.equal(h.timers.size, 0)
    pause.value = false
    h.harness.restartCarouselTimer()
    assert.equal(h.timers.size, 1)
  }
  h.harness.onCarouselKeydown({ key: 'ArrowRight', preventDefault() {} })
  assert.equal(h.index.value, 0)
  const script = readFileSync(new URL('../src/views/HomeView.vue', import.meta.url), 'utf8')
  assert.doesNotMatch(script, /carouselUserPaused|home-carousel__pause|carouselPlay|carouselPause['"]/)
})

test('水平手势换页但纵向滚动不换页，滑动不会意外触发目的地', () => {
  const h = carouselHarness()
  h.slides.value = visibleSlides.slice(0, 2)
  const touch = (x: number, y: number) => ({ touches: [{ clientX: x, clientY: y }], changedTouches: [{ clientX: x, clientY: y }] })
  h.harness.onCarouselTouchStart(touch(200, 100))
  h.harness.onCarouselTouchEnd(touch(150, 220))
  assert.equal(h.index.value, 0)
  h.harness.onCarouselTouchStart(touch(200, 100))
  h.harness.onCarouselTouchEnd(touch(100, 110))
  assert.equal(h.index.value, 1)
  h.harness.openCarouselTarget({ target: { type: 'referrals' } })
  assert.equal(h.calls.length, 0)
  h.harness.onCarouselTouchStart(touch(100, 100))
  h.harness.onCarouselTouchEnd(touch(100, 100))
  h.harness.openCarouselTarget({ target: { type: 'news_detail', id: 4 } })
  assert.deepEqual(h.calls, [{ name: 'news-detail', params: { id: '4' } }])
  h.harness.onCarouselTouchCancel()
  assert.equal(h.touching.value, false)
})
