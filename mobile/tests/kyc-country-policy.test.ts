import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'
import { parse } from 'vue/compiler-sfc'

type Country = { country: string }
const sources = [
  { name: 'mobile', path: new URL('../src/views/KycView.vue', import.meta.url), output: 'countryOptions' },
  { name: 'pc', path: new URL('../../pc/src/views/User/KYC.vue', import.meta.url), output: 'availableCountryOptions' },
]

// Execute the real derived-state expressions; rendering/search are covered separately.
function options(source: typeof sources[number], allowed: string[], rules: Country[]): string[] {
  const content = parse(readFileSync(source.path, 'utf8')).descriptor.scriptSetup?.content || ''
  const ast = ts.createSourceFile('policy.ts', content, ts.ScriptTarget.Latest, true)
  const nodes = ast.statements.filter((node) =>
    (ts.isFunctionDeclaration(node) && node.name?.text === 'uniqueValues') ||
    (ts.isVariableStatement(node) && node.declarationList.declarations.some((item) =>
      ['configuredCountries', 'hasCountryRestrictions', source.output].includes(item.name.getText(ast)),
    )),
  )
  assert.equal(nodes.length, 4)
  const code = ts.transpileModule(nodes.map((node) => node.getText(ast)).join(';'), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } }).outputText
  const directory = [{ code: 'CN', name: 'China' }, { code: 'US', name: 'United States' }]
  const match = (value: string, item: { code: string; name: string }) => [item.code, item.name].some((name) => name.toLowerCase() === value.toLowerCase())
  const context = {
    computed: (fn: () => unknown) => ({ get value() { return fn() } }),
    kyc: { value: { config: { allowedCountries: allowed, countryDocumentTypes: rules } } },
    kycConfig: { value: { allowed_countries: allowed, country_document_types: rules } },
    countries: { value: directory }, countryOptions: { value: directory }, fallbackCountryOptions: directory,
    matchesCountry: match, countryMatches: match,
    kycCountryOption: (value: string, item?: { code: string; name: string }) => ({ ...item, value }),
  }
  const value = runInNewContext(code + ';' + source.output + '.value.map(item => item.value)', context)
  return Array.from(value)
}

for (const source of sources) {
  test(`${source.name}: intersect rules and allowlist, preserving configured identity`, () => {
    assert.deepEqual(options(source, ['cn'], [{ country: 'CN' }, { country: 'US' }]), ['CN'])
    assert.deepEqual(options(source, ['China'], [{ country: 'China' }]), ['China'])
  })
  test(`${source.name}: an empty intersection never falls back to all countries`, () => {
    assert.deepEqual(options(source, ['CN'], [{ country: 'US' }]), [])
    assert.deepEqual(options(source, ['CN'], [{ country: 'China' }]), [])
  })
  test(`${source.name}: empty restrictions preserve the existing default behavior`, () => {
    assert.deepEqual(options(source, [], [{ country: 'US' }]), ['US'])
    assert.deepEqual(options(source, ['CN'], []), ['CN'])
    assert.deepEqual(options(source, [], []), ['China', 'United States'])
  })
}
