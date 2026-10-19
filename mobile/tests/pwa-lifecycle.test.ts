import assert from 'node:assert/strict'
import test from 'node:test'
import {
  createPwaInstallEligibilitySession,
  isPwaInstallValueRoute,
  PWA_INSTALL_FREQUENCY_CAP_MS,
  PWA_INSTALL_SESSION_DELAY_MS,
} from '../src/pwa/eligibility.ts'
import {
  runPwaUpdate,
  type PwaUpdateFailureReason,
  type PwaUpdateTimer,
} from '../src/pwa/update.ts'

class FakeTimer implements PwaUpdateTimer {
  private elapsed = 0
  private nextId = 1
  private readonly tasks = new Map<number, { at: number; callback: () => void }>()

  clear(handle: unknown): void {
    this.tasks.delete(Number(handle))
  }

  set(callback: () => void, delayMs: number): unknown {
    const id = this.nextId++
    this.tasks.set(id, { at: this.elapsed + delayMs, callback })
    return id
  }

  advance(delayMs: number): void {
    const target = this.elapsed + delayMs
    while (true) {
      const next = [...this.tasks.entries()]
        .filter(([, task]) => task.at <= target)
        .sort((left, right) => left[1].at - right[1].at || left[0] - right[0])[0]
      if (!next) break
      this.tasks.delete(next[0])
      this.elapsed = next[1].at
      next[1].callback()
    }
    this.elapsed = target
  }
}

class FakeWorker extends EventTarget {
  state: ServiceWorkerState = 'installed'
  readonly messages: unknown[] = []
  onPostMessage?: () => void

  postMessage(message: unknown): void {
    this.messages.push(message)
    this.onPostMessage?.()
  }
}

class FakeController extends EventTarget {
  controller: FakeWorker | null = new FakeWorker()
  activate(worker: FakeWorker): void {
    this.controller = worker
    this.dispatchEvent(new Event('controllerchange'))
  }
}

class FakeRegistration extends EventTarget {
  installing: FakeWorker | null = null
  waiting: FakeWorker | null
  readonly update: () => Promise<ServiceWorkerRegistration>

  constructor(waiting: FakeWorker | null, update?: () => Promise<ServiceWorkerRegistration>) {
    super()
    this.waiting = waiting
    this.update = update || (async () => this as unknown as ServiceWorkerRegistration)
  }
}

function updateHarness(options: {
  controller: FakeController
  timer: FakeTimer
  waiting: FakeWorker | null
  update?: () => Promise<ServiceWorkerRegistration>
}) {
  const busy: boolean[] = []
  const errors: boolean[] = []
  const failures: PwaUpdateFailureReason[] = []
  let reloads = 0
  let current = 0
  const loadedController = options.controller.controller as unknown as ServiceWorker | null

  const registration = new FakeRegistration(options.waiting, options.update)
  const run = () => runPwaUpdate({
    registration: registration as unknown as ServiceWorkerRegistration,
    controllerTarget: options.controller as unknown as ServiceWorkerContainer,
    loadedController,
    onNoUpdate: () => { current += 1 },
    timeoutMs: 100,
    timer: options.timer,
    onBusyChange: (value) => busy.push(value),
    onErrorChange: (value) => errors.push(value),
    onFailure: (reason) => failures.push(reason),
    reload: () => { reloads += 1 },
  })

  return { busy, errors, failures, registration, current: () => current, reloads: () => reloads, run }
}

test('PWA update waits for controllerchange, clears busy, and reloads once', async () => {
  const controller = new FakeController()
  const timer = new FakeTimer()
  const worker = new FakeWorker()
  worker.onPostMessage = () => controller.activate(worker)
  const harness = updateHarness({ controller, timer, waiting: worker })

  assert.equal(await harness.run(), true)
  assert.deepEqual(worker.messages, [{ type: 'SKIP_WAITING' }])
  assert.deepEqual(harness.busy, [true, false])
  assert.deepEqual(harness.errors, [false])
  assert.deepEqual(harness.failures, [])
  assert.equal(harness.reloads(), 1)
})

