import { ref, watch, type Ref } from 'vue'
import type { Router } from 'vue-router'
import type { SecondsProduct } from '@/api/seconds'
import { normalizeMarketSocketSymbol } from '@/api/marketSocketProtocol'
import { splitSymbol } from '@/core/format'
import { useNavigationStore } from '@/stores/navigation'

/** Keep URL identity authoritative without remounting the seconds order workspace. */
export function useSecondsProductSelection(options: {
  router: Router
  products: Ref<SecondsProduct[]>
  onSelect: (product: SecondsProduct) => void
}) {
  const navigation = useNavigationStore()
  const selected = ref<SecondsProduct | null>(null)
  const route = options.router.currentRoute
  let pendingChoice: { id: number; from: string } | null = null

  function resolveProduct(): SecondsProduct | null {
    // A refresh can finish before router.replace commits the user's new choice.
    if (pendingChoice?.from === route.value.fullPath) {
      return options.products.value.find((product) => product.id === pendingChoice?.id)
        || options.products.value[0] || null
    }
    const symbol = route.value.params.symbol
    const requested = (typeof symbol === 'string' ? normalizeMarketSocketSymbol(symbol) : '') || 'BTCUSDT'
    return options.products.value.find((product) => (
      normalizeMarketSocketSymbol(product.symbol) === requested
    )) || options.products.value[0] || null
  }

  function rememberProduct(product: SecondsProduct): void {
    navigation.rememberSecondsProduct(product.id)
    const { base, quote } = splitSymbol(product.symbol)
    const symbol = `${base}_${quote}`
    if (route.value.name !== 'seconds' || route.value.params.symbol === symbol) return
    const choice = { id: product.id, from: route.value.fullPath }
    pendingChoice = choice
    void options.router.replace({
      name: 'seconds',
      params: { symbol },
      query: route.value.query,
      hash: route.value.hash,
    }).finally(() => {
      if (pendingChoice === choice) pendingChoice = null
    })
  }

  function restoreProducts(products: SecondsProduct[]): void {
    options.products.value = products
    selected.value = resolveProduct()
    if (selected.value) rememberProduct(selected.value)
  }

  watch(() => route.value.params.symbol, () => {
    if (route.value.name !== 'seconds') return
    const product = resolveProduct()
    if (!product) return
    if (product.id !== selected.value?.id) options.onSelect(product)
    else rememberProduct(product)
  })

  return { selected, restoreProducts, rememberProduct }
}
