import { describe, expect, it } from 'vitest';
import { defaultMarketDraft } from '../resources/actions/defaultMarket/model';
import { checkLiveMarket, checkMarketDraft, checkMarketPreview, checkMarketSetup, hydrateMarketDraft, marketStatus } from './marketSetupModel';
import { marketDraft, marketLive, marketPair, marketPreview } from './__tests__/marketFixtures';

describe('new coin market contracts', () => {
  it('preserves exact money and rejects rounded wire numbers', () => {
    const live = marketLive();
    live.initial_price = '9007199254740993.000000000000000001';
    expect(checkLiveMarket(live, marketPair).initial_price).toBe(live.initial_price);
    expect(() => checkLiveMarket({ ...live, initial_price: 1.01 }, marketPair)).toThrow();
    expect(() => checkLiveMarket({ ...live, config: { ...live.config, volatility: 0.01 } }, marketPair)).toThrow();
    expect(() => checkMarketDraft({ ...marketDraft(), config: { ...live.config, price_min: 1 } })).toThrow();
  });
  it('requires exact project and pair identities and complete drafts', () => {
    expect(checkMarketSetup({ project_id: 7, pairs: [marketPair], draft: marketDraft() }, 7).pairs).toHaveLength(1);
    expect(() => checkMarketSetup({ project_id: 8, pairs: [], draft: null }, 7)).toThrow();
    expect(() => checkMarketSetup({ project_id: 7, pairs: [marketPair, marketPair], draft: null }, 7)).toThrow();
    expect(() => checkMarketDraft({ ...marketDraft(), revision: 2 ** 32 })).toThrow();
    expect(() => checkMarketDraft({ ...marketDraft(), published_version: 0 })).toThrow();
    expect(() => checkMarketDraft({ ...marketDraft(), config: {} })).toThrow();
  });
  it('retains custom unexposed configuration instead of applying presets implicitly', () => {
    const draft = marketDraft();
    draft.config.mean_reversion = '0.012345678901234567';
    draft.config.volume_max = '9007199254740993.000000000000000001';
    expect(hydrateMarketDraft(marketLive(), draft)).toMatchObject({ mean_reversion: draft.config.mean_reversion, volume_max: draft.config.volume_max });
  });
  it('does not call configured or stale prices running', () => {
    const live = marketLive();
    expect(marketStatus(live, marketPair, Date.now())).toBe('默认行情未启用');
    live.enabled = true;
    expect(marketStatus(live, marketPair, Date.now())).toBe('等待生成器接管');
    live.runtime.active_source = 'default'; live.runtime.last_tick_at = 10_000; live.runtime.default_version = 1; live.version = 1;
    expect(marketStatus(live, marketPair, 25_001)).toBe('行情更新延迟');
    expect(marketStatus(live, marketPair, 5_000)).toBe('行情时间异常');
    live.runtime.default_version = 0;
    expect(marketStatus(live, marketPair, 15_000)).toBe('等待新版本接续');
    live.all_market_paused = true;
    expect(marketStatus(live, marketPair, 15_000)).toBe('全部行情已暂停');
  });
  it('rejects malformed or wrong-version previews before enabling publish', () => {
    const preview = marketPreview();
    const draft = defaultMarketDraft(marketLive());
    expect(checkMarketPreview(preview, 11, 1, draft).samples).toHaveLength(60);
    expect(() => checkMarketPreview(preview, 11, 2, draft)).toThrow();
    preview.samples[0].high = '0.1';
    expect(() => checkMarketPreview(preview, 11, 1, draft)).toThrow();
  });
});
