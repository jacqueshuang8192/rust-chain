import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { fundingFixture } from '../../tests/fixtures/agent-funding';
import { authStore } from '../auth/authStore';
import { ContractError } from './client';
import { getAgentFunding } from './agentFunding';
const response = (payload: unknown) => new Response(JSON.stringify(payload), { status: 200 });
beforeEach(() => {
  authStore.setSession({ scope: 'admin', subject: 'admin:1', accessToken: 'admin', refreshToken: 'a' });
  authStore.setSession({ scope: 'agent', subject: 'agent:9', accessToken: 'agent', refreshToken: 'b' });
});
afterEach(() => { vi.unstubAllGlobals(); localStorage.clear(); sessionStorage.clear(); });
it('uses agent auth, safely encodes filters, preserves exact coin amounts and null values', async () => {
  const fetchMock = vi.fn().mockResolvedValue(response({ items: [fundingFixture], total: 42 })); vi.stubGlobal('fetch', fetchMock);
  const query = { team_scope: 'descendants', agent_id: 3, user_id: 42, order_no: 'CD+a&b', email: ' test+user@x.test ', source: 'chain', status: 'pending', limit: 20, offset: 20 };
  await expect(getAgentFunding('deposits', query)).resolves.toEqual({ items: [fundingFixture], total: 42 });
  const [url, init] = fetchMock.mock.calls[0];
  expect(new URL(url).pathname).toBe('/agent/api/v1/funding/deposits');
  expect(Object.fromEntries(new URL(url).searchParams)).toEqual(Object.fromEntries(Object.entries(query).map(([key, value]) => [key, String(value).trim()])));
  expect((init.headers as Headers).get('Authorization')).toBe('Bearer agent');
});
it('rejects incomplete identity, numeric money, missing nulls and wrong-source records', async () => {
  const fetchMock = vi.fn(); vi.stubGlobal('fetch', fetchMock);
  for (const invalid of [{ amount: 1.5 }, { credited_amount: 0 }, { fee: 1e-18 }, { owner_agent_id: 0 }, { email: undefined }, { paid_amount: undefined }, { status: 'bogus' }, { source: 'withdrawal' }, { created_at: 'today' }, { expires_at: undefined }]) {
    fetchMock.mockResolvedValueOnce(response({ items: [{ ...fundingFixture, ...invalid }], total: 1 }));
    await expect(getAgentFunding('deposits')).rejects.toBeInstanceOf(ContractError);
  }
});
it('accepts real withdrawal and quick recharge statuses and their distinct amount fields', async () => {
  const withdrawal = { ...fundingFixture, source: 'withdrawal', order_no: null, status: 'pending_review', expires_at: null, total_reserved: '9007199254740993.123456789012345679' };
  const quick = { ...fundingFixture, source: 'quick', status: 'credited', credited_amount: fundingFixture.amount, paid_amount: fundingFixture.amount, fiat_amount: '100.00', fiat_currency: 'CNY' };
  vi.stubGlobal('fetch', vi.fn().mockResolvedValueOnce(response({ items: [withdrawal], total: 1 })).mockResolvedValueOnce(response({ items: [quick], total: 1 })));
  await expect(getAgentFunding('withdrawals')).resolves.toEqual({ items: [withdrawal], total: 1 });
  await expect(getAgentFunding('deposits')).resolves.toEqual({ items: [quick], total: 1 });
});
it.each([{ user_id: 0 }, { record_id: Number.MAX_SAFE_INTEGER + 1 }, { limit: 101 }, { offset: 100001 }, { created_from: -1 }, { created_from: 10, created_to: 10 }])('rejects unsafe request before fetch: %j', async query => {
  const fetchMock = vi.fn(); vi.stubGlobal('fetch', fetchMock);
  await expect(getAgentFunding('deposits', query)).rejects.toThrow(); expect(fetchMock).not.toHaveBeenCalled();
});

it('accepts cancelled chain deposits without losing exact amounts', async () => {
  const cancelled = { ...fundingFixture, source: 'chain', status: 'cancelled' };
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response({ items: [cancelled], total: 1 })));
  await expect(getAgentFunding('deposits', { status: 'cancelled' })).resolves.toEqual({ items: [cancelled], total: 1 });
});
