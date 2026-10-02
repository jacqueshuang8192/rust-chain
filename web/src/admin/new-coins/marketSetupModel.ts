import { ContractError } from '../../api/client';
import { compareDecimalText, decimalFitsStorage } from '../../shared/decimal';
import { defaultMarketDraft, validateDefaultMarketDraft } from '../resources/actions/defaultMarket/model';
import type { DefaultMarketDraft, DefaultMarketParameters, DefaultMarketPreview, DefaultMarketResponse } from '../resources/actions/defaultMarket/types';

export type MarketPair = {
  id: number; symbol: string; status: string; market_type: string; price_precision: number; qty_precision: number;
};
export type MarketDraft = {
  pair_id: number; revision: number; base_version: number; initial_price: string | null;
  config: DefaultMarketParameters; published_version: number | null; updated_at: number;
};
export type MarketSetup = { project_id: number; pairs: MarketPair[]; draft: MarketDraft | null };
const uint = (value: unknown, min = 0) => typeof value === 'number' && Number.isSafeInteger(value) && value >= min && value <= 4_294_967_295;
const id = (value: unknown) => typeof value === 'number' && Number.isSafeInteger(value) && value > 0;
const timestamp = (value: unknown) => id(value) && Number.isFinite(new Date(value as number).getTime());
const object = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value);
const fail = (): never => { throw new ContractError('项目行情响应不完整，请重新加载', { path: 'new-coin-market-setup' }); };

export function checkMarketSetup(value: unknown, projectId: number): MarketSetup {
  if (!object(value) || value.project_id !== projectId || !Array.isArray(value.pairs)) return fail();
  const seen = new Set<number>();
  for (const pair of value.pairs) {
    if (!object(pair) || !id(pair.id) || typeof pair.symbol !== 'string' || !pair.symbol
      || !['active', 'disabled', 'inactive'].includes(String(pair.status))
      || !['internal', 'strategy', 'external'].includes(String(pair.market_type))
      || !uint(pair.price_precision) || Number(pair.price_precision) > 18
      || !uint(pair.qty_precision) || Number(pair.qty_precision) > 18 || seen.has(pair.id as number)) return fail();
    seen.add(pair.id as number);
  }
  if (value.draft !== null) checkMarketDraft(value.draft);
  return value as MarketSetup;
}

export function checkMarketDraft(value: unknown): MarketDraft {
  if (!object(value) || !id(value.pair_id) || !uint(value.revision, 1) || !uint(value.base_version)
    || (value.published_version !== null && (!uint(value.published_version, 1) || Number(value.published_version) <= Number(value.base_version)))
    || !timestamp(value.updated_at)) return fail();
  checkParameters(value.config, value.initial_price);
  return value as MarketDraft;
}

function checkParameters(config: unknown, price: unknown): asserts config is DefaultMarketParameters {
  if (!object(config) || (price !== null && !decimalFitsStorage(price))) return fail();
  for (const key of ['volatility', 'mean_reversion', 'volume_min', 'volume_max', 'wick_strength']) {
    if (!decimalFitsStorage(config[key])) return fail();
  }
  for (const key of ['price_min', 'price_max']) {
    if (config[key] !== null && !decimalFitsStorage(config[key])) return fail();
  }
  if (!uint(config.depth_levels, 1) || Number(config.depth_levels) > 20
    || !['independent', 'follow'].includes(String(config.mode))) return fail();
  if (config.mode === 'follow') {
    const follow = config.follow;
    if (!object(follow) || !id(follow.reference_pair_id) || !decimalFitsStorage(follow.multiplier)
      || !decimalFitsStorage(follow.max_move_ratio) || !uint(follow.stale_after_seconds)) return fail();
  } else if (config.follow !== null) return fail();
}

export function checkLiveMarket(value: unknown, pair: MarketPair): DefaultMarketResponse {
  if (!object(value) || value.pair_id !== pair.id || value.symbol !== pair.symbol || value.market_type !== pair.market_type
    || !uint(value.version) || typeof value.configured !== 'boolean' || typeof value.enabled !== 'boolean'
    || typeof value.all_market_paused !== 'boolean' || typeof value.seed !== 'string'
    || (value.updated_at !== null && !timestamp(value.updated_at)) || !object(value.runtime)) return fail();
  checkParameters(value.config, value.initial_price);
  const runtime = value.runtime;
  if (!['none', 'default', 'strategy'].includes(String(runtime.active_source))
    || (runtime.last_price !== null && !decimalFitsStorage(runtime.last_price))
    || (runtime.last_tick_at !== null && !timestamp(runtime.last_tick_at))
    || (runtime.generation !== 0 && !id(runtime.generation))
    || (runtime.strategy_id !== null && !id(runtime.strategy_id))
    || (runtime.strategy_version !== null && !uint(runtime.strategy_version, 1))
    || (runtime.default_version !== null && !uint(runtime.default_version, 1))
    || (runtime.error_message !== null && typeof runtime.error_message !== 'string')) return fail();
  if (value.reference_pair !== null && value.reference_pair !== undefined) {
    const reference = value.reference_pair;
    if (!object(reference) || !id(reference.id) || typeof reference.symbol !== 'string'
      || typeof reference.status !== 'string' || reference.market_type !== 'external') return fail();
  }
  if (runtime.follow !== null && runtime.follow !== undefined) {
    const follow = runtime.follow;
    if (!object(follow) || !['following', 'fallback'].includes(String(follow.mode))
      || !id(follow.reference_pair_id) || typeof follow.reference_symbol !== 'string'
      || (follow.reference_price !== null && !decimalFitsStorage(follow.reference_price))
      || (follow.reference_observed_at !== null && !timestamp(follow.reference_observed_at))
      || !timestamp(follow.switched_at)
      || (follow.fallback_reason !== null && typeof follow.fallback_reason !== 'string')) return fail();
  }
  const result = value as DefaultMarketResponse;
  if (validateDefaultMarketDraft(defaultMarketDraft(result), pair.price_precision, pair.qty_precision)) return fail();
  return result;
}