test('PWA update times out without controllerchange, recovers busy, and retries successfully', async () => {
  const controller = new FakeController()
  const timer = new FakeTimer()
  const worker = new FakeWorker()
  const harness = updateHarness({ controller, timer, waiting: worker })

  const timedOut = harness.run()
  assert.deepEqual(harness.busy, [true])
  timer.advance(100)
  assert.equal(await timedOut, false)
  assert.deepEqual(harness.busy, [true, false])
  assert.deepEqual(harness.errors, [false, true])
  assert.deepEqual(harness.failures, ['activation-timeout'])
  assert.equal(harness.reloads(), 0)

  worker.onPostMessage = () => controller.activate(worker)
  assert.equal(await harness.run(), true)
  assert.equal(worker.messages.length, 2)
  assert.deepEqual(harness.busy, [true, false, true, false])
  assert.deepEqual(harness.errors, [false, true, false])
  assert.equal(harness.reloads(), 1)
})

test('PWA update check is bounded and never leaves busy set', async () => {
  const controller = new FakeController()
  const timer = new FakeTimer()
  const harness = updateHarness({
    controller,
    timer,
    waiting: null,
    update: () => new Promise<ServiceWorkerRegistration>(() => {}),
  })

  const result = harness.run()
  timer.advance(100)
  assert.equal(await result, false)
  assert.deepEqual(harness.busy, [true, false])
  assert.deepEqual(harness.failures, ['update-check-timeout'])
})

test('PWA update recovers when the waiting worker rejects activation or becomes redundant', async () => {
  const throwingController = new FakeController()
  const throwingTimer = new FakeTimer()
  const throwingWorker = new FakeWorker()
  throwingWorker.onPostMessage = () => { throw new Error('fixture postMessage failure') }
  const throwingHarness = updateHarness({
    controller: throwingController,
    timer: throwingTimer,
    waiting: throwingWorker,
  })

  assert.equal(await throwingHarness.run(), false)
  assert.deepEqual(throwingHarness.busy, [true, false])
  assert.deepEqual(throwingHarness.failures, ['post-message-failed'])

  const redundantController = new FakeController()
  const redundantTimer = new FakeTimer()
  const redundantWorker = new FakeWorker()
  redundantWorker.onPostMessage = () => {
    redundantWorker.state = 'redundant'
    redundantWorker.dispatchEvent(new Event('statechange'))
  }
  const redundantHarness = updateHarness({
    controller: redundantController,
    timer: redundantTimer,
    waiting: redundantWorker,
  })

  assert.equal(await redundantHarness.run(), false)
  assert.deepEqual(redundantHarness.busy, [true, false])
  assert.deepEqual(redundantHarness.failures, ['worker-redundant'])
})

test('PWA update clears a stale notice when the checked registration has no new worker', async () => {
  const harness = updateHarness({
    controller: new FakeController(),
    timer: new FakeTimer(),
    waiting: null,
  })

  assert.equal(await harness.run(), false)
  assert.deepEqual(harness.busy, [true, false])
  assert.deepEqual(harness.errors, [false])
  assert.deepEqual(harness.failures, [])
  assert.equal(harness.current(), 1)
})

test('PWA update waits for a newly installing worker before requesting activation', async () => {
  const controller = new FakeController()
  const timer = new FakeTimer()
  const worker = new FakeWorker()
  worker.state = 'installing'
  let registration: FakeRegistration
  const harness = updateHarness({
    controller,
    timer,
    waiting: null,
    update: async () => {
      registration.installing = worker
      registration.dispatchEvent(new Event('updatefound'))
      return registration as unknown as ServiceWorkerRegistration
    },
  })
  registration = harness.registration
  worker.onPostMessage = () => controller.activate(worker)

  const result = harness.run()
  await Promise.resolve()
  worker.state = 'installed'
  worker.dispatchEvent(new Event('statechange'))
  assert.equal(await result, true)
  assert.deepEqual(worker.messages, [{ type: 'SKIP_WAITING' }])
  assert.deepEqual(harness.failures, [])
})

