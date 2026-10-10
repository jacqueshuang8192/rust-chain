import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'
import { compileScript, parse } from '@vue/compiler-sfc'
import ts from 'typescript'
import * as Vue from 'vue'
import * as VueI18n from 'vue-i18n'
import { renderToString } from 'vue/server-renderer'
import { marketSource, type MarketProvenance } from '../src/core/marketProvenance.ts'
import zh from '../src/i18n/messages/zh-CN.ts'
import en from '../src/i18n/messages/en.ts'

const source = readFileSync(new URL('../src/components/MarketProvenanceLabel.vue', import.meta.url), 'utf8')
const { descriptor } = parse(source)
const compiled = compileScript(descriptor, { id: 'provenance-label-test', inlineTemplate: true })
const output = ts.transpileModule(compiled.content, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
}).outputText
const modules: Record<string, unknown> = {
  vue: Vue,
  'vue-i18n': VueI18n,
  '@/core/marketProvenance': { marketSource },
}
const exports: { default?: Vue.Component } = {}
new Function('require', 'exports', output)((name: string) => {
  assert.ok(Object.hasOwn(modules, name), `Unexpected component import: ${name}`)
  return modules[name]
}, exports)
assert.ok(exports.default)
const component = exports.default

async function renderLabel(provenance: MarketProvenance, locale: 'zh-CN' | 'en', extra = {}) {
  const app = Vue.createSSRApp(component, { provenance, ...extra })
  app.use(VueI18n.createI18n({ legacy: false, locale, messages: { 'zh-CN': zh, en } }))
  return renderToString(app)
}

for (const locale of ['zh-CN', 'en'] as const) {
  test(`${locale}: hidden sources produce no empty label and preserve freshness`, async () => {
    const labels = locale === 'zh-CN' ? zh.marketProvenance : en.marketProvenance
    for (const provenance of [
      { source: 'default', provider: 'strategy' },
      { source: 'external', provider: 'coinbase' },
      { provider: 'htx' },
    ]) {
      const snapshot = { ...provenance }
      assert.doesNotMatch(await renderLabel(provenance, locale), /<small/)
      const live = await renderLabel(provenance, locale, { live: true, observedAt: Date.now() - 120_000 })
      assert.match(live, /<small/)
      assert.ok(live.includes(labels.stale))
      assert.ok(!live.includes(labels.default))
      assert.ok(!live.includes(labels.external))
      assert.doesNotMatch(live.replace(/<[^>]*>/g, '').trim(), /^·/)
      assert.ok((await renderLabel(provenance, locale, { live: true })).includes(labels.timeUnknown))
      assert.deepEqual(provenance, snapshot)
    }
  })

  test(`${locale}: other sources and contradictory evidence retain their labels`, async () => {
    const labels = locale === 'zh-CN' ? zh.marketProvenance : en.marketProvenance
    for (const source of ['platform', 'strategy', 'generated', 'unknown'] as const) {
      const provider = source === 'platform' ? 'platform' : 'strategy'
      assert.ok((await renderLabel({ source, provider }, locale)).includes(labels[source]))
    }
    assert.ok((await renderLabel({ source: 'default', provider: 'htx' }, locale)).includes(labels.unknown))
    assert.ok((await renderLabel({}, locale)).includes(labels.unknown))
  })
}
