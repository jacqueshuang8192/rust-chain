const task = await taskSpace(26);
const page = task.page('p1');
await page.cdp('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: true });
await page.evaluate(async () => {
  const resources = performance.getEntriesByType('resource').map(r => r.name);
  const { client, persistAuthTokens } = await import(resources.find(u => /\/src\/api\/client\.ts/.test(u)));
  const { default: router } = await import(resources.find(u => /\/src\/router\/index\.ts/.test(u)));
  const now = Date.now();
  const policy = { version: 2, timezone: 'Asia/Hong_Kong', purchase_initial_price: '0.2', listing_reference_price: '0.2', subscription: { at: now - 7200000, automatic: true }, distribution: { at: now - 3600000, automatic: true }, purchase: { at: now - 1800000, automatic: true }, listing: { at: now + 86400000, automatic: false }, lock: { kind: 'fixed', at: now + 7200000 }, fee: { kind: 'none' } };
  const project = { id: 900, asset_id: 901, symbol: 'KHB', name: 'KHB local fixture', logo_url: null, lifecycle_status: 'distribution', total_supply: '100000000', issue_price: '0.2', quote_asset_id: 902, quote_asset_symbol: 'USDT', quote_asset_logo_url: null, reserved_supply: '100', allocated_supply: '0', remaining_supply: '99999900', listed_at: policy.listing.at, unlock_type: 'immediate_on_listing', fixed_unlock_at: null, relative_unlock_seconds: null, unlock_fee_enabled: false, unlock_fee_rate: null, unlock_fee_basis: null, unlock_fee_asset: null, post_listing_purchase_enabled: false, post_listing_pair_id: 903, post_listing_pair_symbol: 'KHBUSDT', status: 'active', issuance_policy: policy };
  window.__phaseQA = { project, requests: [], writes: [], fail: false };
  client.defaults.adapter = async config => {
    const f = window.__phaseQA;
    const path = new URL(config.url, location.origin).pathname;
    f.requests.push(path);
    if (config.method !== 'get') { f.writes.push(path); throw new Error('No writes allowed in lifecycle browser checks'); }
    let data = {};
    if (path.endsWith('/new-coins/KHB')) { if (f.fail) throw new Error('Fixture read failure'); data = structuredClone(f.project); }
    else if (path.endsWith('/new-coins')) data = { projects: [structuredClone(f.project)] };
    else if (path.endsWith('/new-coins/holdings')) data = { holdings: [], checked_at: Date.now(), next_before_id: null };
    else if (path.includes('/wallet')) data = { accounts: [] };
    else if (path.endsWith('/klines')) data = { klines: Array.from({ length: 30 }, (_, i) => ({ symbol: 'KHBUSDT', open_time: now - 60000 * (30 - i), open: '0.2', high: '0.3', low: '0.15', close: '0.25', volume: '100', provider: 'strategy', source: 'default' })) };
    else if (path.includes('/new-coins/')) data = { subscriptions: [], distributions: [], purchases: [], unlocks: [] };
    return { data, status: 200, statusText: 'OK', headers: {}, config };
  };
  persistAuthTokens('local-phase-fixture', 'local-phase-fixture-refresh');
  await router.push({ name: 'new-coin-detail', params: { symbol: 'KHB' } });
});
await page.waitForSelector('.new-coin-detail-stage-note');
console.log(await page.snapshot({ scope: 'full_page' }));
console.log(await page.screenshot({ path: '/tmp/new-coin-distribution-390.png', fullPage: true }));
