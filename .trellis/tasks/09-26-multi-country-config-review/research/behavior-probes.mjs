import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

const require = createRequire(resolve('web/package.json'));
const ts = require('typescript');

// Execute current source nodes with isolated dependencies, without network calls.
function sourceNode(path, name, variable = false) {
  let text = readFileSync(path, 'utf8');
  if (path.endsWith('.vue')) text = text.split('<script setup lang="ts">')[1].split('</script>')[0];
  const source = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  for (const node of source.statements) {
    if (!variable && ts.isFunctionDeclaration(node) && node.name?.text === name) return node.getText(source);
    if (variable && ts.isVariableStatement(node) && node.declarationList.declarations.some((d) => d.name.getText(source) === name)) return node.getText(source);
  }
  throw new Error(`Source node missing: ${path}:${name}`);
}

function load(path, name, context = {}, variable = false) {
  const code = ts.transpileModule(sourceNode(path, name, variable), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } }).outputText;
  return runInNewContext(code + ';' + name, { exports: {}, ...context });
}

test('existing loan content is relabeled when country default changes', () => {
  const sync = load('web/src/admin/resources/actions/loan.tsx', 'syncLoanProductCountryLocales');
  const result = sync({ name: 'Existing Chinese text', names: [{ country: 'CN', locale: 'zh', title: 'Existing Chinese text' }] }, [{ countryCode: 'CN', defaultLocale: 'en' }]);
  assert.equal(result.names[0].locale, 'en');
  assert.equal(result.names[0].title, 'Existing Chinese text');
});

test('KYC rule countries override rather than intersect allowed countries', () => {
  const result = load('mobile/src/views/KycView.vue', 'configuredCountries', {
    computed: (fn) => ({ value: fn() }), uniqueValues: (values) => [...new Set(values)],
    kyc: { value: { config: { allowedCountries: ['CN'], countryDocumentTypes: [{ country: 'US' }] } } },
  }, true);
  assert.deepEqual(Array.from(result.value), ['US']);
});

test('mobile country mapping drops server default and supported locales', async () => {
  const fetch = load('mobile/src/api/auth.ts', 'fetchCountries', {
    requestUrl: (url) => url, createReferenceRequestKey: (url) => url,
    referenceRequestRegistry: { request: (_key, _ttl, fn) => fn() },
    client: { get: async () => ({ data: { countries: [{ country_code: 'US', country_name: 'United States', default_locale: 'en', supported_locales: ['en'] }] } }) },
  });
  const result = await fetch();
  assert.deepEqual(Object.keys(result[0]).sort(), ['code', 'name']);
});

test('mobile loan adapter ignores available English product translation', async () => {
  const fetch = load('mobile/src/api/loan.ts', 'fetchLoanProducts', {
    requiredSafeInteger: (value) => value, requiredId: (value) => value, productDecimal: (value) => value,
    requestUrl: (url) => url, createReferenceRequestKey: (url) => url,
    referenceRequestRegistry: { request: (_key, _ttl, fn) => fn() },
    i18n: { global: { locale: { value: 'en' }, t: (key) => key } },
    client: { get: async () => ({ data: { products: [{ id: 1, asset_id: 1, name: 'Default title', name_json: { default_locale: 'zh', items: [{ locale: 'en', title: 'English title' }] }, term_days: 30, interest_rate: '0.01', min_kyc_level: 0, min_amount: '1' }] } }) },
  });
  assert.equal((await fetch())[0].name, 'Default title');
});

test('mobile news requests omit regional targeting', async () => {
  let params;
  const fetch = load('mobile/src/api/news.ts', 'fetchNews', {
    requiredSafeInteger: (value) => value, currentApiLocale: () => 'en-US', requestUrl: (url) => url,
    client: { get: async (_url, options) => { params = options.params; return { data: { news: [] } }; } },
  });
  await fetch();
  assert.equal(params.locale, 'en-US');
  assert.equal(Object.hasOwn(params, 'country_code'), false);
});
