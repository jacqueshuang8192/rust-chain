import assert from 'node:assert/strict'
import test from 'node:test'
import { createChartOrientation } from '../src/core/chartOrientation.ts'
import { bindRotatedChartInput, rotatedChartPoint } from '../src/core/chartRotatedInput.ts'

function deferred() {
  let resolve!: () => void
  let reject!: (error: Error) => void
  const promise = new Promise<void>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}

function platform() {
  let fullscreen: Element | null = null
  let locks = 0
  let unlocks = 0
  let exits = 0
  const changes = new Set<() => void>()
  const element = { async requestFullscreen() { fullscreen = element } } as HTMLElement
  const environment = {
    fullscreenElement: () => fullscreen,
    async exitFullscreen() { fullscreen = null; exits += 1; changes.forEach((listener) => listener()) },
    orientation: {
      async lock(value: 'landscape') { assert.equal(value, 'landscape'); locks += 1 },
      unlock() { unlocks += 1 },
    },
    onFullscreenChange(listener: () => void) { changes.add(listener); return () => { changes.delete(listener) } },
  }
  return { environment, element, changes,
    setFullscreen(value: Element | null) { fullscreen = value },
    get counts() { return { locks, unlocks, exits } },
  }
}

test('chart owns native fullscreen and landscape only until Close', async () => {
  const f = platform()
  let closed = 0
  const controller = createChartOrientation(f.environment, () => { closed += 1 })
  await controller.enter(f.element)
  assert.equal(f.environment.fullscreenElement(), f.element)
  assert.deepEqual(f.counts, { locks: 1, unlocks: 0, exits: 0 })
  controller.exit()
  await Promise.resolve()
  assert.deepEqual(f.counts, { locks: 1, unlocks: 1, exits: 1 })
  assert.equal(closed, 0)
  controller.dispose()
  assert.equal(f.changes.size, 0)
})

test('native fullscreen escape closes presentation and releases orientation once', async () => {
  const f = platform()
  let closed = 0
  const controller = createChartOrientation(f.environment, () => { closed += 1 })
  await controller.enter(f.element)
  await f.environment.exitFullscreen()
  assert.equal(closed, 1)
  assert.equal(f.counts.unlocks, 1)
  controller.dispose()
  assert.equal(f.counts.unlocks, 1)
})

test('rejected native APIs leave CSS presentation available without unhandled rejection', async () => {
  for (const failure of ['fullscreen', 'orientation']) {
    const f = platform()
    const fail = async () => { throw new Error('unsupported') }
    if (failure === 'fullscreen') f.element.requestFullscreen = fail
    else f.environment.orientation.lock = fail
    const controller = createChartOrientation(f.environment, () => assert.fail('must not close CSS fallback'))
    await controller.enter(f.element)
    assert.equal(f.counts.unlocks, 0)
    controller.dispose()
  }
})

test('Close before fullscreen grant cleans up the late native result', async () => {
  const f = platform()
  const grant = deferred()
  f.element.requestFullscreen = async () => { await grant.promise; f.setFullscreen(f.element) }
  const controller = createChartOrientation(f.environment, () => {})
  const enter = controller.enter(f.element)
  await Promise.resolve()
  controller.exit()
  grant.resolve()
  await enter
  assert.equal(f.environment.fullscreenElement(), null)
  assert.equal(f.counts.locks, 0)
  controller.dispose()
})

test('unmount during orientation grant unlocks its late result and removes listeners', async () => {
  const f = platform()
  const grant = deferred()
  f.environment.orientation.lock = () => grant.promise
  const controller = createChartOrientation(f.environment, () => assert.fail('disposed listener'))
  const enter = controller.enter(f.element)
  await Promise.resolve(); await Promise.resolve()
  controller.dispose()
  grant.resolve()
  await enter
  assert.equal(f.environment.fullscreenElement(), null)
  assert.equal(f.counts.unlocks, 1)
  assert.equal(f.changes.size, 0)
})

test('reopening waits for owned fullscreen cleanup without closing the new presentation', async () => {
  const f = platform()
  const cleanup = deferred()
  const exit = f.environment.exitFullscreen
  f.environment.exitFullscreen = async () => { await cleanup.promise; await exit() }
  const controller = createChartOrientation(f.environment, () => assert.fail('new session closed by old cleanup'))
  await controller.enter(f.element)
  controller.exit()
  const again = controller.enter(f.element)
  cleanup.resolve()
  await again
  assert.equal(f.environment.fullscreenElement(), f.element)
  assert.equal(f.counts.locks, 2)
  controller.dispose()
})

