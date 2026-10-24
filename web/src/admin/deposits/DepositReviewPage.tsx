import { Button, Card, Space, Typography } from '@douyinfe/semi-ui';
import { useQuery } from '@tanstack/react-query';
import { useState, useSyncExternalStore } from 'react';
import { apiRequest } from '../../api/client';
import { authStore } from '../../auth/authStore';
import { PageHeader } from '../../layouts/PageHeader';
import { adminErrorMessage } from '../../shared/adminErrorMessage';
import { AdminSelect, AdminTextInput } from '../../shared/SemiFormControls';
import { TimestampText } from '../../shared/TimestampText';
import { AdminRequestActionBoundary } from '../access';
import { ReceiptReview } from './ReceiptReview';
import './RechargeWorkspace.css';
import { base, labels, parseOrder, parsePage, parseReceipt, reviewReasons, type Receipt } from './contracts';

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
  return <main className="exchange-page admin-action-page"><PageHeader title="链上充值订单与到账核对" description="过期未付组合再次收到款项时，必须核对付款人及订单归属。错额、迟到和分笔款项保留独立证据；人工入账使用实际到账金额并扣除订单手续费。" actions={<Button onClick={() => { void query.refetch(); }}>刷新</Button>} />
    <Space wrap><AdminSelect ariaLabel="充值记录类型" value={mode} optionList={[{ value: 'receipts', label: '链上到账证据' }, { value: 'orders', label: '充值订单' }]} onChange={(value) => { setMode(value); setStatus(''); setOffset(0); }} />
      <AdminSelect ariaLabel="充值状态筛选" value={status} optionList={[{ value: '', label: '全部状态' }, ...(mode === 'receipts' ? ['review', 'observed', 'credited', 'ignored'] : ['pending', 'expired', 'cancelled', 'review', 'credited', 'reversed']).map((value) => ({ value, label: labels[value] }))]} onChange={(value) => { setStatus(value); setOffset(0); }} />
      <form onSubmit={(event) => { event.preventDefault(); setOrderNo(orderSearch.trim()); setOffset(0); }}><Space><AdminTextInput ariaLabel="充值订单号搜索" placeholder="输入用户提供的完整充值订单号" value={orderSearch} onChange={setOrderSearch} /><Button htmlType="submit">查找订单号</Button></Space></form>
</Space>
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
  </main>;
}