export function hydrateMarketDraft(live: DefaultMarketResponse, saved: MarketDraft | null): DefaultMarketDraft {
  return defaultMarketDraft(saved ? { ...live, initial_price: saved.initial_price, config: saved.config } : live);
}

export function checkMarketPreview(value: unknown, pairId: number, version: number, draft: DefaultMarketDraft): DefaultMarketPreview {
  if (!object(value) || value.pair_id !== pairId || value.version !== version || typeof value.seed !== 'string'
    || !decimalFitsStorage(value.start_price) || !Array.isArray(value.samples) || value.samples.length !== 60) return fail();
  let previousTime: number | null = null;
  for (const sample of value.samples) {
    if (!object(sample) || !id(sample.open_time) || Number(sample.open_time) % 60_000 !== 0
      || (previousTime !== null && Number(sample.open_time) !== previousTime + 60_000)) return fail();
    previousTime = Number(sample.open_time);
    for (const key of ['open', 'high', 'low', 'close', 'volume']) {
      if (!decimalFitsStorage(sample[key]) || compareDecimalText(sample[key] as string, '0') === -1
        || (key !== 'volume' && compareDecimalText(sample[key] as string, '0') !== 1)) return fail();
    }
    for (const key of ['open', 'close']) {
      if (compareDecimalText(sample[key] as string, sample.high as string) === 1
        || compareDecimalText(sample[key] as string, sample.low as string) === -1) return fail();
    }
  }
  if (draft.mode === 'follow') {
    const follow = value.follow_preview;
    if (!object(follow) || follow.reference_pair_id !== Number(draft.reference_pair_id)
      || !['historical_replay', 'independent_fallback'].includes(String(follow.kind))
      || !id(follow.range_start) || !id(follow.range_end) || !uint(follow.reference_sample_count)
      || typeof follow.reference_symbol !== 'string' || (follow.warning !== null && typeof follow.warning !== 'string')) return fail();
  }
  return value as DefaultMarketPreview;
}

export const volatilityPresets = [
  { value: '0.0005', label: '低 · 每分钟 0.05%' },
  { value: '0.0015', label: '中 · 每分钟 0.15%' },
  { value: '0.005', label: '高 · 每分钟 0.5%' },
];

export function marketChanges(live: DefaultMarketResponse, draft: DefaultMarketDraft) {
  const before = defaultMarketDraft(live);
  const labels: Partial<Record<keyof DefaultMarketDraft, string>> = {
    mode: '行情模式', initial_price: '初始价格', volatility: '波动率',
    price_min: '价格下限', price_max: '价格上限', mean_reversion: '均值回归强度',
    wick_strength: '影线强度', volume_min: '分钟成交量下限', volume_max: '分钟成交量上限',
    depth_levels: '盘口档数',
    ...(draft.mode === 'follow' ? { reference_pair_id: '参考交易对 ID', follow_multiplier: '跟随强度',
      follow_max_move_ratio: '每分钟变动上限', follow_stale_after_seconds: '参考过期秒数' } : {}),
  };
  const display = (value: string | boolean) => value === 'independent' ? '独立生成' : value === 'follow' ? '跟随参考行情' : String(value || '未设置');
  const changes = (Object.entries(labels) as [keyof DefaultMarketDraft, string][])
    .filter(([key]) => before[key] !== draft[key])
    .map(([key, label]) => ({ label, before: display(before[key]), after: display(draft[key]) }));
  if (!live.enabled) changes.unshift({ label: '默认行情开关', before: '未启用', after: '启用' });
  return changes;
}

/** 仅判断读取快照及新鲜度，不以 active_source 代替租约或金融取价资格证明。 */
export function marketStatus(live: DefaultMarketResponse, pair: MarketPair, now: number): string {
  if (pair.status !== 'active') return '交易对未启用';
  if (live.all_market_paused) return '全部行情已暂停';
  if (live.runtime.error_message) return '运行异常';
  if (live.runtime.active_source === 'none') return live.enabled ? '等待生成器接管' : '默认行情未启用';
  const tick = live.runtime.last_tick_at;
  if (tick === null) return '尚无运行价格';
  if (tick > now) return '行情时间异常';
  if (now - tick > 15_000) return '行情更新延迟';
  if (live.runtime.active_source === 'strategy') return '最近由手工策略生成';
  if (!live.enabled) return '默认行情未启用';
  if (live.runtime.default_version !== live.version) return '等待新版本接续';
  return '最近由默认行情生成';
}
