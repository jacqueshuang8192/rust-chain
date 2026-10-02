import type { DefaultMarketPreview, DefaultMarketResponse } from '../../resources/actions/defaultMarket/types';
import type { MarketDraft, MarketPair } from '../marketSetupModel';

export const marketPair: MarketPair = { id: 11, symbol: 'HIPUSDT', status: 'active', market_type: 'strategy', price_precision: 18, qty_precision: 18 };
export const marketLive = (): DefaultMarketResponse => ({
  pair_id: 11, symbol: 'HIPUSDT', market_type: 'strategy', configured: false, version: 0, enabled: false,
  all_market_paused: false, initial_price: null, seed: 'default-market:11', updated_at: null, reference_pair: null,
  config: { mode: 'independent', follow: null, volatility: '0.0015', mean_reversion: '0.05', wick_strength: '0.25',
    price_min: null, price_max: null, volume_min: '10', volume_max: '20', depth_levels: 20 },
  runtime: { active_source: 'none', generation: 0, strategy_id: null, strategy_version: null, default_version: null,
    last_price: null, last_tick_at: null, error_message: null, follow: null },
});
export const marketDraft = (): MarketDraft => ({
  pair_id: 11, revision: 1, base_version: 0, initial_price: '1.000000000000000001', config: marketLive().config,
  published_version: null, updated_at: 1_789_775_040_000,
});
export const marketPreview = (): DefaultMarketPreview => ({
  pair_id: 11, version: 1, seed: 'default-market:11', start_price: '1.000000000000000001', follow_preview: null,
  samples: Array.from({ length: 60 }, (_, index) => ({ open_time: 1_789_775_040_000 + index * 60_000,
    open: '1.000000000000000001', high: '1.01', low: '0.99', close: '1.001', volume: '10' })),
});