test('install eligibility requires session delay and a value action, then caps a closed offer', () => {
  const startedAt = 1_000
  const session = createPwaInstallEligibilitySession(startedAt)
  const base = {
    hasInstallSurface: true,
    isStandalone: false,
  }

  assert.deepEqual(session.evaluate({ ...base, now: startedAt + PWA_INSTALL_SESSION_DELAY_MS + 1 }), {
    eligible: false,
    newlyGranted: false,
  })

  session.recordValueAction()
  assert.deepEqual(session.evaluate({ ...base, now: startedAt + PWA_INSTALL_SESSION_DELAY_MS - 1 }), {
    eligible: false,
    newlyGranted: false,
  })

  const shownAt = startedAt + PWA_INSTALL_SESSION_DELAY_MS
  assert.deepEqual(session.evaluate({ ...base, now: shownAt }), {
    eligible: true,
    newlyGranted: true,
  })
  assert.equal(session.markOfferShown(), true)
  assert.equal(session.markOfferShown(), false)
  assert.deepEqual(session.evaluate({ ...base, now: shownAt + 1, lastShownAt: shownAt }), {
    eligible: true,
    newlyGranted: false,
  })

  session.closeOffer()
  assert.deepEqual(session.evaluate({ ...base, now: shownAt + 1, lastShownAt: shownAt }), {
    eligible: false,
    newlyGranted: false,
  })
  assert.deepEqual(session.evaluate({
    ...base,
    now: shownAt + PWA_INSTALL_FREQUENCY_CAP_MS,
    lastShownAt: shownAt,
  }), {
    eligible: true,
    newlyGranted: true,
  })
})

test('install value actions are explicit business routes rather than every route visit', () => {
  for (const route of ['market-detail', 'trade', 'seconds', 'assets', 'wallet-ledger']) {
    assert.equal(isPwaInstallValueRoute(route), true, route)
  }
  for (const route of ['home', 'markets', 'login', 'register', undefined]) {
    assert.equal(isPwaInstallValueRoute(route), false, String(route))
  }
})

test('retry recovers a worker activated by another tab or after an earlier timeout', async () => {
  const controller = new FakeController()
  const timer = new FakeTimer()
  const worker = new FakeWorker()
  const harness = updateHarness({ controller, timer, waiting: worker })
  const first = harness.run()
  timer.advance(100)
  assert.equal(await first, false)
  harness.registration.waiting = null
  controller.activate(worker)
  assert.equal(harness.reloads(), 0, 'late controller events cannot reload the interactive page')
  assert.equal(await harness.run(), true)
  assert.equal(harness.reloads(), 1)
  assert.equal(worker.messages.length, 1, 'retry does not message an already activated worker')
})

test('controller loss and unchanged controllerchange events never authorize reload', async () => {
  const controller = new FakeController()
  const timer = new FakeTimer()
  const worker = new FakeWorker()
  const harness = updateHarness({ controller, timer, waiting: worker })
  const result = harness.run()
  controller.dispatchEvent(new Event('controllerchange'))
  controller.controller = null
  controller.dispatchEvent(new Event('controllerchange'))
  assert.equal(harness.reloads(), 0)
  timer.advance(100)
  assert.equal(await result, false)
  harness.registration.waiting = null
  assert.equal(await harness.run(), false)
  assert.deepEqual(harness.failures, ['activation-timeout', 'controller-unavailable'])
})

