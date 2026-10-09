// Local, read-only visual fixtures. Block every non-static request not handled here.
(() => {
  if (location.origin !== 'http://127.0.0.1:5198') return;
  sessionStorage.setItem('exchange_agent_session', JSON.stringify({
    scope: 'agent', subject: 'agent:1', generation: 'team-orders-local',
    accessToken: 'local-mock-only', refreshToken: 'local-mock-only'
  }));
  const now = Date.now();
  const common = {
    id: 12, user_id: 42, email: 'long-team-verification-address@example.invalid',
    owner_agent_id: 3, owner_agent_code: 'TEAM-003', created_at: now - 10_000
  };
  const fixtures = {
    seconds: {
      ...common, product_id: 7, pair_id: 2, symbol: 'BTC/USDT', stake_asset: 1,
      stake_asset_symbol: 'USDT', direction: 'up', stake_amount: '128.123456789012345678',
      duration_seconds: 60, payout_rate: '0.8', entry_price: '60000.000000000000000001',
      settlement_price: null, status: 'opened', result: null, expires_at: now + 60_000, settled_at: null
    },
    margin: {
      ...common, product_id: 4, pair_id: 2, symbol: 'BTC/USDT', margin_asset: 1,
      margin_asset_symbol: 'USDT', wallet_scope: 'margin', margin_mode: 'isolated',
      direction: 'long', order_type: 'limit', margin_amount: '100', leverage: '2',
      notional_amount: '200', borrowed_amount: '100', interest_amount: '0',
      entry_price: null, limit_price: '59000', exit_price: null, realized_pnl: null,
      opened_at: now - 10_000, closed_at: null, status: 'opened'
    },
    earn: {
      ...common, product_id: 8, product_name: 'USDT 稳健理财', asset_id: 1, asset_symbol: 'USDT',
      amount: '1200.123456789012345678', apr_rate: '0.08', term_days: 30,
      status: 'subscribed', matures_at: now + 30 * 86_400_000, redeemed_at: null
    },
    spot: {
      ...common, pair_id: 2, symbol: 'BTC/USDT', side: 'buy', order_type: 'limit',
      price: '59900.123456789012345678', trigger_price: null, quantity: '2',
      filled_quantity: '1', status: 'partially_filled', updated_at: now
    }
  };
  window.teamOrderVerification = { requests: [], writes: [], unhandled: [] };
  const nativeFetch = window.fetch.bind(window);
  window.fetch = async (input, init = {}) => {
    const url = new URL(typeof input === 'string' ? input : input.url, location.href);
    if (!url.pathname.includes('/api/') && url.origin === location.origin) return nativeFetch(input, init);
    const method = init.method || 'GET';
    const log = window.teamOrderVerification;
    log.requests.push(url.href);
    const reply = (data, status = 200) => new Response(JSON.stringify(data), { status, headers: { 'content-type': 'application/json' } });
    if (method !== 'GET') {
      log.writes.push({ method, path: url.pathname });
      return reply({ code: 'LOCAL_READ_ONLY', message: 'Read-only verification' }, 403);
    }
    const kind = url.pathname.split('/agent/api/v1/orders/')[1];
    if (fixtures[kind]) {
      const query = url.searchParams;
      let orders = Array.from({ length: 23 }, (_, i) => ({
        ...fixtures[kind], id: 100 - i, user_id: i % 2 ? 43 : 42,
        owner_agent_id: i % 2 ? 1 : 3, owner_agent_code: i % 2 ? 'TEAM-001' : 'TEAM-003'
      }));
      for (const key of ['user_id', 'agent_id', 'status']) {
        if (query.get(key)) orders = orders.filter(row => String(row[key === 'agent_id' ? 'owner_agent_id' : key]) === query.get(key));
      }
      if (query.get('email')) orders = orders.filter(row => row.email.includes(query.get('email')));
      if (query.get('team_scope') === 'own') orders = orders.filter(row => row.owner_agent_id === 1);
      if (query.get('team_scope') === 'descendants') orders = orders.filter(row => row.owner_agent_id !== 1);
      const offset = Number(query.get('offset') || 0);
      return reply({ orders: orders.slice(offset, offset + Number(query.get('limit') || 20)), total: orders.length });
    }
    log.unhandled.push(url.pathname);
    return reply({ code: 'LOCAL_MOCK_UNHANDLED', message: `Unhandled ${url.pathname}` }, 404);
  };
})();
