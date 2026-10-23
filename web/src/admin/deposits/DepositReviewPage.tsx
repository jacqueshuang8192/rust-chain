import { Button, Card, Modal, Space, Typography } from '@douyinfe/semi-ui';
import { useQuery } from '@tanstack/react-query';
import { useRef, useState, useSyncExternalStore } from 'react';
import { Link } from 'react-router-dom';
import { apiRequest } from '../../api/client';
import { authStore } from '../../auth/authStore';
import { PageHeader } from '../../layouts/PageHeader';
import { adminErrorMessage } from '../../shared/adminErrorMessage';
import { ConfirmAction } from '../../shared/ConfirmAction';
import { AdminSelect, AdminTextInput } from '../../shared/SemiFormControls';
import { parseSafeInteger } from '../../shared/integer';
import { TimestampText } from '../../shared/TimestampText';
import { AdminRequestActionBoundary } from '../access';
import { base, labels, parseOrder, parsePage, parseReceipt, reviewReasons, type Order, type Receipt } from './contracts';

export function DepositReviewPage() {
  const session = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('admin'));
  return <ReviewList key={session?.generation || 'anonymous'} identity={session?.generation || ''} />;
}
function ReviewList({ identity }: { identity: string }) {
  const [orderSearch, setOrderSearch] = useState(''); const [orderNo, setOrderNo] = useState('');
  const [mode, setMode] = useState('receipts'); const [status, setStatus] = useState('review'); const [offset, setOffset] = useState(0); const [selected, setSelected] = useState<Receipt | null>(null);
  const query = useQuery({ queryKey: ['timed-deposit-records', identity, mode, status, offset, orderNo], queryFn: async () => {
    const data = await apiRequest<unknown>(`${base}/deposit-${mode}?limit=20&offset=${offset}${status ? `&status=${encodeURIComponent(status)}` : ''}${orderNo ? `&order_no=${encodeURIComponent(orderNo)}` : ''}`);
    return mode === 'receipts' ? parsePage(data, parseReceipt) : parsePage(data, parseOrder);
  }, enabled: Boolean(identity), refetchOnWindowFocus: false });
  return <><PageHeader title="链上充值订单与到账核对" description="过期未付组合再次收到款项时，必须核对付款人及订单归属。错额、迟到和分笔款项保留独立证据；人工入账使用实际到账金额并扣除订单手续费。" actions={<Button onClick={() => { void query.refetch(); }}>刷新</Button>} />
    <Space wrap><AdminSelect ariaLabel="充值记录类型" value={mode} optionList={[{ value: 'receipts', label: '链上到账证据' }, { value: 'orders', label: '充值订单' }]} onChange={(value) => { setMode(value); setStatus(''); setOffset(0); }} />
      <AdminSelect ariaLabel="充值状态筛选" value={status} optionList={[{ value: '', label: '全部状态' }, ...(mode === 'receipts' ? ['review', 'observed', 'credited', 'ignored'] : ['pending', 'expired', 'review', 'credited', 'reversed']).map((value) => ({ value, label: labels[value] }))]} onChange={(value) => { setStatus(value); setOffset(0); }} />
      <form onSubmit={(event) => { event.preventDefault(); setOrderNo(orderSearch.trim()); setOffset(0); }}><Space><AdminTextInput ariaLabel="充值订单号搜索" placeholder="输入用户提供的完整充值订单号" value={orderSearch} onChange={setOrderSearch} /><Button htmlType="submit">查找订单号</Button></Space></form>
      <Link to="/admin/wallet/deposit-settings">监听与地址配置</Link><Link to="/admin/wallet/deposits">已入账流水与冲正</Link></Space>
    {query.error ? <p role="alert">{adminErrorMessage(query.error)}</p> : null}
    {query.isPending ? <p>正在加载记录…</p> : null}
    {query.data?.items.map((row) => <Card key={row.id} style={{ marginTop: 16, overflowWrap: 'anywhere' }} title={`${'order_no' in row ? row.order_no : `到账证据 #${row.id}`} · ${labels[row.status] || row.status}`}>
      {'order_no' in row ? <><p>内部订单编号：{row.id} · 用户编号：{row.user_id}</p><p>应付：{row.amount} {row.asset_symbol} · 实际到账：{row.paid_amount ?? '尚未入账'} · 手续费：{row.fee_amount} {row.asset_symbol}</p><p>创建：<TimestampText value={row.created_at} /> · 到期：<TimestampText value={row.expires_at} /></p></> : <><p>实际数量：{row.amount} · {row.asset_kind === 'native' ? '原生币' : `合约 / mint：${row.token_address}`}</p><p>链上时间：<TimestampText value={row.block_time} /> · 确认：{row.confirmations} · 区块：{row.block_height}</p><p>事件：{row.tx_hash} / {row.event_index}</p><p>核对原因：{row.review_reason ? reviewReasons[row.review_reason] || row.review_reason : '无'} · 候选订单编号：{row.order_id ?? '无'}</p>{row.review_note ? <p>核对说明：{row.review_note}</p> : null}
      {['review', 'observed'].includes(row.status) ? <AdminRequestActionBoundary endpoint={`${base}/deposit-receipts/${row.id}/review`} method="POST"><Button onClick={() => setSelected(row)}>核对到账归属</Button></AdminRequestActionBoundary> : null}</>}
      <p>网络：{row.network} · 收款地址：{row.address}</p>
    </Card>)}
    {query.data && !query.data.items.length ? <p>暂无符合条件的记录</p> : null}
    <Space style={{ marginTop: 16 }}><Button disabled={offset === 0 || query.isFetching} onClick={() => setOffset(Math.max(0, offset - 20))}>上一页</Button><Typography.Text>共 {query.data?.total ?? '—'} 条</Typography.Text><Button disabled={!query.data || offset + query.data.items.length >= query.data.total || query.isFetching} onClick={() => setOffset(offset + 20)}>下一页</Button></Space>
    {selected ? <ReceiptReview key={selected.id} receipt={selected} close={() => setSelected(null)} saved={async () => { setSelected(null); await query.refetch(); }} /> : null}
  </>;
}
function ReceiptReview({ receipt, close, saved }: { receipt: Receipt; close: () => void; saved: () => Promise<void> }) {
  const [targetId, setTargetId] = useState(''); const [target, setTarget] = useState<Order | null>(null); const [error, setError] = useState(''); const [loading, setLoading] = useState(false);
  const sequence = useRef(0);
  const id = parseSafeInteger(targetId, 1); const url = `${base}/deposit-receipts/${receipt.id}/review`;
  const eligible = Boolean(target && target.id === id && target.network === receipt.network && target.address === receipt.address && target.asset_kind === receipt.asset_kind && target.token_address === receipt.token_address && Math.floor(target.created_at / 1000) <= Math.floor(receipt.block_time / 1000) && receipt.confirmations >= target.required_confirmations && !['credited', 'reversed'].includes(target.status));
  return <Modal title={`核对到账证据 #${receipt.id}`} visible onCancel={close} footer={null} width={760} style={{ maxWidth: 'calc(100vw - 32px)' }} maskClosable={false}>
    <div style={{ display: 'grid', gap: 14, overflowWrap: 'anywhere' }}>
      <p>网络：{receipt.network} · 实际数量：{receipt.amount} · 事件序号：{receipt.event_index}</p><p>合约 / mint：{receipt.token_address || '原生币'}</p><p>地址：{receipt.address}</p><p>交易：{receipt.tx_hash}</p><p>区块哈希：{receipt.block_hash}</p><p>链上时间：<TimestampText value={receipt.block_time} /> · 确认数：{receipt.confirmations}</p>
      <p>请先核实付款来源，再明确选择归属订单。候选提示不会自动填入归属。</p>
      <label>目标内部订单编号<AdminTextInput ariaLabel="目标充值订单编号" value={targetId} onChange={(value) => { sequence.current += 1; setTargetId(value); setTarget(null); setLoading(false); }} /></label>
      <Button disabled={loading || id === null} onClick={async () => { const version = ++sequence.current; setLoading(true); setError(''); try { const order = parseOrder(await apiRequest(`${base}/deposit-orders/${id}`)); if (version === sequence.current) setTarget(order); } catch (cause) { if (version === sequence.current) setError(adminErrorMessage(cause)); } finally { if (version === sequence.current) setLoading(false); } }}>读取并核对目标订单</Button>
      {error ? <p role="alert">{error}</p> : null}
      {target ? <><p>订单：{target.order_no} · 用户编号：{target.user_id} · 状态：{labels[target.status] || target.status}</p><p>币种：{target.asset_symbol} · 网络：{target.network} · 应付：{target.amount} · 实际到账：{receipt.amount} · 扣费：{target.fee_amount}</p><p>目标地址：{target.address}</p><p>创建：<TimestampText value={target.created_at} /> · 到期：<TimestampText value={target.expires_at} /></p>{!eligible ? <p role="alert">网络、资产身份、地址、创建时间、确认数或订单状态不符合入账条件。</p> : null}</> : null}
      <AdminRequestActionBoundary endpoint={url} method="POST"><Space wrap>
        <ConfirmAction actionText="确认归属并入账" title="按实际到账数量入账" disabled={!eligible || loading} description={target ? `用户 ${target.user_id}，订单 ${target.order_no}：实际到账 ${receipt.amount} ${target.asset_symbol}，扣除手续费 ${target.fee_amount} ${target.asset_symbol}。填写核对依据；金额或时间不同也需要明确说明。` : ''} onConfirm={async (reason) => { if (!eligible || !target) throw new Error('请先核对有效目标订单'); await apiRequest(url, { method: 'POST', body: JSON.stringify({ action: 'credit', order_id: target.id, reason }) }); await saved(); }} />
        <ConfirmAction actionText="排除此到账证据" title="确认排除到账证据" dangerous description="此操作只标记不入账，不会退款或发送链上资金。请写明核对依据。" onConfirm={async (reason) => { await apiRequest(url, { method: 'POST', body: JSON.stringify({ action: 'ignore', order_id: null, reason }) }); await saved(); }} />
      </Space></AdminRequestActionBoundary>
    </div>
  </Modal>;
}
