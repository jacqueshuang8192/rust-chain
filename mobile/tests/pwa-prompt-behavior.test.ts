import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { compileScript, parse } from '@vue/compiler-sfc'
import ts from 'typescript'
import * as Vue from 'vue'
import { createI18n } from 'vue-i18n'
import zh from '../src/i18n/messages/zh-CN.ts'
import en from '../src/i18n/messages/en.ts'
import type { PwaUpdateFailureReason } from '../src/pwa/update.ts'

interface Prompt {
  showDialog: Vue.ComputedRef<boolean>
  showStatusIsland: Vue.ComputedRef<boolean>
  isUpdateDialog: Vue.ComputedRef<boolean>
  dialogBusy: Vue.ComputedRef<boolean>
  dialogHint: Vue.ComputedRef<string>
  dialogCopy: Vue.ComputedRef<Record<string, string>>
  dialogBenefits: Vue.ComputedRef<Array<{ title: string; description: string }>>
  installHint: Vue.Ref<{ focus(): void } | null>
  promptDialog: Vue.Ref<{ focus(): void } | null>
  closeDialog(): void
  confirmDialog(): Promise<void>
  retry(): Promise<void>
}

function fixture() {
  const state = Vue.reactive({
    enabled: true, isOnline: true, needRefresh: true, updateDismissed: false,
    updateError: false, updateFailure: null as PwaUpdateFailureReason | null, updating: false,
    installError: false, installAvailable: true, iosInstallAvailable: false,
    offlineReady: false, registrationError: false,
  })
  const route = Vue.reactive({ name: 'home', fullPath: '/' })
  const i18n = createI18n({ legacy: false, locale: 'zh-CN', messages: { 'zh-CN': zh, en } })
  const calls = { update: 0, install: 0, shown: 0, retry: 0 }
  let finishInstall!: () => void
  let finishRetry!: () => void
  const modalTransitions: boolean[] = []
  const mounted: Array<() => void> = []
  const { descriptor } = parse(readFileSync(new URL('../src/components/PwaStatus.vue', import.meta.url), 'utf8'))
  const output = ts.transpileModule(compileScript(descriptor, { id: 'pwa-prompt-test' }).content, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
  }).outputText
  const modules: Record<string, unknown> = {
    vue: { ...Vue, onMounted: (callback: () => void) => mounted.push(callback) },
    'vue-router': { useRoute: () => route },
    'vue-i18n': { useI18n: () => i18n.global },
    'lucide-vue-next': {}, '@/assets/brand/hippo-logo-landscape.png': {},
    '@/core/modalDialog': { useModalDialog: (open: Vue.Ref<boolean>) => {
      Vue.watch(open, value => modalTransitions.push(value))
      return { trapFocus() {} }
    } },
    '@/pwa/eligibility': { isPwaInstallValueRoute: () => false },
    '@/pwa': {
      pwaState: state,
      applyPwaUpdate: async () => { calls.update++; state.updating = true; return false },
      dismissOfflineReady: () => { state.offlineReady = false },
      dismissPwaInstall: () => { state.installAvailable = false; state.iosInstallAvailable = false },
      dismissPwaUpdate: () => { state.updateDismissed = true },
      markPwaInstallOfferShown: () => { calls.shown++ }, markPwaInstallValueAction() {},
      promptPwaInstall: () => { calls.install++; return new Promise<void>(resolve => { finishInstall = resolve }) },
      retryPwaRegistration: () => { calls.retry++; return new Promise<void>(resolve => { finishRetry = resolve }) },
    },
  }
  const exports: { default?: { setup: (props: object, context: object) => Prompt } } = {}
  new Function('require', 'exports', output)((name: string) => {
    assert.ok(Object.hasOwn(modules, name), name)
    return modules[name]
  }, exports)
  const scope = Vue.effectScope()
  const prompt = scope.run(() => exports.default!.setup({}, { expose() {} }))!
  return {
    prompt, state, route, calls, modalTransitions, i18n, stop: () => scope.stop(),
    finishInstall: () => finishInstall(), finishRetry: () => finishRetry(),
    async mount() { mounted.forEach(callback => callback()); await Vue.nextTick() },
  }
}

