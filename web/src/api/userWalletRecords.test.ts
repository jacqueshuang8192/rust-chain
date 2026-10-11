import { beforeEach, describe, expect, it, vi } from 'vitest';
import { apiRequest } from './client';
import { getUserWalletRecords, parseUserWalletRecords } from './userWalletRecords';

vi.mock('./client', async (original) => ({ ...await original<typeof import('./client')>(), apiRequest: vi.fn() }));
const row = () => ({ id: 8, user_id: 42, asset_id: 2, account_type: 'spot', asset_symbol: 'USDT',
  change_type: 'seconds_contract_settle_win', balance_type: 'available', amount: '0.000000000000000001',
  balance_after: '9007199254740993.000000000000000001', available_after: '9007199254740993.000000000000000001',
  frozen_after: '0', locked_after: '0', ref_type: 'seconds_contract', ref_id: 'SC-42', created_at: 1791684000000 });

describe('user wallet records contract', () => {
  beforeEach(() => { vi.clearAllMocks(); });
  it('keeps exact amounts and rejects wrong user/account, unsafe identifiers and numeric money', () => {
    expect(parseUserWalletRecords({ ledger: [row()], total: 1 }, 42, 'spot', 20, '/test').ledger[0].amount).toBe('0.000000000000000001');
    for (const patch of [{ user_id: 43 }, { account_type: 'margin' }, { amount: 0.1 }, { available_after: null },
      { id: Number.MAX_SAFE_INTEGER + 1 }, { asset_id: -1 }, { created_at: 0.5 }, { balance_type: 'unknown' }, { ref_id: 1 }]) {
      expect(() => parseUserWalletRecords({ ledger: [{ ...row(), ...patch }], total: 1 }, 42, 'spot', 20, '/test')).toThrow();
    }
    expect(() => parseUserWalletRecords({ ledger: [row()], total: -1 }, 42, 'spot', 20, '/test')).toThrow();
  });
  it('routes through the correct auth scope and locks admin user identity', async () => {
    vi.mocked(apiRequest).mockResolvedValue({ ledger: [row()], total: 1 });
    await getUserWalletRecords('agent', 42, { account_type: 'spot', ref_id: 'SC&42', user_id: '99' }, 20, 40);
    let [path, init] = vi.mocked(apiRequest).mock.calls[0];
    expect(path).toContain('/agent/api/v1/users/42/wallet-ledger?');
    expect(init?.authScope).toBe('agent');
    expect(new URL(path, 'https://example.test').searchParams.get('ref_id')).toBe('SC&42');
    expect(new URL(path, 'https://example.test').searchParams.has('user_id')).toBe(false);
    await getUserWalletRecords('admin', 42, { user_id: '99' }, 20, 0);
    [path, init] = vi.mocked(apiRequest).mock.calls[1];
    expect(init?.authScope).toBe('admin');
    expect(new URL(path, 'https://example.test').searchParams.get('user_id')).toBe('42');
    await expect(getUserWalletRecords('agent', 1.5, {}, 20, 0)).rejects.toThrow();
    await expect(getUserWalletRecords('agent', 42, {}, 101, 0)).rejects.toThrow();
    expect(apiRequest).toHaveBeenCalledTimes(2);
  });
});
