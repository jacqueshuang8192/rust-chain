// Local UI verification only. All backend requests are intercepted in this tab.
(() => {
  if (location.origin !== 'http://127.0.0.1:5198') return;
  sessionStorage.setItem('exchange_admin_session', JSON.stringify({ scope: 'admin', subject: 'admin:1', generation: 'country-review-local', accessToken: 'local-mock-only', refreshToken: 'local-mock-only' }));
  const country = (code, name) => ({ country_code: code, country_name: name, default_locale: 'en', supported_locales: ['zh', 'en'], status: 'active' });
  const countries = [country('CN', 'China'), ...Array.from({ length: 99 }, (_, index) => country('Q' + String.fromCharCode(65 + Math.floor(index / 26)) + String.fromCharCode(65 + index % 26), 'Country ' + index)), country('ZZ', 'Tail Country')];
  const product = { id: 71, revision: 7, status: 'active', asset_id: 11, asset_symbol: 'USDT', loan_type: 'credit', name: '原有中文名称', name_json: { version: 1, default_locale: 'zh-CN', items: [{ country: 'CN', locale: 'zh-CN', title: '原有中文名称' }] }, term_days: 30, interest_rate: '0.02', interest_calculation_mode: 'full_term', min_kyc_level: 1, min_amount: '10', max_amount: '1000', user_principal_limit: null, product_principal_capacity: null, deny_borrowing_while_overdue: false, reserved_principal: '0', outstanding_principal: '0', created_at: 1790000000000, updated_at: 1790000000000 };
  let config = { id: 1, name: 'default', enabled: true, allowed_countries: ['China', 'Japan'], country_document_types: [{ country: 'China', document_types: ['passport'], handheld_document_types: [] }], required_documents: ['identity_front', 'identity_back'], target_kyc_level: 1, max_document_size_bytes: 5242880, updated_at: 1790000000000 };
  window.countryVerification = { requests: [], writes: [] };
  const nativeFetch = window.fetch.bind(window);
  window.fetch = async (input, init = {}) => {
    const url = new URL(typeof input === 'string' ? input : input.url, location.href);
    const path = url.pathname;
    const reply = (data, status = 200) => new Response(JSON.stringify(data), { status, headers: { 'content-type': 'application/json' } });
    if (!path.startsWith('/admin/api/') && url.origin === location.origin) return nativeFetch(input, init);
    const method = init.method || 'GET';
    window.countryVerification.requests.push(url.href);
    if (method !== 'GET') window.countryVerification.writes.push({ path, method, body: JSON.parse(init.body || '{}') });
    if (path.endsWith('/access/me')) return reply({ admin_id: 1, username: '本地验证', role_id: 1, role_name: '测试', is_super_admin: true, permissions: [] });
    if (path.endsWith('/countries')) {
      const offset = Number(url.searchParams.get('offset') || 0);
      const limit = Math.min(100, Number(url.searchParams.get('limit') || 50));
      return reply({ countries: countries.slice(offset, offset + limit), total: countries.length });
    }
    if (path.endsWith('/assets')) return reply({ assets: [{ id: 11, symbol: 'USDT', name: 'Tether', precision_scale: 18 }], total: 1 });
    if (path.endsWith('/loan/products')) return reply({ products: [product], total: 1 });
    if (path.endsWith('/loan/products/71')) return reply(product);
    if (path.endsWith('/kyc/config')) {
      if (method === 'PATCH') config = { ...config, ...JSON.parse(init.body) };
      return reply(config);
    }
    return reply({ code: 'LOCAL_MOCK_UNHANDLED', message: 'Unhandled ' + method + ' ' + path }, 404);
  };
})();
