import { describe, expect, it } from 'vitest';
import { teamOrderFilters, teamOrderQuery } from './teamOrderModel';
import type { FilterValues } from '../shared/FilterBar';

describe('team order filters', () => {
  it('defaults to the whole subtree and no hidden status predicate', () => {
    expect(teamOrderQuery('seconds', {})).toEqual({});
    expect(teamOrderQuery('seconds', { status: 'refunded', user_id: '42', agent_id: '3', team_scope: 'descendants' }))
      .toEqual({ status: 'refunded', user_id: 42, agent_id: 3, team_scope: 'descendants' });
  });
  it('serializes local time to a half-open millisecond range', () => {
    expect(teamOrderQuery('earn', { created_from: '2026-10-10T09:00', created_to: '2026-10-10T10:00', email: ' user ' }))
      .toEqual({ email: 'user', created_from: new Date('2026-10-10T09:00').getTime(), created_to: new Date('2026-10-10T10:00').getTime() });
  });
  it.each(['0', '-1', '1e2', '1.5', '9007199254740992', '0x10'])('rejects nonpositive/unsafe/noninteger IDs %s', (user_id) => {
    expect(() => teamOrderQuery('spot', { user_id })).toThrow('用户ID');
  });
  it.each<FilterValues>([
    { status: 'bogus' }, { team_scope: 'bogus' }, { direction: 'buy' }, { result: 'draw' },
    { created_from: 'not-a-time' }, { created_from: '2026-10-10T10:00', created_to: '2026-10-10T09:00' }
  ])('rejects invalid enums and ranges %j', (filters) => {
    expect(() => teamOrderQuery('seconds', filters)).toThrow();
  });
  it.each(['new-coin-subscriptions', 'new-coin-purchases'] as const)('uses project and coin filters for %s', kind => {
    expect(teamOrderQuery(kind, { project_id: '8', symbol: ' NOVA ', user_id: '42' })).toEqual({ project_id: 8, symbol: 'NOVA', user_id: 42 });
    const keys = teamOrderFilters(kind).map(field => field.key);
    expect(keys).toContain('project_id');
    for (const key of ['product_id', 'direction', 'result']) expect(keys).not.toContain(key);
    expect(() => teamOrderQuery(kind, { project_id: '0' })).toThrow('项目ID');
  });
  it('offers only meaningful business filters', () => {
    expect(teamOrderFilters('earn').map(field => field.key)).not.toEqual(expect.arrayContaining(['symbol', 'direction', 'result']));
    expect(teamOrderFilters('spot').map(field => field.key)).not.toContain('product_id');
  });
});
