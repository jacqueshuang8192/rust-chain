import type { RechargeRecord } from '../api/recharges';
import type { AdminAccess } from '../admin/access';

export const rechargeFixture: RechargeRecord = {
  id: 7, agent_id: 2, agent_admin_id: 3, user_id: 42, asset_id: 1, asset_symbol: 'USDT',
  amount: '9007199254740993.000000000000000001', reason: '人工充值凭证',
  mode: 'review', status: 'pending', reviewed_by: null, review_reason: null,
  created_at: 1_790_000_000_000, reviewed_at: null, credited_at: null
};
export const rechargeAccess = (permissions: string[]): AdminAccess => ({
  admin_id: 1, username: 'reviewer', role_id: 1, role_name: '资金审核', permissions, is_super_admin: false
});
