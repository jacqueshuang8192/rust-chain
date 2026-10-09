import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { preparePwaStartup, PWA_STARTUP_TIMEOUT_MS } from '../src/pwa/startup.ts'

class Worker extends EventTarget {
  state: ServiceWorkerState = 'installed'
  messages: unknown[] = []
  onMessage = () => {}
  postMessage(message: unknown) {
    this.messages.push(message)
    this.onMessage()
  }
}

class Registration extends EventTarget {
  waiting: Worker | null = null
  installing: Worker | null = null
  update = async () => this as unknown as ServiceWorkerRegistration
}

function harness() {
  const registration = new Registration()
  const controller = Object.assign(new EventTarget(), { controller: new Worker() })
  const timers = new Map<number, () => void>()
  let reloads = 0
  let requestedDelay = 0
  const run = (register = async () => registration as unknown as ServiceWorkerRegistration | null) => (
    preparePwaStartup({
      register,
      controllerTarget: controller as unknown as ServiceWorkerContainer,
      reload: () => { reloads++ },
      timer: {
        set(callback, delay) { requestedDelay = delay; timers.set(1, callback); return 1 },
        clear(handle) { timers.delete(Number(handle)) },
      },
    })
  )
  const activate = (worker: Worker) => {
    controller.controller = worker
    worker.state = 'activated'
    controller.dispatchEvent(new Event('controllerchange'))
  }
  return {
    registration, controller, timers, run, activate,
    reloads: () => reloads,
    expire: () => { for (const callback of timers.values()) callback() },
    delay: () => requestedDelay,
  }
}

const flush = async () => { for (let i = 0; i < 8; i++) await Promise.resolve() }

test('startup activates an already waiting worker and reloads only after controller change', async () => {
  const h = harness()
  const worker = new Worker()
  h.registration.waiting = worker
  const pending = h.run()
  await flush()
  assert.deepEqual(worker.messages, [{ type: 'SKIP_WAITING' }])
  assert.equal(h.reloads(), 0)
  h.activate(worker)
  assert.equal(await pending, true)
  h.activate(new Worker())
  assert.equal(h.reloads(), 1)
  assert.equal(h.timers.size, 0)
})

test('startup finds a new deployment, waits for installation, then activates it', async () => {
  const h = harness()
  const worker = new Worker()
  worker.state = 'installing'
  h.registration.update = async () => {
    h.registration.installing = worker
    h.registration.dispatchEvent(new Event('updatefound'))
    return h.registration as unknown as ServiceWorkerRegistration
  }
  const pending = h.run()
  await flush()
  assert.deepEqual(worker.messages, [])
  worker.onMessage = () => h.activate(worker)
  worker.state = 'installed'
  worker.dispatchEvent(new Event('statechange'))
  assert.equal(await pending, true)
  assert.equal(h.reloads(), 1)
})

test('startup releases the shell for no update, offline failure, or registration failure', async () => {
  for (const failure of ['none', 'update', 'registration', 'null'] as const) {
    const h = harness()
    if (failure === 'update') h.registration.update = async () => { throw new Error('offline') }
    const result = await h.run(async () => {
      if (failure === 'registration') throw new Error('registration failed')
      return failure === 'null' ? null : h.registration as unknown as ServiceWorkerRegistration
    })
    assert.equal(result, false)
    assert.equal(h.reloads(), 0)
    assert.equal(h.timers.size, 0)
  }
})

test('one startup deadline includes registration and ignores late registration completion', async () => {
  const h = harness()
  const worker = new Worker()
  h.registration.waiting = worker
  let registered!: (value: ServiceWorkerRegistration) => void
  const pending = h.run(() => new Promise((resolve) => { registered = resolve }))
  assert.equal(h.delay(), PWA_STARTUP_TIMEOUT_MS)
  h.expire()
  assert.equal(await pending, false)
  registered(h.registration as unknown as ServiceWorkerRegistration)
  await flush()
  assert.deepEqual(worker.messages, [])
  assert.equal(h.reloads(), 0)
  assert.equal(h.timers.size, 0)
})

test('late installation or controller activation never reloads an interactive application', async () => {
  for (const state of ['installing', 'installed'] as const) {
    const h = harness()
    const worker = new Worker()
    worker.state = state
    h.registration.installing = worker
    const pending = h.run()
    await flush()
    const messageCount = worker.messages.length
    h.expire()
    assert.equal(await pending, false)
    worker.state = 'installed'
    worker.dispatchEvent(new Event('statechange'))
    h.activate(worker)
    assert.equal(worker.messages.length, messageCount)
    assert.equal(h.reloads(), 0)
    assert.equal(h.timers.size, 0)
  }
})

test('worker rejection, redundant workers and hung update checks cannot block startup', async () => {
  for (const failure of ['post-message', 'redundant', 'hung-update'] as const) {
    const h = harness()
    const worker = new Worker()
    if (failure === 'hung-update') {
      h.registration.update = () => new Promise(() => {})
    } else {
      h.registration.waiting = worker
      if (failure === 'redundant') worker.state = 'redundant'
      else worker.onMessage = () => { throw new Error('worker unavailable') }
    }
    const pending = h.run()
    await flush()
    if (failure === 'hung-update') h.expire()
    assert.equal(await pending, false)
    assert.equal(h.reloads(), 0)
    assert.equal(h.timers.size, 0)
  }
})

test('first visit does not delay application mounting or register through the startup guard', async () => {
  let registrations = 0
  assert.equal(await preparePwaStartup({
    controllerTarget: Object.assign(new EventTarget(), { controller: null }),
    register: async () => { registrations++; return null },
    reload: () => assert.fail('first visit must not reload'),
  }), false)
  assert.equal(registrations, 0)
})

test('production bootstrap waits before mounting, and preserves Tauri and offline boundaries', () => {
  const main = readFileSync(new URL('../src/main.ts', import.meta.url), 'utf8')
  const pwa = readFileSync(new URL('../src/pwa/index.ts', import.meta.url), 'utf8')
  assert.ok(main.indexOf('await initializePwa()') < main.indexOf('createApp(App)'))
  assert.match(main, /if \(reloading\) return/)
  assert.match(main, /__PWA_ENABLED__ && !isTauriRuntime\(\)/)
  assert.match(pwa, /!navigator\.serviceWorker\?\.controller \|\| !state\.isOnline/)
  assert.match(pwa, /return preparePwaStartup\(/)
  assert.match(pwa, /updateViaCache: 'none'/)
})
