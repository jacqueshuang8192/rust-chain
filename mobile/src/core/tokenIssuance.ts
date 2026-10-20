import numeral from 'numeral/src/numeral.js'

/** Versioned issuance transport: amounts stay decimal text; missing evidence never becomes zero. */
export type TokenLock = { kind: 'none' | 'on_purchase' | 'on_listing' } | { kind: 'weeks' | 'months'; count: number } | { kind: 'fixed'; at: number }
export type TokenFee = { kind: 'none' } | { kind: 'profit' | 'gross'; rate: string } | { kind: 'fixed'; amount: string }
export type TokenNode = { automatic: boolean; at: number }
export interface TokenPolicy {
  version: 2; timezone: string; purchase_initial_price: string; listing_reference_price: string
  subscription: TokenNode; distribution: TokenNode; purchase: TokenNode; listing: TokenNode
  lock: TokenLock; fee: TokenFee
}
export interface TokenHolding {
  holding: { id: number; project_id: number; quantity: string; cost: string; status: 'held' | 'sold'; acquired_at: number; sold_at: number | null; policy_json: TokenPolicy }
  symbol: string; quote_symbol: string; pair_id: number; pair_symbol: string | null; lifecycle_status: string
  eligible: boolean; unlock_at: number | null; market_price: string | null; market_value: string | null; profit: string | null
}
export type TokenQuoteIntent = { side: 'buy'; quantity: string; idempotency_key: string } | { side: 'sell'; holding_id: number; idempotency_key: string }
export interface TokenQuote {
  id: string; project_id: number; side: 'buy' | 'sell'; holding_id: number | null
  quantity: string; price: string; gross: string; cost: string; fee: string; net: string
  observed_at: number; expires_at: number; status: 'quoted' | 'executed'; receipt_json: TokenReceipt | null
}
export interface TokenReceipt {
  quote_id: string; project_id: number; side: 'buy' | 'sell'; quantity: string; price: string
  gross: string; fee: string; net: string; holding_id: number; executed_at: number
}
export function tokenObject(raw: unknown): Record<string, unknown> {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) throw new Error('Invalid issuance response')
  return raw as Record<string, unknown>
}
export function tokenDecimal(raw: unknown, negative = false): string {
  if (typeof raw !== 'string' || !(negative ? /^-?\d{1,20}(?:\.\d{1,18})?$/ : /^\d{1,20}(?:\.\d{1,18})?$/).test(raw)) throw new Error('Invalid issuance amount')
  return raw
}
function text(raw: unknown): string { if (typeof raw !== 'string' || !raw.trim()) throw new Error('Invalid issuance text'); return raw }
function id(raw: unknown): number { if (typeof raw !== 'number' || !Number.isSafeInteger(raw) || raw <= 0) throw new Error('Invalid issuance identifier'); return raw }
function time(raw: unknown): number { const v = id(raw); if (v > 8_640_000_000_000_000) throw new Error('Invalid issuance time'); return v }
function bool(raw: unknown): boolean { if (typeof raw !== 'boolean') throw new Error('Invalid issuance flag'); return raw }
export function parseTokenPolicy(raw: unknown): TokenPolicy {
  const p = tokenObject(raw)
  if (p.version !== 2) throw new Error('Unsupported issuance policy')
  new Intl.DateTimeFormat('en', { timeZone: text(p.timezone) }).format()
  tokenDecimal(p.purchase_initial_price); tokenDecimal(p.listing_reference_price)
  let previous = 0
  for (const key of ['subscription', 'distribution', 'purchase', 'listing']) {
    const n = tokenObject(p[key]); bool(n.automatic); const at = time(n.at)
    if (at <= previous) throw new Error('Invalid issuance schedule'); previous = at
  }
  const lock = tokenObject(p.lock); const fee = tokenObject(p.fee)
  if (!['none', 'on_purchase', 'on_listing', 'weeks', 'months', 'fixed'].includes(text(lock.kind))) throw new Error('Invalid lock')
  if (lock.kind === 'weeks' || lock.kind === 'months') id(lock.count)
  if (lock.kind === 'fixed') time(lock.at)
  if (!['none', 'profit', 'gross', 'fixed'].includes(text(fee.kind))) throw new Error('Invalid fee')
  if (fee.kind === 'fixed') tokenDecimal(fee.amount)
  else if (fee.kind !== 'none') tokenDecimal(fee.rate)
  return p as unknown as TokenPolicy
}
export function parseTokenHolding(raw: unknown): TokenHolding {
  const r = tokenObject(raw); const h = tokenObject(r.holding)
  id(h.id); id(h.project_id); tokenDecimal(h.quantity); tokenDecimal(h.cost); time(h.acquired_at)
  if (!['held', 'sold'].includes(text(h.status))) throw new Error('Invalid holding status')
  if (h.sold_at !== null) time(h.sold_at)
  parseTokenPolicy(h.policy_json); text(r.symbol); text(r.quote_symbol); id(r.pair_id); text(r.lifecycle_status); bool(r.eligible)
  if (r.pair_symbol !== null) text(r.pair_symbol)
  if (r.unlock_at !== null) time(r.unlock_at)
  for (const key of ['market_price', 'market_value', 'profit']) if (r[key] !== null) tokenDecimal(r[key], key === 'profit')
  return r as unknown as TokenHolding
}
export function parseTokenReceipt(raw: unknown, quote?: TokenQuote): TokenReceipt {
  const r = tokenObject(raw); text(r.quote_id); id(r.project_id); id(r.holding_id); time(r.executed_at)
  if (!['buy', 'sell'].includes(text(r.side))) throw new Error('Invalid receipt side')
  for (const key of ['quantity', 'price', 'gross', 'fee', 'net']) tokenDecimal(r[key])
  if (quote && ['project_id', 'side', 'quantity', 'price', 'gross', 'fee', 'net'].some(key => String(r[key]) !== String(tokenObject(quote)[key]))) throw new Error('Receipt does not match quote')
  if (quote?.side === 'sell' && r.holding_id !== quote.holding_id) throw new Error('Receipt holding mismatch')
  if (quote && r.quote_id !== quote.id) throw new Error('Receipt identity mismatch')
  return r as unknown as TokenReceipt
}
export function parseTokenQuote(raw: unknown, project: number, intent: TokenQuoteIntent): TokenQuote {
  const r = tokenObject(raw); text(r.id); id(r.project_id); time(r.observed_at); time(r.expires_at)
  if (r.project_id !== project || r.side !== intent.side) throw new Error('Quote identity mismatch')
  if (intent.side === 'sell' && r.holding_id !== intent.holding_id) throw new Error('Holding identity mismatch')
  for (const key of ['quantity', 'price', 'gross', 'cost', 'fee', 'net']) tokenDecimal(r[key])
  if (intent.side === 'buy' && canonical(r.quantity as string) !== canonical(intent.quantity)) throw new Error('Quote quantity mismatch')
  if (!['quoted', 'executed'].includes(text(r.status))) throw new Error('Invalid quote status')
  const result = r as unknown as TokenQuote
  if (r.receipt_json !== null) parseTokenReceipt(r.receipt_json, result)
  return result
}
function canonical(s: string): string { const [whole, fraction = ''] = s.split('.'); return `${BigInt(whole)}.${fraction.replace(/0+$/, '')}` }
/** Display-only return ratio. Decimal integer arithmetic prevents settlement precision loss. */
export function tokenReturn(profit: string | null, cost: string): string | null {
  if (profit === null) return null
  const scaled = (s: string) => { const negative = s.startsWith('-'); const [a, b = ''] = s.replace(/^-/, '').split('.'); return BigInt(a + b.padEnd(18, '0').slice(0, 18)) * (negative ? -1n : 1n) }
  const c = scaled(cost); if (!c) return null
  const value = scaled(profit) * 10_000n / c; const abs = value < 0n ? -value : value
  return `${value < 0n ? '-' : ''}${abs / 100n}.${String(abs % 100n).padStart(2, '0')}%`
}
export function tokenMultiply(a: string, b: string): string {
  tokenDecimal(a); tokenDecimal(b)
  const integer = (s: string) => { const [whole, fraction = ''] = s.split('.'); return { value: BigInt(whole + fraction), scale: fraction.length } }
  const x = integer(a); const y = integer(b); const scale = x.scale + y.scale
  const amount = x.value * y.value; const padded = amount.toString().padStart(scale + 1, '0')
  return scale ? `${padded.slice(0, -scale)}.${padded.slice(-scale)}`.replace(/\.?0+$/, '') || '0' : padded
}

