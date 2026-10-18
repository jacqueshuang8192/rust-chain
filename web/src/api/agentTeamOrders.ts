import {
  agentMarginOrderContract, agentSecondsOrderContract, agentSpotOrderContract,
  listAgentFinancialRows, type AgentFinancialRowContract
} from './agent';
import { requiredSafeInteger } from '../shared/integer';
import type { ApiRecord } from './types';

export const agentOrderKinds = ['seconds', 'margin', 'earn', 'spot', 'new-coin-subscriptions', 'new-coin-purchases'] as const;
export type AgentOrderKind = typeof agentOrderKinds[number];
export type AgentTeamOrder = ApiRecord & {
  id: number; user_id: number; email: string | null;
  owner_agent_id: number; owner_agent_code: string; status: string; created_at: number;
};
export type AgentTeamOrdersQuery = {
  team_scope?: string; agent_id?: number; user_id?: number; email?: string;
  order_id?: number; product_id?: number; project_id?: number; symbol?: string; status?: string;
  direction?: string; result?: string; created_from?: number; created_to?: number;
  limit?: number; offset?: number;
};

const earnContract: AgentFinancialRowContract = {
  requiredFields: ['id', 'user_id', 'product_id', 'product_name', 'asset_id', 'asset_symbol',
    'amount', 'apr_rate', 'term_days', 'status', 'created_at', 'matures_at', 'redeemed_at'],
  integerFields: ['id', 'user_id', 'product_id', 'asset_id'],
  integerRanges: { term_days: [1, 4_294_967_295] },
  stringFields: ['product_name', 'asset_symbol'],
  decimalFields: ['amount', 'apr_rate'],
  timestampFields: ['created_at', 'matures_at'],
  nullableTimestampFields: ['redeemed_at'],
  enumFields: { status: ['subscribed', 'redeemed'] }
};

const newCoinCommon = {
  requiredFields: ['id', 'user_id', 'project_id', 'symbol', 'quote_asset', 'quote_symbol', 'quote_amount', 'status', 'created_at'],
  integerFields: ['id', 'user_id', 'project_id', 'quote_asset'],
  stringFields: ['symbol', 'quote_symbol'],
  timestampFields: ['created_at']
};
const subscriptionContract: AgentFinancialRowContract = {
  ...newCoinCommon,
  requiredFields: [...newCoinCommon.requiredFields, 'issue_price', 'settlement_mode', 'frozen_quote_amount',
    'settled_quote_amount', 'refunded_quote_amount', 'requested_quantity', 'allocated_quantity'],
  decimalFields: ['quote_amount', 'issue_price', 'frozen_quote_amount', 'requested_quantity', 'allocated_quantity'],
  nullableDecimalFields: ['settled_quote_amount', 'refunded_quote_amount'],
  enumFields: { status: ['pending', 'allocated', 'partial_allocated', 'refunded', 'available'], settlement_mode: ['manual_distribution', 'legacy_instant'] }
};
const purchaseContract: AgentFinancialRowContract = {
  ...newCoinCommon,
  requiredFields: [...newCoinCommon.requiredFields, 'pair_id', 'base_asset', 'price', 'quantity', 'lock_position_id'],
  integerFields: [...newCoinCommon.integerFields, 'pair_id', 'base_asset'],
  decimalFields: ['quote_amount', 'price', 'quantity'],
  enumFields: { status: ['pending', 'locked', 'available'] }
};

const contracts: Record<AgentOrderKind, AgentFinancialRowContract> = {
  seconds: agentSecondsOrderContract, margin: agentMarginOrderContract,
  earn: earnContract, spot: agentSpotOrderContract,
  'new-coin-subscriptions': subscriptionContract, 'new-coin-purchases': purchaseContract
};

export async function getAgentTeamOrders(kind: AgentOrderKind, query: AgentTeamOrdersQuery = {}) {
  if (!agentOrderKinds.includes(kind)) throw new Error('订单类型无效');
  const params = new URLSearchParams();
  const ids = ['agent_id', 'user_id', 'order_id', 'product_id', 'project_id'] as const;
  for (const key of ids) {
    if (query[key] !== undefined) params.set(key, String(requiredSafeInteger(query[key], key, 1)));
  }
  for (const key of ['team_scope', 'email', 'symbol', 'status', 'direction', 'result'] as const) {
    const value = query[key]?.trim();
    if (value) params.set(key, value);
  }
  for (const key of ['created_from', 'created_to'] as const) {
    if (query[key] !== undefined) params.set(key, String(requiredSafeInteger(query[key], key, 0, 253_402_300_799_999)));
  }
  if (query.created_from !== undefined && query.created_to !== undefined && query.created_from >= query.created_to) {
    throw new Error('结束时间必须晚于开始时间');
  }
  params.set('limit', String(requiredSafeInteger(query.limit ?? 20, '分页大小', 1, 100)));
  params.set('offset', String(requiredSafeInteger(query.offset ?? 0, '分页偏移', 0, 100_000)));
  const base = contracts[kind];
  const response = await listAgentFinancialRows<AgentTeamOrder>(`/agent/api/v1/orders/${kind}?${params}`, 'orders', {
    ...base,
    requiredFields: [...base.requiredFields, 'email', 'owner_agent_id', 'owner_agent_code'],
    integerFields: [...(base.integerFields ?? []), 'owner_agent_id'],
    integerRanges: { ...base.integerRanges, owner_agent_id: [1, Number.MAX_SAFE_INTEGER] },
    stringFields: [...(base.stringFields ?? []), 'owner_agent_code'],
    nullableStringFields: [...(base.nullableStringFields ?? []), 'email']
  });
  return { orders: response.rows, total: response.total };
}