test('one total deadline covers checking, download and activation, with no late side effects', async () => {
  const controller = new FakeController()
  const timer = new FakeTimer()
  const worker = new FakeWorker()
  worker.state = 'installing'
  let checked!: (registration: ServiceWorkerRegistration) => void
  const harness = updateHarness({
    controller, timer, waiting: null,
    update: () => new Promise((resolve) => { checked = resolve }),
  })
  const result = harness.run()
  timer.advance(40)
  harness.registration.installing = worker
  harness.registration.dispatchEvent(new Event('updatefound'))
  checked(harness.registration as unknown as ServiceWorkerRegistration)
  await Promise.resolve()
  timer.advance(40)
  worker.state = 'installed'
  worker.dispatchEvent(new Event('statechange'))
  assert.deepEqual(worker.messages, [{ type: 'SKIP_WAITING' }])
  timer.advance(20)
  assert.equal(await result, false)
  assert.deepEqual(harness.failures, ['activation-timeout'])
  controller.activate(worker)
  worker.dispatchEvent(new Event('statechange'))
  harness.registration.dispatchEvent(new Event('updatefound'))
  assert.equal(harness.reloads(), 0)
  assert.equal(worker.messages.length, 1)
  assert.deepEqual(harness.busy, [true, false])
})

test('download finishing after the deadline cannot activate itself', async () => {
  const worker = new FakeWorker()
  worker.state = 'installing'
  const timer = new FakeTimer()
  const harness = updateHarness({ controller: new FakeController(), timer, waiting: null })
  harness.registration.installing = worker
  const result = harness.run()
  timer.advance(100)
  assert.equal(await result, false)
  worker.state = 'installed'
  worker.dispatchEvent(new Event('statechange'))
  assert.deepEqual(worker.messages, [])
  assert.deepEqual(harness.failures, ['update-check-timeout'])
})

test('update check rejection unlocks controls and retry can find the new worker', async () => {
  const controller = new FakeController()
  const timer = new FakeTimer()
  const harness = updateHarness({
    controller, timer, waiting: null, update: async () => { throw new Error('offline') },
  })
  assert.equal(await harness.run(), false)
  assert.deepEqual(harness.failures, ['update-check-failed'])
  const worker = new FakeWorker()
  harness.registration.waiting = worker
  worker.onPostMessage = () => controller.activate(worker)
  assert.equal(await harness.run(), true)
  assert.deepEqual(harness.busy, [true, false, true, false])
})

test('a missing registration is recovered within the same deadline', async () => {
  const controller = new FakeController()
  const loadedController = controller.controller as unknown as ServiceWorker
  const worker = new FakeWorker()
  worker.onPostMessage = () => controller.activate(worker)
  const registration = new FakeRegistration(worker)
  let registrations = 0
  let reloads = 0
  const busy: boolean[] = []
  assert.equal(await runPwaUpdate({
    controllerTarget: controller as unknown as ServiceWorkerContainer,
    loadedController, registration: null,
    recoverRegistration: async () => { registrations++; return registration as unknown as ServiceWorkerRegistration },
    reload: () => { reloads++ }, onBusyChange: (value) => busy.push(value), onErrorChange: () => {},
  }), true)
  assert.equal(registrations, 1)
  assert.equal(reloads, 1)
  assert.deepEqual(busy, [true, false])
})

test('a registration resolving after the deadline cannot activate a worker', async () => {
  const controller = new FakeController()
  const timer = new FakeTimer()
  const worker = new FakeWorker()
  const registration = new FakeRegistration(worker)
  let registered!: (registration: ServiceWorkerRegistration) => void
  const failures: PwaUpdateFailureReason[] = []
  const result = runPwaUpdate({
    controllerTarget: controller as unknown as ServiceWorkerContainer,
    loadedController: controller.controller as unknown as ServiceWorker,
    registration: null, timer, timeoutMs: 100,
    recoverRegistration: () => new Promise((resolve) => { registered = resolve }),
    reload: () => assert.fail('must not reload'), onBusyChange: () => {}, onErrorChange: () => {},
    onFailure: (reason) => failures.push(reason),
  })
  timer.advance(100)
  assert.equal(await result, false)
  registered(registration as unknown as ServiceWorkerRegistration)
  await Promise.resolve()
  assert.deepEqual(worker.messages, [])
  assert.deepEqual(failures, ['update-check-timeout'])
})
