import { fundingStatuses, type FundingKind, type FundingQuery } from '../api/agentFunding';
import type { FilterField, FilterValues } from '../shared/FilterBar';
import { parseSafeInteger } from '../shared/integer';

const options = (values: Record<string, string>) => Object.entries(values).map(([value, label]) => ({ value, label }));
export function fundingFilters(kind: FundingKind): FilterField[] {
  return [
    { key: 'team_scope', label: '团队范围', type: 'select', options: options({ own: '本级归属用户', descendants: '下级代理归属用户' }) },
    { key: 'user_id', label: '用户ID' }, { key: 'email', label: '用户邮箱', placeholder: '邮箱包含' },
    { key: 'status', label: '状态', type: 'select', options: options(fundingStatuses[kind]) },
    ...(kind === 'deposits' ? [{ key: 'source', label: '充值方式', type: 'select' as const, options: options({ quick: '快速充值', chain: '链上充值' }) }, { key: 'order_no', label: '充值订单号', placeholder: '完整订单号' }] : []),
    { key: 'agent_id', label: '代理分支ID', placeholder: '该代理及其下级' },
    { key: 'record_id', label: '记录ID' }, { key: 'asset_symbol', label: '币种', placeholder: '如 ETH、USDT、USDC' },
    { key: 'network', label: '网络', placeholder: '如 eth、base、tron' },
    { key: 'created_from', label: '开始时间', type: 'datetime' }, { key: 'created_to', label: '结束时间（不含）', type: 'datetime' }
  ];
}
export function fundingQuery(kind: FundingKind, filters: FilterValues): FundingQuery {
  const query: FundingQuery = {};
  for (const field of fundingFilters(kind)) {
    const value = filters[field.key]?.trim(); if (!value) continue;
    if (field.key.endsWith('_id')) {
      const id = parseSafeInteger(value, 1); if (id === null) throw new Error(`${field.label}必须为正安全整数`);
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
  if (query.created_from !== undefined && query.created_to !== undefined && query.created_from >= query.created_to) throw new Error('结束时间必须晚于开始时间');
  return query;
}
