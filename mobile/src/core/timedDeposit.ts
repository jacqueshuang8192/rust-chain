/** Exact wire contract for timed on-chain deposits. No monetary Number round trips. */
export interface DepositOption { asset_symbol: string; network: string; display_name: string; precision_scale: number; min_deposit_amount: string; deposit_fee: string; required_confirmations: number }
export interface DepositOrder { id: number; order_no: string; user_id: number; asset_symbol: string; network: string; address: string; amount: string; fee_amount: string; paid_amount: string | null; required_confirmations: number; status: string; tx_hash: string | null; created_at: number; expires_at: number; paid_at: number | null; server_time: number }
export interface DepositPage { items: DepositOrder[]; total: number }
export function exactDepositAmount(value: unknown): string {
  if (typeof value !== 'string' || value.length > 256) throw new TypeError('Invalid exact deposit amount')
  const match = /^(\d+)(?:\.(\d*))?(?:[eE]([+-]?\d{1,3}))?$/.exec(value.trim())
  if (!match) throw new TypeError('Invalid exact deposit amount')
  const exponent = Number(match[3] || '0')
  if (Math.abs(exponent) > 256) throw new TypeError('Invalid deposit exponent')
  const digits = match[1] + (match[2] || '')
  const split = match[1].length + exponent
  const integer = (split <= 0 ? '0' : digits.slice(0, split).padEnd(split, '0')).replace(/^0+(?=\d)/, '')
  const fraction = (split < 0 ? '0'.repeat(-split) + digits : digits.slice(split)).replace(/0+$/, '')
  if (integer.length > 20 || fraction.length > 18) throw new TypeError('Deposit amount exceeds storage precision')
  return fraction ? `${integer}.${fraction}` : integer
}
export function depositInteger(value: unknown): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0) throw new TypeError('Invalid deposit integer')
  return value
}
function text(value: unknown): string { if (typeof value !== 'string' || !value) throw new TypeError('Missing deposit text'); return value }
function object(value: unknown): Record<string, unknown> { if (!value || typeof value !== 'object' || Array.isArray(value)) throw new TypeError('Invalid deposit response'); return value as Record<string, unknown> }
export function mapDepositOption(value: unknown): DepositOption {
  const row = object(value)
  const scale = depositInteger(row.precision_scale)
  if (scale > 18) throw new TypeError('Invalid deposit precision')
  return { asset_symbol: text(row.asset_symbol), network: text(row.network), display_name: text(row.display_name), precision_scale: scale, min_deposit_amount: exactDepositAmount(row.min_deposit_amount), deposit_fee: exactDepositAmount(row.deposit_fee), required_confirmations: depositInteger(row.required_confirmations) }
}
export function mapDepositOrder(value: unknown): DepositOrder {
  const row = object(value)
  const status = text(row.status)
  if (!['pending','expired','review','credited','reversed'].includes(status)) throw new TypeError('Unknown deposit state')
  return { id: depositInteger(row.id), user_id: depositInteger(row.user_id), order_no: text(row.order_no), asset_symbol: text(row.asset_symbol), network: text(row.network), address: text(row.address), amount: exactDepositAmount(row.amount), fee_amount: exactDepositAmount(row.fee_amount), paid_amount: row.paid_amount === null ? null : exactDepositAmount(row.paid_amount), required_confirmations: depositInteger(row.required_confirmations), status, tx_hash: row.tx_hash === null ? null : text(row.tx_hash), created_at: depositInteger(row.created_at), expires_at: depositInteger(row.expires_at), paid_at: row.paid_at === null ? null : depositInteger(row.paid_at), server_time: depositInteger(row.server_time) }
}
export function mapDepositPage(value: unknown): DepositPage {
  const row = object(value)
  if (!Array.isArray(row.items)) throw new TypeError('Invalid deposit page')
  return { items: row.items.map(mapDepositOrder), total: depositInteger(row.total) }
}
export function validateDepositDraft(value: string, option: DepositOption): string {
  const amount = exactDepositAmount(value)
  const units = (input: string) => { const [whole, fraction = ''] = exactDepositAmount(input).split('.'); return BigInt(whole + fraction.padEnd(18, '0')) }
  if ((amount.split('.')[1]?.length || 0) > option.precision_scale || units(amount) <= 0n || units(amount) < units(option.min_deposit_amount) || units(amount) <= units(option.deposit_fee)) throw new TypeError('Invalid deposit amount for this asset')
  return amount
}
