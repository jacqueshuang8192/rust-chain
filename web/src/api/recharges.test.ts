import { beforeEach, describe, expect, it, vi } from 'vitest';
import { apiRequest, ContractError } from './client';
import { getAdminRecharge, getRechargeOptions, getRechargePolicy, listRecharges, parseRecharge, reviewRecharge, submitAgentRecharge, updateRechargePolicy } from './recharges';
import { rechargeFixture } from '../recharges/testFixtures';

vi.mock('./client', async () => ({ ...await vi.importActual<typeof import('./client')>('./client'), apiRequest: vi.fn() }));
const request = vi.mocked(apiRequest);
beforeEach(() => request.mockReset());

describe('recharge wire contracts', () => {
  it('pins agent auth and preserves exact decimal strings and request keys', async () => {
    request.mockResolvedValue(rechargeFixture);
    const { amount, reason, asset_id } = rechargeFixture;
    await submitAgentRecharge(42, { amount, reason, asset_id }, 'stable-key');
    expect(request).toHaveBeenCalledWith('/agent/api/v1/users/42/recharge', {
      authScope: 'agent', method: 'POST', body: JSON.stringify({ amount, reason, asset_id, idempotency_key: 'stable-key' })
    });
  });
  it('serializes only the account-owned history filters for agents', async () => {
    request.mockResolvedValue({ recharges: [rechargeFixture], total: 41 });
    await listRecharges('agent', { user_id: 42, agent_id: 999, status: 'pending', limit: 20, offset: 20 });
    expect(request).toHaveBeenLastCalledWith('/agent/api/v1/recharges?status=pending&user_id=42&limit=20&offset=20', { authScope: 'agent', signal: undefined });
    await listRecharges('admin', { agent_id: 2, user_id: 42, limit: 20, offset: 0 });
    expect(request.mock.lastCall?.[0]).toBe('/admin/api/v1/agent-recharges?user_id=42&agent_id=2&limit=20&offset=0');
  });
  it('uses POST review with exact decision/reason and PATCH policy CAS', async () => {
    request.mockResolvedValue({ ...rechargeFixture, status: 'rejected', review_reason: '核验不符', reviewed_by: 1, reviewed_at: rechargeFixture.created_at });
    await reviewRecharge(7, { decision: 'reject', reason: ' 核验不符 ' });
    expect(request).toHaveBeenLastCalledWith('/admin/api/v1/agent-recharges/7/review', {
      authScope: 'admin', method: 'POST', body: '{"decision":"reject","reason":"核验不符"}'
    });
    request.mockResolvedValue({ agent_id: 2, direct_recharge_enabled: true, version: 5 });
    await getRechargePolicy(2);
    await updateRechargePolicy(2, { direct_recharge_enabled: true, expected_version: 4, reason: ' 授权凭证 ' });
    expect(request).toHaveBeenLastCalledWith('/admin/api/v1/agents/2/recharge-policy', {
      authScope: 'admin', method: 'PATCH', body: '{"direct_recharge_enabled":true,"expected_version":4,"reason":"授权凭证"}'
    });
  });
  it.each([
    ['amount', 0.1], ['amount', '1e21'], ['amount', '1e-19'], ['id', 9007199254740992],
    ['reviewed_at', undefined], ['created_at', '1790000000000'], ['reviewed_by', undefined],
    ['review_reason', undefined], ['mode', 'unknown'], ['status', 'approved']
  ])('rejects malformed %s: %s', (key, value) => {
    expect(() => parseRecharge({ ...rechargeFixture, [key]: value }, '/test')).toThrow(ContractError);
  });
  it.each([-1, 1.5, '41', 9007199254740992])('rejects malformed total %s', async (total) => {
    request.mockResolvedValue({ recharges: [], total });
    await expect(listRecharges('agent', { limit: 20, offset: 0 })).rejects.toThrow(ContractError);
  });
  it('reads the exact authoritative record without mutation and rejects mismatched IDs', async () => {
    request.mockResolvedValue(rechargeFixture);
    expect(await getAdminRecharge(7)).toEqual(rechargeFixture);
    expect(request).toHaveBeenCalledWith('/admin/api/v1/agent-recharges/7', { authScope: 'admin' });
    await expect(getAdminRecharge(8)).rejects.toThrow(ContractError);
  });
  it.each([
    { status: 'credited' },
    { mode: 'direct' },
    { credited_at: rechargeFixture.created_at },
    { status: 'rejected', reviewed_by: 1, reviewed_at: rechargeFixture.created_at, review_reason: '' }
  ])('rejects inconsistent state evidence %s', (changes) => {
    expect(() => parseRecharge({ ...rechargeFixture, ...changes }, '/test')).toThrow(ContractError);
  });
  it('requires authoritative precision and rejects unsafe request IDs before HTTP', async () => {
    request.mockResolvedValue({ direct_recharge_enabled: true, assets: [{ asset_id: 1, symbol: 'USDT', precision_scale: 19 }] });
    await expect(getRechargeOptions()).rejects.toThrow(ContractError);
    request.mockClear();
    await expect(submitAgentRecharge(9007199254740992, { asset_id: 1, amount: '1', reason: 'test' }, 'key')).rejects.toThrow();
    await expect(listRecharges('admin', { limit: 20, offset: 0, user_id: 1.5 })).rejects.toThrow();
    expect(request).not.toHaveBeenCalled();
  });
  it('does not acknowledge a successful response for another user or amount', async () => {
    request.mockResolvedValue({ ...rechargeFixture, user_id: 43 });
    await expect(submitAgentRecharge(42, { asset_id: 1, amount: rechargeFixture.amount, reason: rechargeFixture.reason }, 'key')).rejects.toThrow(ContractError);
  });
});
