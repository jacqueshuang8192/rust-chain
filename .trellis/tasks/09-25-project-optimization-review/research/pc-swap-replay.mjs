import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from '../../../../web/node_modules/typescript/lib/typescript.js';

const source = readFileSync('pc/src/views/Swap.vue', 'utf8');
const script = source.match(/<script setup lang="ts">([\s\S]*?)<\/script>/)?.[1];
assert.ok(script, 'Vue setup script must exist');
const ast = ts.createSourceFile('Swap.ts', script, ts.ScriptTarget.Latest, true);
const handler = ast.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === 'handleSwap');
assert.ok(handler, 'Current handleSwap implementation must exist');
assert.ok(script.includes('Number(fromAmount.value)'), 'Amount ingress must still use the inspected Number conversion');
const handlerJs = ts.transpileModule(handler.getText(ast), { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText;

const fromAmount = { value: '1.000000000000000001' };
const quote = { value: { quoteId: 'displayed-quote' } };
const loading = { value: false };
const submitted = [];
const committed = new Set();
const requestedAmounts = [];
let quoteCount = 0;
const context = vm.createContext({
  isLoggedIn: { value: true },
  goToLogin() { throw new Error('Unexpected login redirect'); },
  selectedPair: { value: { fromUnit: 'A', toUnit: 'B' } },
  validationMessage: { value: '' },
  amountNumber: { get value() { return Number(fromAmount.value); } },
  fromAmount, quote, loading,
  t: value => value,
  errorMessage: error => error.message,
  toast: { error() {}, success() {} },
  async refreshBalances() {},
  async requestSwapQuote(_pair, amount) {
    requestedAmounts.push(amount);
    return { data: { data: { quoteId: `new-quote-${++quoteCount}` } } };
  },
  async confirmSwapQuote(id) {
    submitted.push(id);
    committed.add(id);
    if (submitted.length === 1) throw new Error('Simulated response loss after commit');
  },
});
vm.runInContext(`${handlerJs}
globalThis.replay = handleSwap;`, context);
await context.replay();
const afterLoss = { amount: fromAmount.value, displayedQuote: quote.value.quoteId, loading: loading.value };
assert.equal(afterLoss.amount, '1.000000000000000001');
assert.equal(afterLoss.loading, false);
assert.equal(afterLoss.displayedQuote, 'displayed-quote');
await context.replay();
assert.deepEqual(submitted, ['new-quote-1', 'new-quote-2']);
assert.equal(committed.size, 2);
assert.ok(!submitted.includes('displayed-quote'));
assert.equal(requestedAmounts[0], 1);
console.log(JSON.stringify({ afterLoss, submitted, modeledCommits: committed.size, requestedAmounts, networkRequests: 0 }, null, 2));
console.log('Current handler reproduces new-quote retry and display mismatch; APIs are memory fixtures, not a backend integration test.');
