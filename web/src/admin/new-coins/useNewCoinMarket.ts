import { useCallback, useEffect, useRef, useState } from 'react';
import { ApiError, apiRequest } from '../../api/client';
import { adminErrorMessage } from '../../shared/adminErrorMessage';
import { defaultMarketPayload, validateDefaultMarketDraft } from '../resources/actions/defaultMarket/model';
import type { DefaultMarketDraft, DefaultMarketPreview, DefaultMarketResponse } from '../resources/actions/defaultMarket/types';
import { checkLiveMarket, checkMarketDraft, checkMarketPreview, hydrateMarketDraft, type MarketDraft, type MarketPair } from './marketSetupModel';

export function useNewCoinMarket(projectId: number, pair: MarketPair, initialSaved: MarketDraft | null) {
  const endpoint = `/admin/api/v1/new-coins/${projectId}/market-setup`;
  const marketEndpoint = `/admin/api/v1/market-pairs/${pair.id}/default-generator`;
  const [live, setLive] = useState<DefaultMarketResponse | null>(null);
  const [checkedAt, setCheckedAt] = useState(Date.now);
  const [draft, setDraft] = useState<DefaultMarketDraft | null>(null);
  const [saved, setSaved] = useState(initialSaved);
  const [baseline, setBaseline] = useState('');
  const [baseVersion, setBaseVersion] = useState(0);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [conflict, setConflict] = useState(false);
  const [uncertain, setUncertain] = useState(false);
  const [preview, setPreview] = useState<DefaultMarketPreview | null>(null);
  const [notice, setNotice] = useState('');
  const epoch = useRef(0);
  const singleFlight = useRef(false);
  const revision = saved?.revision ?? 0;
  const dirty = draft !== null && JSON.stringify(draft) !== baseline;
  const invalid = draft ? validateDefaultMarketDraft(draft, pair.price_precision, pair.qty_precision) : '请先加载行情配置';
  const readLive = useCallback(async (signal?: AbortSignal) =>
    checkLiveMarket(await apiRequest<unknown>(marketEndpoint, { signal }), pair), [marketEndpoint, pair]);

  useEffect(() => {
    const controller = new AbortController();
    let active = true;
    void readLive(controller.signal).then(value => {
      if (!active) return;
      const own = initialSaved?.pair_id === pair.id && initialSaved.published_version === null ? initialSaved : null;
      const next = hydrateMarketDraft(value, own);
      setLive(value); setCheckedAt(Date.now()); setDraft(next); setBaseline(JSON.stringify(next));
      setBaseVersion(own && own.published_version === null ? own.base_version : value.version);
      setConflict(Boolean(own && own.published_version === null && own.base_version !== value.version));
    }).catch(cause => { if (active) setError(adminErrorMessage(cause, '行情加载失败')); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; epoch.current += 1; controller.abort(); };
  }, [initialSaved, pair.id, readLive]);

  function edit<K extends keyof DefaultMarketDraft>(key: K, value: DefaultMarketDraft[K]) {
    if (singleFlight.current) return;
    epoch.current += 1; setPreview(null); setNotice('');
    setDraft(current => current ? { ...current, [key]: value } : null);
  }

  async function action(work: () => Promise<void>, mutation = false) {
    if (singleFlight.current) throw new Error('请求正在处理中');
    singleFlight.current = true; setBusy(true); setError('');
    try { await work(); }
    catch (cause) {
      if (cause instanceof ApiError && cause.status === 409) setConflict(true);
      if (mutation && (!(cause instanceof ApiError) || cause.status >= 500)) setUncertain(true);
      setError(adminErrorMessage(cause, '操作失败，请重新查询状态')); throw cause;
    } finally { singleFlight.current = false; setBusy(false); }
  }

  function ensureEditable() {
    if (!draft || !live || loading || conflict || uncertain || invalid) throw new Error(invalid || '请重新加载并核对最新配置');
    return draft;
  }

  async function runPreview() {
    const value = ensureEditable();
    const ticket = ++epoch.current;
    setPreview(null);
    await action(async () => {
      const { expected_version, initial_price, config } = defaultMarketPayload(value, baseVersion);
      const body = { expected_version, initial_price, config };
      const result = checkMarketPreview(await apiRequest(`${marketEndpoint}/preview`, { method: 'POST', body: JSON.stringify(body) }), pair.id, baseVersion + 1, value);
      if (ticket === epoch.current) setPreview(result);
    });
  }

  async function save(reason: string) {
    const value = ensureEditable();
    await action(async () => {
      const { initial_price, config } = defaultMarketPayload(value, baseVersion);
      const next = checkMarketDraft(await apiRequest(endpoint, { method: 'PATCH', body: JSON.stringify({
        pair_id: pair.id, expected_revision: revision, expected_market_version: baseVersion, initial_price, config, reason,
      }) }));
      if (next.pair_id !== pair.id || next.revision !== revision + 1 || next.base_version !== baseVersion) throw new Error('草稿返回版本不符，请查询最新状态');
      setSaved(next); setBaseline(JSON.stringify(value)); setNotice('草稿已保存，线上行情未改变');
    }, true);
  }

  async function publish(reason: string) {
    ensureEditable();
    if (!saved || saved.pair_id !== pair.id || saved.published_version !== null || dirty || !preview || pair.status !== 'active') throw new Error('请先保存当前草稿并完成预览，确认交易对已启用');
    await action(async () => {
      const result = await apiRequest<{ revision: number; published_version: number; replayed: boolean }>(`${endpoint}/publish`, {
        method: 'POST', body: JSON.stringify({ pair_id: pair.id, expected_revision: saved.revision, reason }),
      });
      if (result.revision !== saved.revision || result.published_version !== baseVersion + 1 || typeof result.replayed !== 'boolean') throw new Error('发布返回版本不符，请查询最新状态');
      setSaved({ ...saved, published_version: result.published_version }); setBaseVersion(result.published_version);
      setPreview(null); setNotice(`已发布 V${result.published_version}，实际运行状态请刷新核对`);
      try { setLive(await readLive()); setCheckedAt(Date.now()); } catch { setError('发布已成功，运行状态读取失败，请刷新状态'); }
    }, true);
  }

  async function refreshStatus() {
    await action(async () => {
      const value = await readLive();
      setLive(value); setCheckedAt(Date.now());
      if (value.version !== baseVersion) {
        epoch.current += 1; setPreview(null); setConflict(true);
      }
    });
  }

  function startFromLive() {
    if (!live || singleFlight.current || uncertain) return;
    const next = hydrateMarketDraft(live, null);
    epoch.current += 1; setPreview(null); setDraft(next); setBaseVersion(live.version);
    // Explicitly adopting live values creates a new draft, even when the parameters are equal.
    setBaseline(''); setConflict(false); setError(''); setNotice('');
  }

  async function pause(reason: string) {
    if (!live || dirty || uncertain || conflict) throw new Error('请先处理未保存或冲突的草稿');
    await action(async () => {
      const result = checkLiveMarket(await apiRequest(`${marketEndpoint}/pause-all`, { method: 'PATCH', body: JSON.stringify({
        expected_version: live.version, all_market_paused: !live.all_market_paused, reason,
      }) }), pair);
      setLive(result); setCheckedAt(Date.now()); setPreview(null);
      if (saved?.pair_id === pair.id && saved.published_version === null) setConflict(true);
      else setBaseVersion(result.version);
    }, true);
  }

  return { live, draft, saved, loading, busy, dirty, invalid, error, conflict, uncertain, preview, notice,
    baseVersion, checkedAt, edit, runPreview, save, publish, refreshStatus, startFromLive, pause };
}
