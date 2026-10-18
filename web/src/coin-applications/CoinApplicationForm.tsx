import { buildPolicy, emptyPolicyDraft } from './issuancePolicy';
import { IssuancePolicyFields } from './IssuancePolicyFields';
import { IssuancePolicySummary } from './IssuancePolicySummary';
import { Banner, Button, Modal, TextArea } from '@douyinfe/semi-ui';
import { useEffect, useRef, useState } from 'react';
import { type AuthSession, authStore } from '../auth/authStore';
import { coinOptions, submitCoin, validateCoinInput, type CoinAsset, type CoinInput } from '../api/coinApplications';
import { ApiError } from '../api/client';
import { AdminSelect, AdminTextInput } from '../shared/SemiFormControls';
import { requiredSafeInteger } from '../shared/integer';
import { decimalFitsPrecision } from '../shared/decimal';
import { adminErrorMessage } from '../shared/adminErrorMessage';

type Intent = { key: string; input: CoinInput };
const empty = { symbol: '', name: '', precision: '8', supply: '', price: '', quote: '', listing: '', reason: '' };
function load(key: string): { intent: Intent | null; error: string } {
  try {
    const raw = sessionStorage.getItem(key);
    if (!raw || raw === 'null') return { intent: null, error: '' };
    const value = JSON.parse(raw) as Intent;
    if (typeof value.key !== 'string' || !value.key) throw new Error();
    validateCoinInput(value.input);
    return { intent: value, error: '' };
  } catch { return { intent: null, error: '无法恢复未确认的发币申请，请先核对申请记录并恢复会话存储。' }; }
}
function save(key: string, intent: Intent | null) {
  const raw = JSON.stringify(intent);
  sessionStorage.setItem(key, raw);
  if (sessionStorage.getItem(key) !== raw) throw new Error('无法保存申请标识，本次未发送');
}
export function CoinApplicationForm({ session, onSuccess }: { session: AuthSession; onSuccess: () => void }) {
  const storageKey = `coin-application:${session.subject}:${session.generation}`;
  const [initial] = useState(() => load(storageKey));
  const [intent, setIntent] = useState(initial.intent);
  const [draft, setDraft] = useState(empty);
  const [policyDraft, setPolicyDraft] = useState(emptyPolicyDraft);
  const [assets, setAssets] = useState<CoinAsset[]>([]);
  const [reload, setReload] = useState(0);
  const [optionsError, setOptionsError] = useState('');
  const [error, setError] = useState(initial.error);
  const [receipt, setReceipt] = useState('');
  const [confirmation, setConfirmation] = useState<CoinInput | null>(null);
  const [busy, setBusy] = useState(false);
  const flight = useRef(false);
  useEffect(() => {
    const c = new AbortController(); setAssets([]); setOptionsError('');
    void coinOptions(c.signal).then((v) => { if (!c.signal.aborted) setAssets(v); }).catch((e: unknown) => { if (!c.signal.aborted) setOptionsError(adminErrorMessage(e)); });
    return () => c.abort();
  }, [reload]);
  function prepare() {
    try {
      const input = intent?.input ?? validateCoinInput({ symbol: draft.symbol, name: draft.name,
        precision_scale: requiredSafeInteger(draft.precision, '资产精度', 0, 18), total_supply: draft.supply, issue_price: draft.price,
        quote_asset_id: requiredSafeInteger(draft.quote, '计价资产', 1), listed_at: new Date(policyDraft.nodes.listing.local).getTime(), reason: draft.reason,
        issuance_policy: buildPolicy(policyDraft, assets.find(v => String(v.asset_id) === draft.quote)?.precision_scale ?? 18) });
      if (!intent) {
        const asset = assets.find((v) => v.asset_id === input.quote_asset_id);
        if (!asset) throw new Error('请重新加载计价资产');
        if (asset.symbol === input.symbol) throw new Error('新币不能与计价资产相同');
        if (!decimalFitsPrecision(input.issue_price, asset.precision_scale)) throw new Error('发行价格超过计价资产精度');
      }
      setConfirmation(input); setError('');
    } catch (e) { setError(adminErrorMessage(e)); }
  }
  async function submit() {
    if (flight.current || !confirmation) return;
    flight.current = true; setBusy(true); setError('');
    try {
      if (authStore.getSession('agent')?.generation !== session.generation) throw new Error('会话已变更，请重新进入页面');
      const current = intent ?? { key: crypto.randomUUID(), input: confirmation };
      save(storageKey, current); setIntent(current);
      const row = await submitCoin(current.input, current.key);
      save(storageKey, null); setIntent(null); setConfirmation(null); setDraft(empty); setPolicyDraft(emptyPolicyDraft());
      setReceipt(`申请 ${row.id} 已提交，可在“我的申请记录”查看审核进度。`); onSuccess();
    } catch (e) {
      if (e instanceof ApiError && [400, 422].includes(e.status)) {
        try { save(storageKey, null); setIntent(null); } catch { /* Keep recovery locked when storage is unavailable. */ }
      }
      setError(adminErrorMessage(e, '提交结果未确认，请按原申请重试或前往“我的申请记录”核对'));
    } finally { flight.current = false; setBusy(false); }
  }
  function field(key: keyof typeof empty, label: string, type?: string) {
    return <label>{label}<AdminTextInput ariaLabel={label} value={draft[key]} type={type} disabled={busy || Boolean(intent)} onChange={(v) => setDraft({ ...draft, [key]: v })} /></label>;
  }
  return <section className="coin-application-form">
    <Banner type="info" description="填写发行资料并提交审核。通过后创建资产与钱包，按各节点的手动或自动设置推进。派发始终由代理分批确认。" />
    {intent ? <Banner type="warning" description={`申请 ${intent.input.symbol} 的提交结果待核对，已保留原始资料与请求标识。`} /> : null}
    {!intent ? <div className="coin-application-fields">
      {field('symbol', '币种代码')}{field('name', '币种名称')}{field('precision', '资产精度')}{field('supply', '发行总量')}
      <label id="coin-quote-label">计价资产<AdminSelect ariaLabel="计价资产" ariaLabelledBy="coin-quote-label" value={draft.quote} disabled={busy}
        onChange={(quote) => setDraft({ ...draft, quote })} optionList={assets.map((v) => ({ value: String(v.asset_id), label: `${v.symbol}（${v.precision_scale} 位小数）` }))} /></label>
      {field('price', '申购价格')}
      <label className="coin-application-wide">申请说明<TextArea aria-label="申请说明" value={draft.reason} disabled={busy} onChange={(reason) => setDraft({ ...draft, reason })} /></label>
    </div> : null}
    {!intent ? <IssuancePolicyFields value={policyDraft} onChange={setPolicyDraft} disabled={busy} /> : null}
    {optionsError ? <p role="alert">{optionsError} <Button onClick={() => setReload((v) => v + 1)}>重新加载资产</Button></p> : null}
    {error ? <p role="alert">{error}</p> : null}{receipt ? <p role="status">{receipt}</p> : null}
    <Button theme="solid" disabled={busy || Boolean(initial.error)} onClick={prepare}>{intent ? '核对原申请' : '提交发行申请'}</Button>
    <Modal title="确认发行申请" visible={Boolean(confirmation)} motion={false} maskClosable={false} closeOnEsc={!busy}
      width="min(560px, calc(100vw - 32px))" onCancel={() => { if (!busy) setConfirmation(null); }} onOk={() => void submit()} confirmLoading={busy}
      okText={intent ? '按原申请重试' : '确认提交'} okButtonProps={{ 'aria-label': intent ? '按原申请重试' : '确认提交' }} cancelButtonProps={{ 'aria-label': '取消', disabled: busy }}>
      {confirmation ? <><p>{confirmation.symbol} · {confirmation.name}</p><p>发行总量：{confirmation.total_supply}；价格：{confirmation.issue_price}</p><p>计划上市：{new Date(confirmation.listed_at).toLocaleString()}</p><p>申请说明：{confirmation.reason}</p>{confirmation.issuance_policy ? <IssuancePolicySummary policy={confirmation.issuance_policy} quote={assets.find(v => v.asset_id === confirmation.quote_asset_id)?.symbol} /> : <p>历史申请按原锁仓规则处理。</p>}<p>申请提交后不能修改，请核对全部规则。</p></> : null}
      {error ? <p role="alert">{error}</p> : null}
    </Modal>
  </section>;
}
