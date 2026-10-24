import { Banner, Button, SideSheet, Space, Typography } from '@douyinfe/semi-ui';
import { useEffect, useRef, useState } from 'react';
import { apiRequest } from '../../api/client';
import { adminErrorMessage } from '../../shared/adminErrorMessage';
import { ConfirmAction } from '../../shared/ConfirmAction';
import { AdminTextInput } from '../../shared/SemiFormControls';
import { parseSafeInteger } from '../../shared/integer';
import { TimestampText } from '../../shared/TimestampText';
import { AdminRequestActionBoundary } from '../access';
import { base, labels, parseOrder, parsePage, type Order, type Receipt } from './contracts';

function blockers(target: Order, receipt: Receipt): string[] {
  const reasons: string[] = [];
  if (target.network !== receipt.network) reasons.push('网络不一致');
  if (target.asset_kind !== receipt.asset_kind || target.token_address !== receipt.token_address) reasons.push('链上资产身份不一致');
  if (target.address !== receipt.address) reasons.push('收款地址不一致');
  if (Math.floor(target.created_at / 1000) > Math.floor(receipt.block_time / 1000)) reasons.push('转账早于订单创建');
  if (receipt.confirmations < target.required_confirmations) reasons.push(`确认数不足：${receipt.confirmations} / ${target.required_confirmations}`);
  if (target.paid_amount !== null || ['credited', 'reversed'].includes(target.status)) reasons.push('订单已入账或已冲正');
  return reasons;
}

