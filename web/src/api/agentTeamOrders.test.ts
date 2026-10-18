import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { teamOrderFixtures } from '../../tests/fixtures/agent-team-orders';
import { authStore } from '../auth/authStore';
import { ContractError } from './client';
import { agentOrderKinds, getAgentTeamOrders, type AgentTeamOrdersQuery } from './agentTeamOrders';

const response = (payload: unknown) => new Response(JSON.stringify(payload), { status: 200 });

describe('agent team order API', () => {
  beforeEach(() => {
    authStore.setSession({ scope: 'admin', subject: 'admin:1', accessToken: 'admin', refreshToken: 'a-refresh' });
    authStore.setSession({ scope: 'agent', subject: 'agent:9', accessToken: 'agent', refreshToken: 'b-refresh' });
  });
  afterEach(() => { vi.unstubAllGlobals(); localStorage.clear(); sessionStorage.clear(); });

  it.each(agentOrderKinds)('reads %s using agent auth and preserves exact snapshots', async (kind) => {
    const row = teamOrderFixtures[kind];
    const fetchMock = vi.fn().mockResolvedValue(response({ orders: [row], total: 41 }));
    vi.stubGlobal('fetch', fetchMock);
    const query = {
      team_scope: 'descendants', agent_id: 3, user_id: 42, email: ' test+tag@x.test ',
      order_id: 6, limit: 20, offset: 20, created_from: 1_735_732_800_000, created_to: 1_735_732_900_000
    };
    await expect(getAgentTeamOrders(kind, query)).resolves.toEqual({ orders: [row], total: 41 });
    const [url, init] = fetchMock.mock.calls[0];
    expect(new URL(url).pathname).toBe(`/agent/api/v1/orders/${kind}`);
    const params = Object.fromEntries(new URL(url).searchParams);
    expect(params).toEqual(Object.fromEntries(Object.entries(query).map(([key, value]) => [key, String(value).trim()])));
    expect((init.headers as Headers).get('Authorization')).toBe('Bearer agent');
  });

  it.each(agentOrderKinds)('rejects malformed %s metadata, money, enums and pagination', async (kind) => {
    const row = teamOrderFixtures[kind];
    const amount = { seconds: 'stake_amount', margin: 'margin_amount', earn: 'amount', spot: 'quantity', 'new-coin-subscriptions': 'quote_amount', 'new-coin-purchases': 'quote_amount' }[kind];
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    for (const invalid of [
      { ...row, email: undefined }, { ...row, owner_agent_id: 0 },
      { ...row, owner_agent_code: '' }, { ...row, [amount]: 1.5 },
      { ...row, status: 'unknown' }, { ...row, created_at: '2026-10-10' }
    ]) {
      fetchMock.mockResolvedValueOnce(response({ orders: [invalid], total: 1 }));
      await expect(getAgentTeamOrders(kind)).rejects.toBeInstanceOf(ContractError);
    }
    for (const total of [-1, 1.5, '1', Number.MAX_SAFE_INTEGER + 1]) {
      fetchMock.mockResolvedValueOnce(response({ orders: [row], total }));
      await expect(getAgentTeamOrders(kind)).rejects.toBeInstanceOf(ContractError);
    }
  });

  it.each<AgentTeamOrdersQuery>([
    { user_id: 0 }, { agent_id: Number.MAX_SAFE_INTEGER + 1 }, { limit: 101 },
    { limit: 0 }, { offset: 100001 }, { offset: 1.5 }, { product_id: -1 },
    { created_from: -1 }, { created_to: 253_402_300_800_000 },
    { created_from: 10, created_to: 10 }
  ])('blocks invalid request values before fetch: %j', async (query) => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    await expect(getAgentTeamOrders('seconds', query)).rejects.toThrow();
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('preserves historical subscription nulls and rejects fabricated numeric settlement amounts', async () => {
    const row = { ...teamOrderFixtures['new-coin-subscriptions'], status: 'available',
      settlement_mode: 'legacy_instant', settled_quote_amount: null, refunded_quote_amount: null };
    const fetchMock = vi.fn().mockResolvedValue(response({ orders: [row], total: 1 }));
    vi.stubGlobal('fetch', fetchMock);
    await expect(getAgentTeamOrders('new-coin-subscriptions', { project_id: 8, symbol: 'NOVA' })).resolves.toEqual({ orders: [row], total: 1 });
    expect(new URL(fetchMock.mock.calls[0][0]).searchParams.get('project_id')).toBe('8');
    fetchMock.mockResolvedValue(response({ orders: [{ ...row, settled_quote_amount: 0 }], total: 1 }));
    await expect(getAgentTeamOrders('new-coin-subscriptions')).rejects.toBeInstanceOf(ContractError);
  });

  it('accepts refunded seconds without a fabricated result and nullable email', async () => {
    const row = { ...teamOrderFixtures.seconds, email: null, status: 'refunded' };
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response({ orders: [row], total: 1 })));
    await expect(getAgentTeamOrders('seconds')).resolves.toEqual({ orders: [row], total: 1 });
  });
});
