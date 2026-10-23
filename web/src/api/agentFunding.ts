import { listAgentFinancialRows } from './agent';
import type { ApiRecord } from './types';
import { requiredSafeInteger } from '../shared/integer';

export const fundingKinds = ['deposits', 'withdrawals'] as const;
export type FundingKind = typeof fundingKinds[number];
export const fundingLabels: Record<FundingKind, string> = { deposits: '充值记录', withdrawals: '提现记录' };
export const fundingSources = { quick: '快速充值', chain: '链上充值', withdrawal: '提现' };
export const fundingStatuses: Record<FundingKind, Record<string, string>> = {
  deposits: { created: '已创建', pending: '待付款', expired: '已过期', review: '待核对', credited: '已入账', reversed: '已冲正', failed: '失败' },
  withdrawals: { pending_review: '待审核', approved: '已批准', broadcasting: '广播中', unknown_broadcast: '广播结果待核实', broadcasted: '待链上确认', confirmed: '已完成', rejected: '已拒绝', failed: '失败', manual_review: '人工复核' }
};
export type FundingRecord = ApiRecord & {
  id: number; source: keyof typeof fundingSources; order_no: string | null;
  user_id: number; email: string | null; owner_agent_id: number; owner_agent_code: string;
  asset_symbol: string; network: string | null; address: string | null;
  amount: string | null; paid_amount: string | null; credited_amount: string | null; fee: string;
  total_reserved: string | null; fiat_amount: string | null; fiat_currency: string | null;
  status: string; tx_hash: string | null; created_at: number; expires_at: number | null; completed_at: number | null;
};
export type FundingQuery = {
  team_scope?: string; agent_id?: number; user_id?: number; record_id?: number; email?: string;
  order_no?: string; asset_symbol?: string; network?: string; source?: string; status?: string;
  created_from?: number; created_to?: number; limit?: number; offset?: number;
};

export async function getAgentFunding(kind: FundingKind, query: FundingQuery = {}) {
  if (!fundingKinds.includes(kind)) throw new Error('资金记录类型无效');
  const params = new URLSearchParams();
  for (const field of ['agent_id', 'user_id', 'record_id'] as const) {
    if (query[field] !== undefined) params.set(field, String(requiredSafeInteger(query[field], field, 1)));
  }
  for (const field of ['created_from', 'created_to'] as const) {
    if (query[field] !== undefined) params.set(field, String(requiredSafeInteger(query[field], field, 0, 253_402_300_799_999)));
  }
  if (query.created_from !== undefined && query.created_to !== undefined && query.created_from >= query.created_to) throw new Error('结束时间必须晚于开始时间');
  for (const field of ['team_scope', 'email', 'order_no', 'asset_symbol', 'network', 'source', 'status'] as const) {
    const value = query[field]?.trim(); if (value) params.set(field, value);
  }
  params.set('limit', String(requiredSafeInteger(query.limit ?? 20, '分页大小', 1, 100)));
  params.set('offset', String(requiredSafeInteger(query.offset ?? 0, '分页偏移', 0, 100_000)));
  const result = await listAgentFinancialRows<FundingRecord>(`/agent/api/v1/funding/${kind}?${params}`, 'items', {
    requiredFields: ['id', 'source', 'order_no', 'user_id', 'email', 'owner_agent_id', 'owner_agent_code', 'asset_symbol', 'network', 'address',
      'amount', 'paid_amount', 'credited_amount', 'fee', 'total_reserved', 'fiat_amount', 'fiat_currency', 'status', 'tx_hash', 'created_at', 'expires_at', 'completed_at'],
    integerRanges: { id: [1, Number.MAX_SAFE_INTEGER], user_id: [1, Number.MAX_SAFE_INTEGER], owner_agent_id: [1, Number.MAX_SAFE_INTEGER] },
    stringFields: ['owner_agent_code', 'asset_symbol'],
    nullableStringFields: ['order_no', 'email', 'network', 'address', 'fiat_currency', 'tx_hash'],
    decimalFields: ['fee'], nullableDecimalFields: ['amount', 'paid_amount', 'credited_amount', 'total_reserved', 'fiat_amount'],
    timestampFields: ['created_at'], nullableTimestampFields: ['expires_at', 'completed_at'],
    enumFields: { source: kind === 'deposits' ? ['quick', 'chain'] : ['withdrawal'], status: Object.keys(fundingStatuses[kind]) }
  });
  return { items: result.rows, total: result.total };
}
