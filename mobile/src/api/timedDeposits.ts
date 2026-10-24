import { client, requestUrl } from './client'
import { mapDepositOption, mapDepositOrder, mapDepositPage, validateDepositDraft, type DepositOption } from '../core/timedDeposit'
export async function fetchTimedDepositOptions(signal?: AbortSignal) {
  const { data } = await client.get(requestUrl('/wallet/deposit-options'), { signal })
  if (!Array.isArray(data)) throw new TypeError('Invalid deposit options')
  return data.map(mapDepositOption)
}
export async function fetchTimedDepositOrders(asset: string, network: string, offset = 0, signal?: AbortSignal) {
  const { data } = await client.get(requestUrl('/wallet/deposit-orders'), { signal, params: { asset_symbol: asset || undefined, network: network || undefined, limit: 20, offset } })
  return mapDepositPage(data)
}
export async function fetchTimedDepositOrder(id: number, signal?: AbortSignal) {
  if (!Number.isSafeInteger(id) || id <= 0) throw new TypeError('Invalid deposit order ID')
  const { data } = await client.get(requestUrl(`/wallet/deposit-orders/${id}`), { signal })
  return mapDepositOrder(data)
}
export async function createTimedDepositOrder(option: DepositOption, draft: string, key: string, signal?: AbortSignal) {
  const amount = validateDepositDraft(draft, option)
  const { data } = await client.post(requestUrl('/wallet/deposit-orders'), { asset_symbol: option.asset_symbol, network: option.network, amount, idempotency_key: key }, { signal })
  return mapDepositOrder(data)
}

export async function cancelTimedDepositOrder(id: number, signal?: AbortSignal) {
  if (!Number.isSafeInteger(id) || id <= 0) throw new TypeError('Invalid deposit order ID')
  const { data } = await client.post(requestUrl(`/wallet/deposit-orders/${id}/cancel`), undefined, { signal })
  return mapDepositOrder(data)
}
