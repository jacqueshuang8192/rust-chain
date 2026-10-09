import type { AgentOrderKind, AgentTeamOrdersQuery } from '../api/agentTeamOrders';
import type { FilterField, FilterValues } from '../shared/FilterBar';
import { parseSafeInteger } from '../shared/integer';

export const teamOrderLabels: Record<AgentOrderKind, string> = {
  seconds: '秒合约订单', margin: '合约订单', earn: '理财订单', spot: '现货订单'
};
export const teamOrderPrefixes: Record<AgentOrderKind, string> = { seconds: 'SC', margin: 'MG', earn: 'EA', spot: 'SP' };
const options = (values: Record<string, string>) => Object.entries(values).map(([value, label]) => ({ value, label }));
export const teamOrderStatuses = {
  seconds: { opened: '进行中', settled: '已结算', manual_review: '人工复核', refunded: '已退还本金' },
  margin: { pending: '待成交', opened: '持仓中', closed: '已平仓', canceled: '已取消', liquidated: '已强平' },
  earn: { subscribed: '持有中', redeemed: '已赎回' },
  spot: { pending: '待处理', open: '委托中', partially_filled: '部分成交', filled: '已成交', cancelled: '已取消', rejected: '已拒绝' }
};

export function teamOrderFilters(kind: AgentOrderKind): FilterField[] {
  return [
    { key: 'team_scope', label: '团队范围', type: 'select', options: options({ own: '本级归属用户', descendants: '下级代理归属用户' }) },
    { key: 'agent_id', label: '代理分支ID', placeholder: '该代理及其下级' },
    { key: 'user_id', label: '用户ID' }, { key: 'email', label: '用户邮箱', placeholder: '邮箱包含' },
    { key: 'order_id', label: '记录ID' },
    { key: 'status', label: '订单状态', type: 'select', options: options(teamOrderStatuses[kind]) },
    ...(kind === 'earn' ? [] : [{ key: 'symbol', label: '交易对', placeholder: 'BTC/USDT' }]),
    ...(kind === 'spot' ? [] : [{ key: 'product_id', label: '产品ID' }]),
    ...(kind === 'earn' ? [] : [{ key: 'direction', label: '方向', type: 'select' as const,
      options: options(kind === 'seconds' ? { up: '看涨', down: '看跌' } : kind === 'margin' ? { long: '做多', short: '做空' } : { buy: '买入', sell: '卖出' }) }]),
    ...(kind === 'seconds' ? [{ key: 'result', label: '结算结果', type: 'select' as const, options: options({ win: '盈利', loss: '亏损' }) }] : []),
    { key: 'created_from', label: '开始时间', type: 'datetime' },
    { key: 'created_to', label: '结束时间（不含）', type: 'datetime' }
  ];
}

export function teamOrderQuery(kind: AgentOrderKind, values: FilterValues): AgentTeamOrdersQuery {
  const query: AgentTeamOrdersQuery = {};
  for (const field of teamOrderFilters(kind)) {
    const value = values[field.key]?.trim();
    if (!value) continue;
    if (field.key.endsWith('_id')) {
      const id = parseSafeInteger(value, 1);
      if (id === null) throw new Error(`${field.label}必须为正安全整数`);
      Object.assign(query, { [field.key]: id });
    } else if (field.type === 'datetime') {
      const time = new Date(value).getTime();
      if (!Number.isSafeInteger(time) || time < 0 || time > 253_402_300_799_999) throw new Error(`${field.label}无效`);
      Object.assign(query, { [field.key]: time });
    } else {
      if (field.type === 'select' && !field.options?.some(option => option.value === value)) throw new Error(`${field.label}无效`);
      Object.assign(query, { [field.key]: value });
    }
  }
  if (query.created_from !== undefined && query.created_to !== undefined && query.created_from >= query.created_to) {
    throw new Error('结束时间必须晚于开始时间');
  }
  return query;
}
