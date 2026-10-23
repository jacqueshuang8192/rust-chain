import { requiredDecimalText } from '../../shared/decimal';
import { requiredSafeInteger } from '../../shared/integer';
export const base = '/admin/api/v1/wallet';
export const networkOptions = [{ value: 'eth', label: 'Ethereum' }, { value: 'base', label: 'Base' }, { value: 'tron', label: 'Tron（TRC20）' }, { value: 'btc', label: 'Bitcoin' }, { value: 'solana', label: 'Solana' }];
export const labels: Record<string, string> = { pending: '待付款', expired: '已过期', review: '待核对', observed: '待确认', credited: '已入账', reversed: '已冲正', ignored: '已排除' };
export const reviewReasons: Record<string, string> = { no_unique_amount_and_time_window: '金额或付款时间无法唯一匹配', additional_transfer_after_payment: '订单已入账后再次收到转账', expired_unpaid_combination_reused: '组合曾有过期未付订单，需核对付款归属', amount_exceeds_wallet_precision: '到账数量超过钱包精度', awaiting_confirmations: '等待确认数达标' };
export interface Scanner { network: string; enabled: boolean; endpoint_configured: boolean; api_key_configured: boolean; confirmations: number; revision: number; last_scan_at: number | null; last_error: string | null }
export interface Asset { id: number; network: string; asset_symbol: string; asset_kind: string; token_address: string; decimals: number; enabled: boolean; revision: number }
export interface SharedAddress { pool_id: number; network: string; address: string; enabled: boolean }
export interface Settings { scanners: Scanner[]; assets: Asset[]; addresses: SharedAddress[] }
export interface Receipt { id: number; network: string; tx_hash: string; event_index: number; asset_kind: string; token_address: string; address: string; amount: string; block_height: number; block_hash: string; block_time: number; confirmations: number; status: string; review_reason: string | null; order_id: number | null; review_note: string | null }
export interface Order { asset_kind: string; token_address: string; id: number; order_no: string; user_id: number; asset_symbol: string; network: string; address: string; amount: string; paid_amount: string | null; fee_amount: string; status: string; created_at: number; expires_at: number; required_confirmations: number; tx_hash: string | null }
export interface Inventory { id: number; network: string; address: string; status: string; memo: string | null; assigned_user_id: number | null }
const row = (value: unknown): Record<string, unknown> => { if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('响应格式无效'); return value as Record<string, unknown>; };
const string = (value: unknown): string => { if (typeof value !== 'string') throw new Error('响应文本无效'); return value; };
const integer = (value: unknown) => { if (typeof value !== 'number') throw new Error('响应数字无效'); return requiredSafeInteger(value, '响应编号'); };
const bool = (value: unknown): boolean => { if (typeof value !== 'boolean') throw new Error('响应开关无效'); return value; };
const money = (value: unknown) => requiredDecimalText(string(value), '币种数量');
const nullableText = (value: unknown) => value === null ? null : string(value);
export function parseSettings(value: unknown): Settings {
  const data = row(value);
  if (!Array.isArray(data.scanners) || !Array.isArray(data.assets) || !Array.isArray(data.addresses)) throw new Error('充值配置响应不完整');
  return {
    scanners: data.scanners.map((value) => { const r = row(value); return { network: string(r.network), enabled: bool(r.enabled), endpoint_configured: bool(r.endpoint_configured), api_key_configured: bool(r.api_key_configured), confirmations: integer(r.confirmations), revision: integer(r.revision), last_scan_at: r.last_scan_at === null ? null : integer(r.last_scan_at), last_error: nullableText(r.last_error) }; }),
    assets: data.assets.map((value) => { const r = row(value); return { id: integer(r.id), network: string(r.network), asset_symbol: string(r.asset_symbol), asset_kind: string(r.asset_kind), token_address: string(r.token_address), decimals: integer(r.decimals), enabled: bool(r.enabled), revision: integer(r.revision) }; }),
    addresses: data.addresses.map((value) => { const r = row(value); return { pool_id: integer(r.pool_id), network: string(r.network), address: string(r.address), enabled: bool(r.enabled) }; })
  };
}
export function parseReceipt(value: unknown): Receipt {
  const r = row(value); return { id: integer(r.id), network: string(r.network), tx_hash: string(r.tx_hash), event_index: integer(r.event_index), asset_kind: string(r.asset_kind), token_address: string(r.token_address), address: string(r.address), amount: money(r.amount), block_height: integer(r.block_height), block_hash: string(r.block_hash), block_time: integer(r.block_time), confirmations: integer(r.confirmations), status: string(r.status), review_reason: nullableText(r.review_reason), order_id: r.order_id === null ? null : integer(r.order_id), review_note: nullableText(r.review_note) };
}
export function parseOrder(value: unknown): Order {
  const r = row(value); return { asset_kind: string(r.asset_kind), token_address: string(r.token_address), id: integer(r.id), order_no: string(r.order_no), user_id: integer(r.user_id), asset_symbol: string(r.asset_symbol), network: string(r.network), address: string(r.address), amount: money(r.amount), paid_amount: r.paid_amount === null ? null : money(r.paid_amount), fee_amount: money(r.fee_amount), status: string(r.status), created_at: integer(r.created_at), expires_at: integer(r.expires_at), required_confirmations: integer(r.required_confirmations), tx_hash: nullableText(r.tx_hash) };
}
export function parsePage<T>(value: unknown, parse: (value: unknown) => T): { items: T[]; total: number } {
  const r = row(value); if (!Array.isArray(r.items)) throw new Error('充值列表响应无效'); return { items: r.items.map(parse), total: integer(r.total) };
}
export function parseInventory(value: unknown): Inventory {
  const r = row(value); return { id: integer(r.id), network: string(r.network), address: string(r.address), status: string(r.status), memo: nullableText(r.memo), assigned_user_id: r.assigned_user_id === null ? null : integer(r.assigned_user_id) };
}
