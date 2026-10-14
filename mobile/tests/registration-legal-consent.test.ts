import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { compileScript, parse } from '@vue/compiler-sfc'
import ts from 'typescript'
import * as Vue from 'vue'
import { createI18n } from 'vue-i18n'
import type { RegistrationLegalBundle } from '../src/api/auth.ts'
import zh from '../src/i18n/messages/zh-CN.ts'
import en from '../src/i18n/messages/en.ts'

type Kind = 'terms' | 'privacy'
const bundle: RegistrationLegalBundle = {
  publication_id: 12, edition: 1, scope_code: 'GLOBAL', locale: 'zh-CN',
  published_at: '2026-10-14T08:00:00Z',
  terms: { id: 21, title: 'Test terms', body: 'Fixture only', effective_on: '2026-10-14' },
  privacy: { id: 22, title: 'Test privacy', body: 'Fixture only', effective_on: '2026-10-14' },
}

interface ConsentState {
  countryCode: Vue.Ref<string>
  acceptedTerms: Vue.Ref<boolean>
  legalSeen: Vue.Ref<Set<Kind>>
  reviewedLegal: Vue.ComputedRef<boolean>
  legalInitial: Vue.Ref<Kind>
  legalDocuments: Vue.Ref<RegistrationLegalBundle | null>
  legalOpen: Vue.Ref<boolean>
  legalError: Vue.Ref<string>
  email: Vue.Ref<string>
  password: Vue.Ref<string>
  confirmation: Vue.Ref<string>
  code: Vue.Ref<string>
  loadLegal(): Promise<void>
  openLegal(kind?: Kind): void
  toggleLegal(event: Event): void
  sendCode(): Promise<void>
  submit(): Promise<void>
}

interface ReaderState {
  selected: Vue.Ref<Kind>
  open: Vue.Ref<boolean>
  copy: Vue.Ref<{ scrollTop: number } | null>
  select(kind: Kind): Promise<void>
  dismiss(confirmed?: boolean): Promise<void>
}

// Execute the actual SFC setup with real Vue reactivity; DOM/focus is browser-tested.
function setupSfc<T>(path: string, modules: Record<string, unknown>, props = {}, emit = (..._args: unknown[]) => {}) {
  const lifecycle: Array<() => unknown> = []
  const { descriptor } = parse(readFileSync(new URL(`../src/${path}.vue`, import.meta.url), 'utf8'))
  const compiled = compileScript(descriptor, { id: 'legal-consent-test' })
  const output = ts.transpileModule(compiled.content, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
  }).outputText
  const bindings = { ...modules, vue: { ...Vue, onMounted: (callback: () => unknown) => lifecycle.push(callback), onUnmounted: () => {} } }
  const exports: { default?: { setup: (props: object, context: object) => T } } = {}
  new Function('require', 'exports', output)((name: string) => {
    assert.ok(Object.hasOwn(bindings, name), `Missing test binding: ${name}`)
    return bindings[name as keyof typeof bindings]
  }, exports)
  assert.ok(exports.default)
  const scope = Vue.effectScope()
  const state = scope.run(() => exports.default!.setup(props, { expose() {}, emit }))!
  return { state, async mount() { for (const callback of lifecycle) await callback(); await Vue.nextTick() }, stop: () => scope.stop() }
}

async function registration() {
  const i18n = createI18n({ legacy: false, locale: 'zh-CN', messages: { 'zh-CN': zh, en } })
  let read = async () => bundle
  const writes: unknown[][] = []
  const fixture = setupSfc<ConsentState>('views/RegisterView', {
    axios: { default: { isAxiosError: () => false } },
    'vue-router': { useRoute: () => ({ query: {} }), useRouter: () => ({}) },
    'vue-i18n': { useI18n: () => i18n.global },
    'lucide-vue-next': {},
    '@/api/client': { apiErrorMessage: (_error: unknown, fallback: string) => fallback },
    '@/api/auth': {
      fetchCountries: async () => [{ code: 'CN', name: 'China' }, { code: 'HK', name: 'Hong Kong' }],
      fetchRegisterConfig: async () => ({ emailCodeRequired: true, inviteCodeRequired: false }),
      fetchRegistrationLegal: () => read(),
      sendRegistrationCode: async (...args: unknown[]) => { writes.push(args); throw new Error('No outbound email in tests') },
      registerWithEmail: async (...args: unknown[]) => { writes.push(args) },
    },
    '@/components/RegistrationLegalDialog.vue': {},
    '@/core/countrySearch': {},
    '@/core/modalDialog': { useModalDialog: () => ({}) },
    '@/core/navigation': { sanitizeInternalRedirect: () => '/', replaceAuthStep: async () => {} },
    '@/stores/session': { useSessionStore: () => ({ sync() {} }) },
    '@/assets/logo.png': {},
  })
  await fixture.mount()
  await Vue.nextTick()
  return { ...fixture, i18n, writes, setRead: (next: typeof read) => { read = next } }
}

function toggle(state: ConsentState, checked: boolean) {
  const input = { checked }
  state.toggleLegal({ target: input } as unknown as Event)
  return input.checked
}

