import { apiRequest, ContractError } from './client';
import { decimalFitsStorage } from '../shared/decimal';
import { isUnixMillis, requiredSafeInteger } from '../shared/integer';

export type WalletRecordsScope = 'admin' | 'agent';
export type UserWalletRecord = Record<string, unknown> & {
  id: number; user_id: number; account_type: 'spot' | 'margin'; asset_id: number;
  asset_symbol: string; change_type: string; balance_type: string; amount: string;
  balance_after: string; available_after: string; frozen_after: string; locked_after: string;
  ref_type: string; ref_id: string; created_at: number;
};
export type UserWalletRecordPage = { ledger: UserWalletRecord[]; total: number };

export function parseUserWalletRecords(value: unknown, userId: number, account: string, limit: number, path: string): UserWalletRecordPage {
  const invalid = () => { throw new ContractError('用户资金记录响应格式无效', { path }); };
  if (!value || typeof value !== 'object' || Array.isArray(value)) return invalid();
  const page = value as Record<string, unknown>;
  if (!Array.isArray(page.ledger) || page.ledger.length > limit || typeof page.total !== 'number' || !Number.isSafeInteger(page.total) || page.total < 0) return invalid();
  for (const item of page.ledger) {
    if (!item || typeof item !== 'object' || Array.isArray(item)) return invalid();
    const row = item as Record<string, unknown>;
    if (row.user_id !== userId || row.account_type !== account || !isUnixMillis(row.created_at)) return invalid();
    for (const field of ['id', 'asset_id']) if (typeof row[field] !== 'number' || !Number.isSafeInteger(row[field]) || row[field] <= 0) return invalid();
    for (const field of ['amount', 'balance_after', 'available_after', 'frozen_after', 'locked_after']) if (!decimalFitsStorage(row[field])) return invalid();
    for (const field of ['asset_symbol', 'change_type', 'balance_type', 'ref_type', 'ref_id']) if (typeof row[field] !== 'string') return invalid();
    if (!['available', 'frozen', 'locked'].includes(row.balance_type as string)) return invalid();
  }
  return { ledger: page.ledger as UserWalletRecord[], total: page.total };
}

export async function getUserWalletRecords(scope: WalletRecordsScope, userId: number, filters: Record<string, string>, limit: number, offset: number, signal?: AbortSignal): Promise<UserWalletRecordPage> {
  requiredSafeInteger(userId, '用户ID', 1);
  requiredSafeInteger(limit, '分页大小', 1, 100);
  requiredSafeInteger(offset, '分页偏移', 0, 4_294_967_295);
  const account = filters.account_type || 'spot';
  if (!['spot', 'margin'].includes(account)) throw new Error('请选择现货或杠杆账户');
  const params = new URLSearchParams({ account_type: account, limit: String(limit), offset: String(offset) });
  if (scope === 'admin') params.set('user_id', String(userId));
  if (filters.asset_id) params.set('asset_id', String(requiredSafeInteger(filters.asset_id, '资产ID', 1)));
  for (const field of ['change_type', 'balance_type', 'ref_type', 'ref_id']) {
    const value = filters[field]?.trim();
    if (value && value.length > 128) throw new Error('筛选内容过长');
    if (value) params.set(field, value);
  }
  const path = `${scope === 'admin' ? '/admin/api/v1/wallet/ledger' : `/agent/api/v1/users/${userId}/wallet-ledger`}?${params}`;
  const response = await apiRequest<unknown>(path, { authScope: scope, signal });
  return parseUserWalletRecords(response, userId, account, limit, path);
}
