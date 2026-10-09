import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import ts from 'typescript'

function harness() {
  const source = readFileSync(new URL('../src/composables/useSecondsPrivateReconciliation.ts', import.meta.url), 'utf8')
  const code = ts.transpileModule(source, { compilerOptions: {
    target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS,
  } }).outputText
  let mounted = () => {}
  let unmount = () => {}
  let lease: { onEvent: () => void; onOpen: () => void } | undefined
  const timers = new Set<() => void>()
  const listeners = new Map<string, () => void>()
  const document = {
    visibilityState: 'visible',
    addEventListener: (name: string, fn: () => void) => listeners.set(name, fn),
    removeEventListener: (name: string) => listeners.delete(name),
  }
  const bindings = {
    exports: {} as Record<string, any>,
    require: (name: string) => name === 'vue' ? {
      onMounted: (fn: () => void) => { mounted = fn },
      onBeforeUnmount: (fn: () => void) => { unmount = fn },
    } : { usePrivateUserStreamLease: (options: typeof lease) => { lease = options; return { live: { value: true } } } },
    document, window: document,
    setInterval: (fn: () => void) => { timers.add(fn); return fn },
    clearInterval: (fn: () => void) => timers.delete(fn),
  }
  new Function(...Object.keys(bindings), code)(...Object.values(bindings))
  const completions: Array<(value: { ordersLoaded: boolean; accountsLoaded: boolean }) => void> = []
  const results: boolean[] = []
  let enabled = true
  let key = 'first'
  const controller = bindings.exports.useSecondsPrivateReconciliation({
    enabled: () => enabled,
    sessionKey: () => key,
    reconcile: () => new Promise(resolve => completions.push(resolve)),
    onResult: (loaded: boolean) => results.push(loaded),
  }) as { refresh: () => Promise<void> }
  mounted()
  return { controller, completions, results, timers, listeners, document,
    lease: () => lease!, unmount: () => unmount(),
    changeSession: () => { key = 'second' }, logout: () => { enabled = false } }
}

const ready = { ordersLoaded: true, accountsLoaded: true }
const flush = async () => { for (let i = 0; i < 8; i++) await Promise.resolve() }

test('秒合约 WS 事件与重连合并在途请求，并保留最后一次补偿刷新', async () => {
  const h = harness()
  h.lease().onOpen()
  h.lease().onEvent()
  h.lease().onEvent()
  assert.equal(h.completions.length, 1)
  h.completions[0]!(ready)
  await flush()
  assert.equal(h.completions.length, 2)
  h.completions[1]!(ready)
  await flush()
  assert.deepEqual(h.results, [true, true])
  h.unmount()
  assert.equal(h.timers.size, 0)
  assert.equal(h.listeners.size, 0)
})

test('旧会话和卸载后的刷新结果不回写，隐藏页不做恢复请求', async () => {
  const h = harness()
  const first = h.controller.refresh()
  h.changeSession()
  h.completions[0]!(ready)
  await first
  assert.deepEqual(h.results, [])
  h.document.visibilityState = 'hidden'
  h.listeners.get('visibilitychange')!()
  assert.equal(h.completions.length, 1)
  h.document.visibilityState = 'visible'
  h.listeners.get('focus')!()
  assert.equal(h.completions.length, 2)
  h.unmount()
  h.completions[1]!(ready)
  await flush()
  assert.deepEqual(h.results, [])
  await h.controller.refresh()
  assert.equal(h.completions.length, 2)
})

test('访客没有后台请求，读取失败保持可恢复状态', async () => {
  const h = harness()
  const pending = h.controller.refresh()
  h.completions[0]!({ ordersLoaded: false, accountsLoaded: true })
  await pending
  assert.deepEqual(h.results, [false])
  h.logout()
  await h.controller.refresh()
  assert.equal(h.completions.length, 1)
  h.unmount()
})