test('chart does not take over another fullscreen owner or its orientation', async () => {
  const f = platform()
  const other = {} as Element
  f.setFullscreen(other)
  const controller = createChartOrientation(f.environment, () => {})
  await controller.enter(f.element)
  controller.dispose()
  assert.equal(f.environment.fullscreenElement(), other)
  assert.deepEqual(f.counts, { locks: 0, unlocks: 0, exits: 0 })
})

test('clockwise fallback maps corners and scaled canvas coordinates back to logical axes', () => {
  const box = { left: 10, top: 20, right: 210, width: 200, height: 400 }
  assert.deepEqual(rotatedChartPoint({ clientX: 210, clientY: 20 }, box, 800, 400), { clientX: 10, clientY: 20 })
  assert.deepEqual(rotatedChartPoint({ clientX: 10, clientY: 420 }, box, 800, 400), { clientX: 810, clientY: 420 })
  assert.deepEqual(rotatedChartPoint({ clientX: 110, clientY: 220 }, box, 800, 400), { clientX: 410, clientY: 220 })
})

function inputFixture() {
  const document = new EventTarget()
  class Element {
    tagName: string
    clientWidth = 500
    clientHeight = 200
    ownerDocument = document
    constructor(tagName: string) { this.tagName = tagName }
    getBoundingClientRect() { return { left: 20, top: 30, right: 220, width: 200, height: 500 } }
    contains(target: unknown) { return target === canvas }
    closest() { return root }
  }
  const window = Object.assign(new EventTarget(), { HTMLElement: Element, scrollX: 0, scrollY: 50 })
  Object.assign(document, { defaultView: window })
  const host = new Element('DIV'), root = new Element('DIV'), canvas = new Element('CANVAS')
  let rotated = true
  const stop = bindRotatedChartInput(host as unknown as HTMLElement, () => rotated)
  function dispatch(type: string, fields: Record<string, unknown> = {}) {
    const event = new Event(type)
    Object.defineProperties(event, Object.fromEntries(Object.entries({ target: canvas, clientX: 160, clientY: 230, ...fields })
      .map(([key, value]) => [key, { configurable: true, value }])))
    document.dispatchEvent(event)
    return event as MouseEvent & TouchEvent
  }
  return { dispatch, stop, host, canvas, root, setRotated(value: boolean) { rotated = value } }
}

test('rotated input adapts mouse/crosshair and drag-outside, but not buttons or inline view', () => {
  const f = inputFixture()
  const down = f.dispatch('mousedown')
  assert.equal(down.clientX, 220)
  assert.equal(down.clientY, 90)
  assert.equal(down.pageY, 140)
  const move = f.dispatch('mousemove', { target: f.host, clientY: 330 })
  assert.equal(move.clientX, 320)
  f.dispatch('mouseup', { target: f.host })
  assert.equal(f.dispatch('click', { target: f.host }).clientX, 160)
  f.setRotated(false)
  assert.equal(f.dispatch('mousemove').clientX, 160)
  f.setRotated(true)
  f.dispatch('mousedown')
  f.setRotated(false)
  f.dispatch('mouseup', { target: f.host })
  f.setRotated(true)
  assert.equal(f.dispatch('mousemove', { target: f.host }).clientX, 160)
  f.stop()
  assert.equal(f.dispatch('mousemove').clientX, 160)
})

test('rotated touch lists retain identities and pinch distances without mutating browser touches', () => {
  const f = inputFixture()
  const first = Object.freeze({ identifier: 1, target: f.canvas, clientX: 160, clientY: 230, screenX: 1, screenY: 2 })
  const second = Object.freeze({ ...first, identifier: 2, clientY: 330 })
  const event = f.dispatch('touchstart', { touches: [first, second], targetTouches: [first, second], changedTouches: [second] })
  assert.equal(event.touches[0]!.identifier, 1)
  assert.equal(event.touches[1]!.identifier, 2)
  assert.equal(event.touches[0]!.clientX, 220)
  assert.equal(event.touches[1]!.clientX, 320)
  assert.equal(event.touches.item(0), event.targetTouches.item(0))
  assert.equal(event.changedTouches[0]!.target, f.canvas)
  assert.equal(first.clientX, 160)
  f.dispatch('touchend', { touches: [], targetTouches: [], changedTouches: [first, second] })
  assert.equal(f.dispatch('mousemove', { target: f.host }).clientX, 160)
  f.stop()
})
