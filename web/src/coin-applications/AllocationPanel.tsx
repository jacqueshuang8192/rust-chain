import { Banner, Button, Modal, TextArea } from '@douyinfe/semi-ui';
import { useRef, useState } from 'react';
import type { AgentTeamOrder } from '../api/agentTeamOrders';
import { allocateTokens, type AllocationInput } from '../api/tokenIssuance';
import { authStore } from '../auth/authStore';
import { ApiError } from '../api/client';
import { AdminTextInput } from '../shared/SemiFormControls';
import { addDecimalText, canonicalDecimalText, compareDecimalText, decimalFitsPrecision, decimalFitsStorage, multiplyDecimalText } from '../shared/decimal';
import { adminErrorMessage } from '../shared/adminErrorMessage';

export function canAllocate(row: AgentTeamOrder): boolean {
  return row.issuance_version === 2 && row.project_lifecycle_status === 'distribution' && row.status === 'pending' && row.settlement_mode === 'manual_distribution';
}
export function allocationPreview(row: AgentTeamOrder, percent: string) {
  if (!canAllocate(row) || !decimalFitsStorage(percent) || compareDecimalText(percent, '0') === -1 || compareDecimalText(percent, '100') === 1) throw new Error('请选择待派发订单，并填写 0 至 100 的派发比例');
  const precision = Number(row.base_asset_precision), quotePrecision = Number(row.quote_asset_precision);
  if (!Number.isInteger(precision) || precision < 0 || precision > 18 || !Number.isInteger(quotePrecision) || quotePrecision < 0 || quotePrecision > 18) throw new Error('订单资产精度缺失，请刷新');
  const raw = multiplyDecimalText(multiplyDecimalText(String(row.requested_quantity), percent) ?? '', '0.01');
  if (raw === null) throw new Error('派发数量无法计算');
  const [whole, fraction = ''] = raw.split('.');
  const quantity = canonicalDecimalText(`${whole}${precision && fraction ? `.${fraction.slice(0, precision)}` : ''}`);
  if (quantity === null) throw new Error('派发数量无效');
  const payment = multiplyDecimalText(quantity, String(row.issue_price));
  const refund = payment === null ? null : addDecimalText(String(row.quote_amount), `-${payment}`);
  if (payment === null || refund === null || !decimalFitsPrecision(payment, quotePrecision)) throw new Error(`订单 ${row.id} 的付款精度不支持该比例，请调整`);
  return { id: row.id, user: row.email || `用户 ${row.user_id}`, symbol: String(row.symbol), quote: String(row.quote_symbol), quantity, payment, refund };
}
type Intent = { input: AllocationInput; preview: ReturnType<typeof allocationPreview>[] };
function restore(key: string): { intent: Intent | null; error: string } {
  try { const text = sessionStorage.getItem(key); if (!text || text === 'null') return { intent: null, error: '' };
    const v = JSON.parse(text) as Intent;
    if (!v.input?.idempotency_key || !Array.isArray(v.input.subscription_ids) || !Array.isArray(v.preview) || !v.input.subscription_ids.length || v.input.subscription_ids.length > 100 || !decimalFitsStorage(v.input.percent)) throw new Error();
    return { intent: v, error: '' };
  } catch { return { intent: null, error: '未确认批次恢复失败，请先核对派发记录并恢复会话存储。' }; }
}
export function AllocationPanel({ rows, onSuccess }: { rows: AgentTeamOrder[]; onSuccess: () => void }) {
  const session = authStore.getSession('agent');
  const storageKey = `token-allocation:${session?.subject}:${session?.generation}`;
  const [initial] = useState(() => restore(storageKey));
  const [intent, setIntent] = useState(initial.intent), [percent, setPercent] = useState('100'), [reason, setReason] = useState('');
  const [preview, setPreview] = useState<Intent['preview'] | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState(initial.error), [receipt, setReceipt] = useState('');
  const flight = useRef(false);
  function prepare() { try { const data = intent?.preview ?? rows.map(row => allocationPreview(row, percent)); if (!data.length) throw new Error('请先勾选本页需要派发的下级订单'); setPreview(data); setError(''); } catch (e) { setError(adminErrorMessage(e)); } }
  async function confirm() {
    if (flight.current || !preview) return;
    flight.current = true; setBusy(true); setError('');
    try {
      if (authStore.getSession('agent')?.generation !== session?.generation) throw new Error('代理会话已变化');
      if (!intent && (!reason.trim() || [...reason.trim()].length > 500)) throw new Error('请填写 1 至 500 字的派发说明');
      const current = intent ?? { input: { subscription_ids: preview.map(v => v.id), percent, reason: reason.trim(), idempotency_key: crypto.randomUUID() }, preview };
      const text = JSON.stringify(current); sessionStorage.setItem(storageKey, text); if (sessionStorage.getItem(storageKey) !== text) throw new Error('无法保存派发标识，本次未发送');
      setIntent(current); const result = await allocateTokens(current.input);
      sessionStorage.setItem(storageKey, 'null'); setIntent(null); setPreview(null); setReason('');
      setReceipt(`已完成 ${result.results.length} 笔订单派发和退款。可重新勾选下一批并设置不同的比例。`); onSuccess();
    } catch (e) {
      if (e instanceof ApiError && [400, 422].includes(e.status)) { sessionStorage.setItem(storageKey, 'null'); setIntent(null); }
      setError(adminErrorMessage(e, '派发结果待核对，请按原批次重试'));
    } finally { flight.current = false; setBusy(false); }
  }
  return <section className="allocation-panel">
    <div className="allocation-controls"><p>{intent ? `待核对批次：${intent.input.subscription_ids.length} 笔` : `本页已选择 ${rows.length} 笔待派发订单`}</p>
      <label>本批派发比例（%）<AdminTextInput ariaLabel="本批派发比例" value={intent?.input.percent ?? percent} disabled={busy || !!intent} onChange={setPercent} /></label>
      <Button theme="solid" disabled={busy || !!initial.error || (!intent && !rows.length)} onClick={prepare}>{intent ? '核对原派发批次' : '预览本批派发'}</Button>
    </div><p className="muted">每笔订单只结算一次；按代币精度向下取整，未获配部分原路退款。不同批次可用不同派发比例。</p>
    {error ? <Banner type="danger" description={error} /> : null}{receipt ? <p role="status">{receipt}</p> : null}
    <Modal title="确认本批派发与退款" visible={preview !== null} width="min(760px, calc(100vw - 32px))" motion={false} maskClosable={false} closeOnEsc={!busy}
      onCancel={() => { if (!busy) setPreview(null); }} onOk={() => void confirm()} okText={intent ? '按原批次重试' : '确认派发'} confirmLoading={busy} okButtonProps={{ 'aria-label': intent ? '按原批次重试' : '确认派发' }} cancelButtonProps={{ disabled: busy, 'aria-label': '取消派发确认' }}>
      <div className="allocation-preview">{preview?.map(v => <div key={v.id}><strong>{v.user} · 订单 {v.id}</strong><p>派发 {v.quantity} {v.symbol}；实付 {v.payment} {v.quote}；退款 {v.refund} {v.quote}</p></div>)}</div>
      <label>派发说明<TextArea aria-label="派发说明" value={intent?.input.reason ?? reason} disabled={busy || !!intent} onChange={setReason} /></label>
      {error ? <p role="alert">{error}</p> : null}
    </Modal>
  </section>;
}
