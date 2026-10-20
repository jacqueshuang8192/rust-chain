/** Wallet valuation is independent of spot discovery and never grants trading access. */
export interface IssuanceValuation { priceUsdt: string | null; expiresAt: number }
export function parseIssuanceValuation(raw: unknown): IssuanceValuation | undefined {
  if (raw === undefined || raw === null) return undefined
  const unknown = { priceUsdt: null, expiresAt: 0 }
  if (typeof raw !== 'object' || Array.isArray(raw)) return unknown
  const r = raw as Record<string, unknown>
  if (!Number.isSafeInteger(r.expires_at) || Number(r.expires_at) <= 0) return unknown
  if (r.price_usdt === null) return { priceUsdt: null, expiresAt: Number(r.expires_at) }
  if (typeof r.price_usdt !== 'string' || !/^\d{1,20}(?:\.\d{1,18})?$/.test(r.price_usdt) || !/[1-9]/.test(r.price_usdt)) return unknown
  return { priceUsdt: r.price_usdt, expiresAt: Number(r.expires_at) }
}
/** Undefined uses normal market pricing; null means an issuance price is unavailable. */
export function issuancePrice(value: IssuanceValuation | undefined, now: number): string | null | undefined {
  if (value === undefined) return undefined
  return now < value.expiresAt ? value.priceUsdt : null
}
