// Local browser-only fixtures. No API request is forwarded to a backend.
(() => {
  if (location.origin !== 'http://127.0.0.1:5198') return;
  for (const scope of ['agent', 'admin']) {
    sessionStorage.setItem(`exchange_${scope}_session`, JSON.stringify({
      scope, subject: `${scope}:${scope === 'agent' ? 3 : 1}`, generation: 'recharge-browser-test',
      accessToken: 'local-mock-only', refreshToken: 'local-mock-only'
    }));
  }
  const base = {
    id: 7, agent_id: 2, agent_admin_id: 3, user_id: 42, asset_id: 1, asset_symbol: 'USDT',
    amount: '9007199254740993.000000000000000001', reason: '人工充值凭证',
    mode: 'review', status: 'pending', reviewed_by: null, review_reason: null,
    created_at: 1790000000000, reviewed_at: null, credited_at: null
  };
  const state = JSON.parse(sessionStorage.getItem('recharge-browser-state') || 'null') || {
    records: [base], policy: { agent_id: 2, direct_recharge_enabled: false, version: 0 }, writes: [], keys: {}
  };
  window.rechargeMock = state;
  const persist = () => sessionStorage.setItem('recharge-browser-state', JSON.stringify(state));
  const nativeFetch = window.fetch.bind(window);
  window.fetch = async (input, init = {}) => {
    const url = new URL(typeof input === 'string' ? input : input.url, location.href);
    if (!url.pathname.includes('/api/')) return nativeFetch(input, init);
    const path = url.pathname;
    const method = init.method || 'GET';
    const body = init.body ? JSON.parse(init.body) : {};
    const reply = (value, status = 200) => new Response(JSON.stringify(value), {
      status, headers: { 'content-type': 'application/json' }
    });
    if (method !== 'GET') { state.writes.push({ path, method, body }); persist(); }
    if (path.endsWith('/access/me')) return reply({
      admin_id: 1, username: '本地审核员', role_id: 1, role_name: '资金审核', is_super_admin: false,
      permissions: ['agents.read', 'agents.recharges.read', 'agents.recharges.review', 'agents.recharges.authorize']
    });
    if (path.endsWith('/recharge-policy')) {
      if (method === 'PATCH') {
        if (body.expected_version !== state.policy.version) return reply({ code: 'CONFLICT', message: '版本变化' }, 409);
        state.policy = { agent_id: 2, direct_recharge_enabled: body.direct_recharge_enabled, version: state.policy.version + 1 };
        persist();
      }
      return reply(state.policy);
    }
    if (path.endsWith('/recharges/options')) return reply({
      direct_recharge_enabled: state.policy.direct_recharge_enabled,
      assets: [{ asset_id: 1, symbol: 'USDT', precision_scale: 18 }]
    });
    if (path === '/agent/api/v1/users/42/recharge') {
      if (state.keys[body.idempotency_key]) return reply(state.records.find((row) => row.id === state.keys[body.idempotency_key]));
      const direct = state.policy.direct_recharge_enabled;
      const row = { ...base, id: 8 + state.records.length, amount: body.amount, reason: body.reason,
        mode: direct ? 'direct' : 'review', status: direct ? 'credited' : 'pending', credited_at: direct ? Date.now() : null };
      state.records.push(row); state.keys[body.idempotency_key] = row.id; persist();
      return reply(row);
    }
    const review = /^\/admin\/api\/v1\/agent-recharges\/(\d+)\/review$/.exec(path);
    if (review) {
      const row = state.records.find((record) => record.id === Number(review[1]));
      if (row.status !== 'pending') return reply({ code: 'CONFLICT', message: '已审核' }, 409);
      Object.assign(row, { status: body.decision === 'approve' ? 'credited' : 'rejected', reviewed_by: 1,
        review_reason: body.reason, reviewed_at: Date.now(), credited_at: body.decision === 'approve' ? Date.now() : null });
      persist(); return reply(row);
    }
    const detail = /^\/admin\/api\/v1\/agent-recharges\/(\d+)$/.exec(path);
    if (detail) return reply(state.records.find((row) => row.id === Number(detail[1])));
    if (path.endsWith('/agent-recharges') || path.endsWith('/recharges')) {
      const rows = state.records.filter((row) => !url.searchParams.get('status') || row.status === url.searchParams.get('status'));
      return reply({ recharges: rows, total: rows.length });
    }
    if (path === '/admin/api/v1/agents') return reply({ agents: [{
      id: 2, user_id: 50, agent_code: 'LOCAL-AGENT', email: 'agent@example.test', level: 1, status: 'active',
      parent_agent_code: null, root_agent_code: 'LOCAL-AGENT', direct_user_count: 1, team_user_count: 1,
      child_agent_count: 0, admin_username: 'local-agent', admin_user_id: 3, admin_status: 'active', created_at: base.created_at
    }] });
    if (path === '/agent/api/v1/users') return reply({ users: [{
      user_id: 42, email: 'user@example.test', status: 'active', kyc_level: 1, depth: 1,
      owner_agent_id: 2, root_agent_id: 2, direct_inviter_type: 'agent', direct_inviter_id: 2,
      referred_at: base.created_at, created_at: base.created_at
    }], total: 1 });
    return reply({ code: 'LOCAL_MOCK_UNHANDLED', message: `Unmocked ${method} ${path}` }, 404);
  };
})();
