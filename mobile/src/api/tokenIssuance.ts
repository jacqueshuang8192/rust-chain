import { client, requestUrl } from './client'
import { parseTokenPurchase, type TokenPurchaseIntent, parseTokenSubscriptionReceipt, parseTokenHolding, parseTokenQuote, parseTokenReceipt, tokenObject, type TokenQuote, type TokenQuoteIntent } from '@/core/tokenIssuance'
export async function fetchTokenHoldings(project?: number, before?: number) {
  const r = tokenObject((await client.get(requestUrl('/new-coins/holdings'), { params: { project_id: project, before_id: before } })).data)
  if (!Array.isArray(r.holdings)) throw new Error('Invalid holdings response')
  if (r.next_before_id !== null && (!Number.isSafeInteger(r.next_before_id) || Number(r.next_before_id) <= 0)) throw new Error('Invalid holding cursor')
  return { holdings: r.holdings.map(parseTokenHolding), next: r.next_before_id as number | null }
}
export async function quoteToken(symbol: string, project: number, intent: TokenQuoteIntent) {
  return parseTokenQuote((await client.post(requestUrl(`/new-coins/${encodeURIComponent(symbol)}/quotes`), intent)).data, project, intent)
}
export async function confirmToken(q: TokenQuote) {
  return parseTokenReceipt((await client.post(requestUrl(`/new-coins/quotes/${encodeURIComponent(q.id)}/confirm`), {})).data, q)
}
export type TokenSubscriptionIntent = { quote_asset_id: number; quote_amount: string; quantity: string; idempotency_key: string }
export async function subscribeToken(symbol: string, intent: TokenSubscriptionIntent) {
  const r = tokenObject((await client.post(requestUrl(`/new-coins/${encodeURIComponent(symbol)}/subscriptions`), intent)).data)
  return parseTokenSubscriptionReceipt(r, intent.idempotency_key)
}

export async function purchaseToken(symbol: string, project: number, intent: TokenPurchaseIntent) {
  return parseTokenPurchase((await client.post(requestUrl(`/new-coins/${encodeURIComponent(symbol)}/market-purchases`), intent)).data, project, intent)
}
