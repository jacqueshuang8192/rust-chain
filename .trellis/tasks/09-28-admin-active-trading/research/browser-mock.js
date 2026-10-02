// Local UI verification only; no requests reach an application backend.
(() => {
  if (location.origin !== 'http://127.0.0.1:5198') return;
  sessionStorage.setItem('exchange_admin_session', JSON.stringify({
    scope: 'admin', subject: 'admin:1', generation: 'active-trading-local',
    accessToken: 'local-mock-only', refreshToken: 'local-mock-only'
  }));
  const now = Date.now();
  const order = {
    id: 12, user_id: 42, email: 'local@example.invalid', symbol: 'BTC/USDT',
    direction: 'up', stake_amount: '1.000000000000000001', stake_asset_symbol: 'USDT',
    duration_seconds: 60, entry_price: '60000', settlement_price: null, result: null,
    status: 'opened', expires_at: now + 60_000, created_at: now - 10_000
  };
  const position = {
    id: 9, user_id: 42, product_id: 3, pair_id: 7, margin_asset: 11,
    margin_mode: 'isolated', wallet_scope: 'margin', order_type: 'limit',
    leverage: '10', direction: 'long', margin_amount: '10', notional_amount: '100',
    borrowed_amount: '90', interest_amount: '0.000000000000000001',
    entry_price: '60000', limit_price: '60000', status: 'opened'
  };
  window.tradingVerification = { requests: [], writes: [] };
  const nativeFetch = window.fetch.bind(window);
  window.fetch = async (input, init = {}) => {
    const url = new URL(typeof input === 'string' ? input : input.url, location.href);
    if (!url.pathname.startsWith('/admin/api/') && url.origin === location.origin) return nativeFetch(input, init);
    const method = init.method || 'GET';
    window.tradingVerification.requests.push(url.href);
    const reply = (data, status = 200) => new Response(JSON.stringify(data), { status, headers: { 'content-type': 'application/json' } });
    if (method !== 'GET') {
      window.tradingVerification.writes.push({ method, path: url.pathname });
      return reply({ code: 'LOCAL_READ_ONLY', message: 'Read-only verification' }, 403);
    }
    if (url.pathname.endsWith('/access/me')) return reply({ admin_id: 1, username: '本地验证', role_id: 1, role_name: '只读', is_super_admin: false, permissions: ['seconds.orders.read', 'margin.positions.read'] });
    if (url.pathname.endsWith('/seconds-contracts/orders')) {
      let orders = [order, { ...order, id: 13, status: 'settled', result: 'win', settlement_price: '61000' }];
      const status = url.searchParams.get('status');
      if (status) orders = orders.filter(row => row.status === status);
      return reply({ orders, total: orders.length });
    }
    if (url.pathname.endsWith('/margin/positions')) {
      let positions = [position, { ...position, id: 10, entry_price: null }, { ...position, id: 11, status: 'closed' }];
      const status = url.searchParams.get('status');
      if (status) positions = positions.filter(row => row.status === status);
      if (url.searchParams.get('filled_only') === 'true') positions = positions.filter(row => row.entry_price !== null);
      return reply({ positions, total: positions.length });
    }
    return reply({ code: 'LOCAL_MOCK_UNHANDLED', message: 'Unhandled ' + url.pathname }, 404);
  };
})();
