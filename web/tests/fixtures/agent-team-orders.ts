import type { AgentOrderKind, AgentTeamOrder } from '../../src/api/agentTeamOrders';

const identity = {
  id: 6, user_id: 42, email: 'team@example.test', owner_agent_id: 3,
  owner_agent_code: 'TEAM-003', created_at: 1_735_732_800_000
};
export const teamOrderFixtures: Record<AgentOrderKind, AgentTeamOrder> = {
  'new-coin-subscriptions': {
    ...identity, project_id: 8, symbol: 'NOVA', quote_asset: 2, quote_symbol: 'USDT',
    issue_price: '2.5', quote_amount: '25', requested_quantity: '10', allocated_quantity: '4',
    frozen_quote_amount: '0', settled_quote_amount: '10', refunded_quote_amount: '15',
    settlement_mode: 'manual_distribution', status: 'partial_allocated'
  },
  'new-coin-purchases': {
    ...identity, project_id: 8, symbol: 'NOVA', quote_asset: 2, quote_symbol: 'USDT',
    pair_id: 3, base_asset: 7, price: '2.5', quantity: '10', quote_amount: '25',
    lock_position_id: null, status: 'available'
  },
  seconds: {
    ...identity, product_id: 7, pair_id: 5, symbol: 'BTC-USDT',
    stake_asset: 2, stake_asset_symbol: 'USDT', direction: 'up',
    stake_amount: '7.123456789012345678', duration_seconds: 60, payout_rate: '0.8',
    entry_price: '100', settlement_price: null, status: 'opened', result: null,
    expires_at: 1_735_732_860_000, settled_at: null
  },
  margin: {
    ...identity, product_id: 4, pair_id: 5, symbol: 'BTC-USDT',
    margin_asset: 2, margin_asset_symbol: 'USDT', wallet_scope: 'margin', margin_mode: 'isolated',
    direction: 'long', order_type: 'limit', margin_amount: '10', leverage: '2',
    notional_amount: '20', borrowed_amount: '10', interest_amount: '0',
    entry_price: null, limit_price: '90.123456789012345678', exit_price: null,
    realized_pnl: null, opened_at: identity.created_at, closed_at: null, status: 'opened'
  },
  earn: {
    ...identity, product_id: 8, product_name: '稳健理财', asset_id: 2, asset_symbol: 'USDT',
    amount: '9007199254740993.123456789012345678', apr_rate: '0.08', term_days: 30,
    status: 'subscribed', matures_at: identity.created_at + 30 * 86_400_000, redeemed_at: null
  },
  spot: {
    ...identity, pair_id: 3, symbol: 'BTC-USDT', side: 'buy', order_type: 'limit',
    price: '90.123456789012345678', trigger_price: null, quantity: '2', filled_quantity: '1',
    status: 'partially_filled', updated_at: identity.created_at
  }
};