test('unread checkbox opens the first unread document without accepting', async () => {
  const f = await registration()
  try {
    assert.equal(toggle(f.state, true), false)
    assert.equal(f.state.legalOpen.value, true)
    assert.equal(f.state.legalInitial.value, 'terms')
    f.state.legalSeen.value.add('terms')
    f.state.legalOpen.value = false
    assert.equal(toggle(f.state, true), false)
    assert.equal(f.state.legalInitial.value, 'privacy')
    f.state.openLegal('terms')
    assert.equal(f.state.legalInitial.value, 'terms')
    f.state.openLegal('privacy')
    assert.equal(f.state.legalInitial.value, 'privacy')
    assert.equal(f.state.acceptedTerms.value, false)
    assert.deepEqual(f.writes, [])
  } finally { f.stop() }
})

test('viewing both documents only enables explicit acceptance and withdrawal', async () => {
  const f = await registration()
  try {
    f.state.legalSeen.value.add('privacy').add('terms')
    assert.equal(f.state.reviewedLegal.value, true)
    assert.equal(f.state.acceptedTerms.value, false)
    assert.equal(toggle(f.state, true), true)
    assert.equal(toggle(f.state, false), false)
    assert.equal(f.state.legalOpen.value, false)
    assert.deepEqual(f.writes, [])
  } finally { f.stop() }
})

test('country, locale and publication reload each clear consent and viewed documents', async () => {
  const f = await registration()
  try {
    for (const change of [
      () => { f.state.countryCode.value = 'HK' },
      () => { f.i18n.global.locale.value = 'en' },
      () => f.state.loadLegal(),
    ]) {
      f.state.legalSeen.value.add('terms').add('privacy')
      toggle(f.state, true)
      f.state.openLegal()
      await change()
      await Vue.nextTick()
      assert.equal(f.state.acceptedTerms.value, false)
      assert.equal(f.state.legalSeen.value.size, 0)
      assert.equal(f.state.legalOpen.value, false)
    }
  } finally { f.stop() }
})

test('missing documents and late reads cannot restore acceptance', async () => {
  const f = await registration()
  try {
    let resolve!: (value: RegistrationLegalBundle) => void
    f.setRead(() => new Promise((done) => { resolve = done }))
    const pending = f.state.loadLegal()
    assert.equal(f.state.legalDocuments.value, null)
    f.state.openLegal('privacy')
    assert.equal(toggle(f.state, true), false)
    assert.equal(f.state.legalOpen.value, false)
    f.setRead(async () => { throw new Error('Unavailable') })
    await f.state.loadLegal()
    resolve(bundle)
    await pending
    assert.equal(f.state.legalDocuments.value, null)
    assert.equal(f.state.legalError.value, zh.auth.legalUnavailable)
    assert.equal(f.state.acceptedTerms.value, false)
  } finally { f.stop() }
})

test('send code and register require review plus acceptance and retain publication identity', async () => {
  const f = await registration()
  try {
    f.state.email.value = 'test@example.com'
    f.state.password.value = f.state.confirmation.value = 'testing-only-password'
    f.state.code.value = '123456'
    for (const action of [() => f.state.sendCode(), () => f.state.submit()]) {
      await action()
      assert.equal(f.state.legalOpen.value, true)
      assert.deepEqual(f.writes, [])
      f.state.legalOpen.value = false
    }
    f.state.legalSeen.value.add('terms').add('privacy')
    await f.state.submit()
    assert.deepEqual(f.writes, [])
    toggle(f.state, true)
    await f.state.sendCode()
    assert.deepEqual(f.writes[0], ['test@example.com', 'CN', 'zh-CN', 12])
    await f.state.submit()
    assert.deepEqual(f.writes[1], [{ email: 'test@example.com', password: 'testing-only-password', code: '123456', countryCode: 'CN', inviteCode: '', legalLocale: 'zh-CN', legalPublicationId: 12 }])
  } finally { f.stop() }
})

test('reader opens the requested tab, preserves visits across mounts and never confirms on dismissal', async () => {
  const seen = Vue.reactive(new Set<Kind>())
  const events: unknown[][] = []
  const makeReader = (initial: Kind) => setupSfc<ReaderState>('components/RegistrationLegalDialog', {
    'vue-i18n': { useI18n: () => ({ t: (key: string) => key }) },
    'lucide-vue-next': {},
    '@/core/modalDialog': { useModalDialog: () => ({}) },
    '@/core/newsRichText': {},
    '@/components/NewsRichText.vue': {},
  }, { bundle, initial, seen }, (...args) => {
    events.push(args)
    if (args[0] === 'view') seen.add(args[1] as Kind)
  })
  const first = makeReader('privacy')
  await first.mount()
  assert.equal(first.state.selected.value, 'privacy')
  await first.state.dismiss(true)
  assert.equal(first.state.open.value, true)
  await first.state.dismiss()
  first.stop()
  assert.deepEqual(events, [['view', 'privacy'], ['close']])
  const second = makeReader('terms')
  try {
    await second.mount()
    assert.equal(seen.size, 2)
    second.state.copy.value = { scrollTop: 100 }
    await second.state.select('privacy')
    assert.equal(second.state.copy.value.scrollTop, 0)
    await second.state.dismiss(true)
    assert.equal(events.at(-1)?.[0], 'confirm')
    await second.state.dismiss(true)
    assert.equal(events.filter(([event]) => event === 'confirm').length, 1)
  } finally { second.stop() }
})
