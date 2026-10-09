// Isolated UI verification. Every application request is intercepted, writes refused.
(() => {
  if (location.origin !== 'http://127.0.0.1:5198') return;
  for (const scope of ['admin', 'agent']) {
    sessionStorage.setItem(`exchange_${scope}_session`, JSON.stringify({
      scope, subject: `${scope}:1`, generation: 'console-quality-local',
      accessToken: 'local-mock-only', refreshToken: 'local-mock-only'
    }));
  }
  const now = Date.now();
  const order = {
    id: 12, order_no: 'SC20261010000012', user_id: 42, email: 'local@example.invalid',
    product_id: 1, pair_id: 2, symbol: 'BTC/USDT', stake_asset: 1, stake_asset_symbol: 'USDT',
    direction: 'up', stake_amount: '1.000000000000000001', payout_rate: '0.8',
    duration_seconds: 60, entry_price: '60000', settlement_price: null, result: null,
    status: 'opened', expires_at: now + 60_000, created_at: now - 10_000, settled_at: null
  };
  const user = {
    user_id: 42, email: 'long-local-verification-address@example.invalid', phone: '+85212345678',
    status: 'active', kyc_level: 2, root_agent_id: 1, depth: 2, referred_at: now
  };
  const me = {
    agent_admin_id: 1, agent_id: 1, username: '本地代理验收', agent_code: 'LOCAL-001',
    level: 1, agent_status: 'active', admin_status: 'active', last_login_at: now
  };
  window.consoleVerification = { requests: [], writes: [], unhandled: [] };
  const nativeFetch = window.fetch.bind(window);
  window.fetch = async (input, init = {}) => {
    const url = new URL(typeof input === 'string' ? input : input.url, location.href);
    if (!url.pathname.includes('/api/') && url.origin === location.origin) return nativeFetch(input, init);
    const method = init.method || 'GET';
    window.consoleVerification.requests.push(url.href);
    const reply = (data, status = 200) => new Response(JSON.stringify(data), { status, headers: { 'content-type': 'application/json' } });
    if (method !== 'GET') {
      window.consoleVerification.writes.push({ method, path: url.pathname });
      return reply({ code: 'LOCAL_READ_ONLY', message: 'Read-only verification' }, 403);
    }
    const path = url.pathname;
    if (path.endsWith('/access/me')) return reply({
      admin_id: 1, username: '本地管理员验收', role_id: 1, role_name: '只读', is_super_admin: false,
      permissions: ['seconds.orders.read', 'margin.positions.read']
    });
    if (path === '/agent/api/v1/me') return reply(me);
    if (path === '/agent/api/v1/dashboard') return reply({
      agent_id: 1, team_user_count: 128, active_invite_code_count: 1, commission_record_count: 26,
      pending_commission_amount: '12', settled_commission_amount: '88', total_commission_amount: '100',
      commission_assets: [{ payout_asset_id: 1, pending_commission_amount: '12', settled_commission_amount: '88', total_commission_amount: '100' }]
    });
    if (path.endsWith('/convert/stats')) return reply({ agent_id: 1, total_orders: 10, pending_orders: 2, completed_orders: 8, total_from_amount: '100', total_to_amount: '99' });
    if (path === '/agent/api/v1/users') return reply({ users: [user, { ...user, user_id: 43, email: 'second@example.invalid' }] });
    if (path.endsWith('/invite-codes')) return reply({ invite_codes: [] });
    if (path.endsWith('/sub-agents')) return reply({ agents: [] });
    if (path.endsWith('/users/42/assets')) return reply({ assets: [], total: 0 });
    if (path.endsWith('/users/42/spot-orders')) return reply({ orders: [], total: 0 });
    if (path.endsWith('/users/42/margin-positions')) return reply({ positions: [], total: 0 });
    if (path.endsWith('/users/42/margin-orders')) return reply({ orders: [], total: 0 });
    if (path.endsWith('/seconds-contract-orders')) return reply({ orders: [order], total: 1 });
    if (path.endsWith('/seconds-contracts/orders')) return reply({ orders: [order], total: 1 });
    if (path.endsWith('/seconds-contracts/orders/12')) return reply({
      ...order, settlement_price_source: 'redis', settlement_price_observed_at: now,
      settlement_price_version: `redis:${'abcdef0123456789'.repeat(4)}`,
      settlement_price_tick_id: null, settlement_price_generation: null,
      idempotency_key: 'local-read-only-verification',
      receipt: { policy_version: 2, eligible_at: now, reason: '保留原始诊断' },
      entries: [{ settlement_price_source: 'redis', settlement_price_observed_at: now, price: '60001.000000000000000001' }],
      future_unknown_key: 'unknown-value-retained'
    });
    window.consoleVerification.unhandled.push(path);
    return reply({ code: 'LOCAL_MOCK_UNHANDLED', message: `Unhandled ${path}` }, 404);
  };
})();
