import type { FundingRecord } from '../../src/api/agentFunding';
export const fundingFixture: FundingRecord = {
  id: 7, source: 'chain', order_no: 'CD2026102400007', user_id: 42, email: 'team@example.test', owner_agent_id: 3, owner_agent_code: 'TEAM-003',
  asset_symbol: 'USDT', network: 'eth', address: '0x1111111111111111111111111111111111111111',
  amount: '9007199254740993.123456789012345678', paid_amount: null, credited_amount: null, fee: '0.000000000000000001', total_reserved: null,
  fiat_amount: null, fiat_currency: null, status: 'pending', tx_hash: null, created_at: 1_735_732_800_000, expires_at: 1_735_734_600_000, completed_at: null
};
