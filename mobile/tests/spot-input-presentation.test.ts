import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import ts from 'typescript'
import { computed, ref } from 'vue'
import { baseQuantityFromQuoteAmount, quoteAmountFromBaseQuantity, createTradeFinancialPresentation } from '../src/core/tradeFinancial.ts'
import { financialAssetDisplayDigits } from '../src/core/financialDisplay.ts'

function harness() {
  const source = readFileSync(new URL('../src/views/TradeView.vue', import.meta.url), 'utf8')
  const code = source.slice(source.indexOf('const amountValue = computed('), source.indexOf('function spotDraft()'))
  const quantity = ref('0.540055112180188796'), orderPrice = ref('83141.24'), quoteAsset = ref('USDT')
  let clears = 0
  const bindings = { computed, ref, quantity, orderPrice, quoteAsset, financialAssetDisplayDigits,
    quoteFromBase: quoteAmountFromBaseQuantity, baseFromQuote: baseQuantityFromQuoteAmount,
    moneyText: createTradeFinancialPresentation({locale: () => 'en-US', translate: key => key}).formatValue,
    clearPercentageSelection: () => { clears++ },
  }
  const output = ts.transpileModule(`${code}\nreturn {spotQuantityInput,spotAmountInput,spotQuantityEditing,spotAmountEditing,beginSpotAmountEditing,amountValue}`, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText
  const state = new Function(...Object.keys(bindings), output)(...Object.values(bindings))
  return {state, quantity, quoteAsset, clears: () => clears}
}
test('spot idle fields shorten screenshot values while focus and blur preserve exact order quantity', () => {
  const h = harness(), s = h.state, raw = h.quantity.value
  assert.equal(s.spotQuantityInput.value, '0.54005511')
  assert.equal(s.spotAmountInput.value, '44,900.85')
  s.spotQuantityEditing.value = true
  assert.equal(s.spotQuantityInput.value, raw)
  s.spotQuantityEditing.value = false
  s.beginSpotAmountEditing()
  assert.equal(s.spotAmountInput.value, s.amountValue.value)
  s.spotAmountEditing.value = false
  assert.equal(h.quantity.value, raw)
  assert.equal(h.clears(), 0)
})
test('spot quote editing keeps typed draft stable and submits exact conversion, including huge and tiny values', () => {
  const h=harness(),s=h.state
  s.beginSpotAmountEditing()
  s.spotAmountInput.value='100.'
  assert.equal(s.spotAmountInput.value,'100.')
  s.spotAmountInput.value='100.50'
  assert.equal(s.spotAmountInput.value,'100.50')
  assert.equal(h.quantity.value,baseQuantityFromQuoteAmount('100.50','83141.24',18))
  const raw=h.quantity.value
  s.spotAmountEditing.value=false
  assert.equal(h.quantity.value,raw)
  assert.equal(s.spotAmountInput.value,'100.5')
  h.quantity.value='0.000000000000000001'
  assert.equal(s.spotQuantityInput.value,'<0.00000001')
  h.quantity.value='9007199254740993.125'
  assert.equal(s.spotQuantityInput.value,'9,007,199,254,740,993.125')
})