/** Subscription creation returns a compact replay receipt, not a history row. */
export function parseTokenSubscriptionReceipt(raw: unknown, key: string) {
  const r = tokenObject(raw)
  if (r.idempotency_key !== key || !['pending', 'allocated', 'partial_allocated', 'refunded'].includes(String(r.status)) || r.lock_position_id !== null) throw new Error('Subscription receipt mismatch')
  return r
}

export type TokenPurchaseIntent = { quantity: string; idempotency_key: string }
/** A direct buy receipt must identify this exact request, without accepting a client price. */
export function parseTokenPurchase(raw: unknown, project: number, intent: TokenPurchaseIntent): TokenReceipt {
  const r = tokenObject(raw)
  if (r.idempotency_key !== intent.idempotency_key) throw new Error('Purchase identity mismatch')
  const receipt = parseTokenReceipt(r.receipt)
  if (receipt.project_id !== project || receipt.side !== 'buy' || canonical(receipt.quantity) !== canonical(tokenDecimal(intent.quantity))) throw new Error('Purchase intent mismatch')
  if (!/[1-9]/.test(receipt.price) || !/[1-9]/.test(receipt.gross) || canonical(receipt.fee) !== canonical('0') || canonical(receipt.net) !== canonical(receipt.gross)) throw new Error('Invalid purchase settlement')
  return receipt
}

/** Display only: Numeral groups safe integers; fractional digits never enter Number. */
export function formatTokenAmount(value: string | null, minimumFractionDigits = 0): string {
  if (value === null || value.length > 128 || !/^-?\d+(?:\.\d+)?$/.test(value)) return '—'
  const [rawWhole = '', rawFraction = ''] = value.replace(/^-/, '').split('.')
  const whole = rawWhole.replace(/^0+(?=\d)/, '')
  const fraction = rawFraction.replace(/0+$/, '').padEnd(minimumFractionDigits, '0')
  const grouped = BigInt(whole) <= BigInt(Number.MAX_SAFE_INTEGER)
    ? numeral(Number(whole)).format('0,0')
    : whole.replace(/\B(?=(\d{3})+(?!\d))/g, ',')
  const sign = value.startsWith('-') && /[1-9]/.test(whole + fraction) ? '-' : ''
  return `${sign}${grouped}${fraction ? `.${fraction}` : ''}`
}

export function formatTokenReturn(profit: string | null, cost: string): string {
  const result = tokenReturn(profit, cost)
  return result === null ? '—' : `${formatTokenAmount(result.slice(0, -1), 2)}%`
}