/** 先读取明确目标并显示匹配结果，再由管理员以独立确认提交归属；失败读取撤销旧目标。 */
export function ReceiptReview({ receipt, close, saved }: { receipt: Receipt; close: () => void; saved: () => Promise<void> }) {
  const [search, setSearch] = useState('');
  const [target, setTarget] = useState<Order | null>(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [saving, setSaving] = useState(false);
  const sequence = useRef(0); const flight = useRef(false);
  useEffect(() => () => { sequence.current += 1; }, []);
  const reasons = target ? blockers(target, receipt) : [];
  const eligible = Boolean(target && reasons.length === 0);
  const url = `${base}/deposit-receipts/${receipt.id}/review`;
  async function lookup(value: string) {
    const text = value.trim(); if (!text || flight.current || confirming) return;
    const version = ++sequence.current;
    setLoading(true); setTarget(null); setError('');
    try {
      const id = /^\d+$/.test(text) ? parseSafeInteger(text, 1) : null;
      let order: Order;
      if (/^\d+$/.test(text)) {
        if (id === null) throw new Error('内部编号必须是有效的正整数');
        order = parseOrder(await apiRequest(`${base}/deposit-orders/${id}`));
        if (order.id !== id) throw new Error('返回的订单编号不一致，请重新查询');
      } else {
        const page = parsePage(await apiRequest(`${base}/deposit-orders?limit=2&offset=0&order_no=${encodeURIComponent(text)}`), parseOrder);
        if (page.total !== 1 || page.items.length !== 1 || page.items[0].order_no !== text) throw new Error('未找到唯一订单，请填写完整充值订单号或内部编号');
        order = page.items[0];
      }
      if (version === sequence.current) setTarget(order);
    } catch (cause) { if (version === sequence.current) setError(adminErrorMessage(cause)); }
    finally { if (version === sequence.current) setLoading(false); }
  }
  async function submit(action: 'credit' | 'ignore', reason: string) {
    if (flight.current) throw new Error('正在提交核对结果');
    if (action === 'credit' && (!eligible || !target || loading)) throw new Error('请先核对有效目标订单');
    flight.current = true; setSaving(true);
    try {
      await apiRequest(url, { method: 'POST', body: JSON.stringify({ action, order_id: action === 'credit' ? target!.id : null, reason }) });
      await saved();
    } finally { flight.current = false; setSaving(false); }
  }
  return <SideSheet title={`核对到账证据 #${receipt.id}`} visible motion={false} width="min(880px, 100vw)" maskClosable={false} closeOnEsc={!confirming && !saving} closable={!confirming && !saving} onCancel={() => { if (!confirming && !saving) close(); }} footer={<Button disabled={confirming || saving} onClick={close}>关闭核对</Button>}>
    <div className="deposit-review-sheet">
      <section className="deposit-evidence-summary"><Typography.Title heading={4}>{receipt.amount}</Typography.Title><p>{receipt.network} · {receipt.asset_kind === 'native' ? '原生币' : '代币'} · 确认数 {receipt.confirmations}</p>
        <dl><div><dt>收款地址</dt><dd>{receipt.address}</dd></div><div><dt>合约 / mint</dt><dd>{receipt.token_address || '原生币'}</dd></div><div><dt>交易哈希</dt><dd>{receipt.tx_hash} / {receipt.event_index}</dd></div><div><dt>区块哈希</dt><dd>{receipt.block_hash}</dd></div><div><dt>链上时间</dt><dd><TimestampText value={receipt.block_time} /></dd></div></dl>
      </section>
      <section className="deposit-target-search"><Typography.Title heading={5}>查找归属订单</Typography.Title><p>填写用户提供的完整充值订单号（CD 开头），或后台内部编号。读取只检查匹配条件，不会直接入账。</p>
        <form onSubmit={event => { event.preventDefault(); void lookup(search); }}><label>充值订单号或内部编号<AdminTextInput ariaLabel="目标充值订单编号" placeholder="CD… 或内部编号" disabled={saving || confirming} value={search} onChange={value => { sequence.current += 1; setSearch(value); setTarget(null); setError(''); setLoading(false); }} /></label><Button htmlType="submit" theme="solid" loading={loading} disabled={loading || saving || confirming || !search.trim()}>读取并核对目标订单</Button></form>
        {receipt.order_id ? <Button disabled={loading || saving || confirming} onClick={() => { const value = String(receipt.order_id); setSearch(value); void lookup(value); }}>读取候选订单 #{receipt.order_id}</Button> : <p className="muted">此到账暂无关联候选订单，请向付款人取得订单号。</p>}
        <p className="muted">候选订单只供核实，请确认付款来源与用户身份后再决定归属。</p>
        {error ? <p role="alert" className="deposit-review-error">{error}</p> : null}
      </section>
      {target ? <section className="deposit-target-result" aria-label="订单核对结果"><Typography.Title heading={5}>{target.order_no}</Typography.Title>
        <p>用户编号：{target.user_id} · {labels[target.status] || target.status}</p>
        <dl><div><dt>应付数量</dt><dd>{target.amount} {target.asset_symbol}</dd></div><div><dt>实际到账</dt><dd>{receipt.amount} {target.asset_symbol}</dd></div><div><dt>扣除手续费</dt><dd>{target.fee_amount} {target.asset_symbol}</dd></div><div><dt>目标网络 / 地址</dt><dd>{target.network} / {target.address}</dd></div><div><dt>创建 / 到期</dt><dd><TimestampText value={target.created_at} /> / <TimestampText value={target.expires_at} /></dd></div></dl>
        {reasons.length ? <Banner type="warning" description={`不符合入账条件：${reasons.join('；')}`} /> : <Banner type="success" description="基础条件匹配。请人工确认付款归属后，再提交入账。" />}
      </section> : null}
      <AdminRequestActionBoundary endpoint={url} method="POST"><Space wrap>
        <ConfirmAction onOpenChange={setConfirming} actionText="确认归属并入账" title="按实际到账数量入账" disabled={!eligible || loading || saving || confirming} description={target ? `用户 ${target.user_id}，订单 ${target.order_no}：实际到账 ${receipt.amount} ${target.asset_symbol}，扣除手续费 ${target.fee_amount} ${target.asset_symbol}。填写核对依据；金额或时间不同也需要明确说明。` : ''} onConfirm={reason => submit('credit', reason)} />
        <ConfirmAction onOpenChange={setConfirming} actionText="排除此到账证据" title="确认排除到账证据" dangerous disabled={saving || confirming} description="此操作只标记不入账，不会退款或发送链上资金。请写明核对依据。" onConfirm={reason => submit('ignore', reason)} />
      </Space></AdminRequestActionBoundary>
    </div>
  </SideSheet>;
}