test('an initially available update opens the shared modal after mount and takes priority over install', async () => {
  const f = fixture()
  try {
    assert.equal(f.prompt.showDialog.value, false)
    assert.equal(f.calls.shown, 0)
    await f.mount()
    assert.equal(f.prompt.showDialog.value, true)
    assert.equal(f.prompt.isUpdateDialog.value, true)
    assert.deepEqual(f.modalTransitions, [true], 'initial display must run focus/scroll-lock lifecycle')
    f.prompt.closeDialog()
    await Vue.nextTick()
    assert.equal(f.prompt.isUpdateDialog.value, false)
    assert.equal(f.prompt.showDialog.value, true)
    assert.deepEqual(f.modalTransitions, [true], 'update-to-install reuses one modal lock')
    assert.equal(f.calls.shown, 1)
    f.prompt.closeDialog()
    await Vue.nextTick()
    assert.equal(f.prompt.showDialog.value, false)
    assert.deepEqual(f.modalTransitions, [true, false])
  } finally { f.stop() }
})

test('updates cannot be repeated or dismissed while busy and stay paused offline', async () => {
  const f = fixture()
  try {
    await f.mount()
    let dialogFocused = false
    f.prompt.promptDialog.value = { focus() { dialogFocused = true } }
    await f.prompt.confirmDialog()
    await Vue.nextTick()
    assert.equal(dialogFocused, true, 'disabled actions must not send focus behind the modal')
    await f.prompt.confirmDialog()
    f.prompt.closeDialog()
    assert.equal(f.calls.update, 1)
    assert.equal(f.state.updateDismissed, false)
    assert.equal(f.prompt.dialogBusy.value, true)
    f.state.updating = false
    f.state.isOnline = false
    assert.equal(f.prompt.dialogHint.value, 'pwa.updateOffline')
    await f.prompt.confirmDialog()
    assert.equal(f.calls.update, 1)
    assert.equal(f.prompt.showDialog.value, true)
    f.prompt.closeDialog()
    await Vue.nextTick()
    assert.equal(f.prompt.showDialog.value, false)
    assert.equal(f.prompt.showStatusIsland.value, true)
    assert.equal(f.calls.shown, 0, 'offline installs must not count as visible offers')
  } finally { f.stop() }
})

test('update errors use localized recovery copy rather than registration failure copy', async () => {
  const f = fixture()
  try {
    await f.mount()
    f.state.updateError = true
    const reasons: Array<[PwaUpdateFailureReason, string]> = [
      ['activation-timeout', 'pwa.updateActivationFailed'],
      ['controller-unavailable', 'pwa.updateActivationFailed'],
      ['post-message-failed', 'pwa.updateActivationFailed'],
      ['update-check-timeout', 'pwa.updateTimedOut'],
      ['registration-unavailable', 'pwa.updateRegistrationFailed'],
      ['worker-redundant', 'pwa.updateFailed'],
      ['update-check-failed', 'pwa.updateFailed'],
    ]
    for (const locale of ['zh-CN', 'en'] as const) {
      f.i18n.global.locale.value = locale
      for (const [reason, key] of reasons) {
        f.state.updateFailure = reason
        assert.equal(f.prompt.dialogHint.value, key)
        assert.ok(f.i18n.global.te(key))
        assert.notEqual(f.i18n.global.t(key), f.i18n.global.t('pwa.registrationFailed'))
        assert.equal(f.prompt.dialogCopy.value.action, 'pwa.retry')
      }
      for (const item of f.prompt.dialogBenefits.value) {
        assert.ok(f.i18n.global.te(item.title))
        assert.ok(f.i18n.global.te(item.description))
      }
    }
    f.route.name = 'trade'
    await Vue.nextTick()
    assert.equal(f.prompt.showDialog.value, false)
  } finally { f.stop() }
})

test('native installation and registration retries remain guarded, and iOS focuses its hint', async () => {
  const f = fixture()
  try {
    f.state.needRefresh = false
    await f.mount()
    const install = f.prompt.confirmDialog()
    await f.prompt.confirmDialog()
    f.prompt.closeDialog()
    assert.equal(f.calls.install, 1)
    assert.equal(f.state.installAvailable, true)
    f.finishInstall()
    await install
    f.state.installAvailable = false
    f.state.iosInstallAvailable = true
    let focused = false
    f.prompt.installHint.value = { focus() { focused = true } }
    await f.prompt.confirmDialog()
    assert.equal(focused, true)
    assert.equal(f.calls.install, 1)
    const retry = f.prompt.retry()
    await f.prompt.retry()
    assert.equal(f.calls.retry, 1)
    f.finishRetry()
    await retry
  } finally { f.stop() }
})
