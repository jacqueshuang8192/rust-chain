import { Banner, Button, SideSheet, Space, Tag } from '@douyinfe/semi-ui';
import type { ColumnProps } from '@douyinfe/semi-ui/lib/es/table';
import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { Link } from 'react-router-dom';
import { authStore, type AuthSession } from '../auth/authStore';
import { coinDetail, coinList, coinStatuses, reviewCoin, type CoinApplication, type CoinPage, type CoinStatus } from '../api/coinApplications';
import { useCanAdminRequest } from '../admin/access';
import { PageHeader } from '../layouts/PageHeader';
import { DataTable } from '../shared/DataTable';
import { AdminSelect } from '../shared/SemiFormControls';
import { TimestampText } from '../shared/TimestampText';
import { ConfirmAction } from '../shared/ConfirmAction';
import { adminErrorMessage } from '../shared/adminErrorMessage';
import { CoinApplicationForm } from './CoinApplicationForm';
import './coinApplications.css';

function Status({ value }: { value: CoinStatus }) {
  return <Tag color={value === 'approved' ? 'green' : value === 'rejected' ? 'red' : 'orange'}>{coinStatuses[value]}</Tag>;
}
function History({ scope, revision, canReview, session }: { scope: 'agent' | 'admin'; revision: number; canReview: boolean; session: AuthSession }) {
  const [status, setStatus] = useState<CoinStatus | ''>(scope === 'admin' ? 'pending' : '');
  const [page, setPage] = useState(1);
  const [size, setSize] = useState(20);
  const [reload, setReload] = useState(0);
  const [result, setResult] = useState<CoinPage>({ applications: [], total: 0 });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [detail, setDetail] = useState<CoinApplication | null>(null);
  const [detailError, setDetailError] = useState('');
  const [busy, setBusy] = useState(false);
  const [uncertain, setUncertain] = useState(false);
  const flight = useRef(false);
  const request = useRef(0);
  useEffect(() => {
    const c = new AbortController(); setLoading(true); setError(null); setResult({ applications: [], total: 0 });
    void coinList(scope, status, page, c.signal, size).then((v) => { if (!c.signal.aborted) setResult(v); })
      .catch((e: unknown) => { if (!c.signal.aborted) setError(new Error(adminErrorMessage(e))); })
      .finally(() => { if (!c.signal.aborted) setLoading(false); });
    return () => c.abort();
  }, [scope, status, page, size, reload, revision]);
  useEffect(() => {
    const c = new AbortController(); setDetail(null); setDetailError(''); setUncertain(false);
    if (selected !== null) void coinDetail(scope, selected, c.signal).then((v) => { if (!c.signal.aborted) setDetail(v); })
      .catch((e: unknown) => { if (!c.signal.aborted) setDetailError(adminErrorMessage(e)); });
    return () => { c.abort(); request.current += 1; };
  }, [scope, selected]);
  async function refreshDetail() {
    if (selected === null || flight.current) return;
    const generation = ++request.current;
    flight.current = true; setBusy(true); setDetailError('');
    try {
      const v = await coinDetail(scope, selected);
      if (generation === request.current) { setDetail(v); setUncertain(false); setReload((v) => v + 1); }
    } catch (e) { if (generation === request.current) setDetailError(adminErrorMessage(e)); }
    finally { flight.current = false; if (generation === request.current) setBusy(false); }
  }
  async function review(decision: 'approve' | 'reject', reason: string) {
    if (!detail || flight.current || !canReview || uncertain) throw new Error('请先核对当前申请');
    if (authStore.getSession(scope)?.generation !== session.generation) throw new Error('登录会话已变更');
    flight.current = true; setBusy(true); setDetailError('');
    const generation = ++request.current;
    try {
      const updated = await reviewCoin(detail.id, decision, reason);
      if (generation === request.current) { setDetail(updated); setReload((v) => v + 1); }
    } catch (e) {
      if (generation === request.current) { setUncertain(true); setDetailError('审核结果待核对，请刷新申请详情后继续。'); }
      throw e;
    } finally { flight.current = false; if (generation === request.current) setBusy(false); }
  }
  const columns: ColumnProps<CoinApplication>[] = [
    { key: 'id', dataIndex: 'id', title: '申请编号', width: 110 },
    { key: 'symbol', dataIndex: 'symbol', title: '币种', width: 130 },
    { key: 'name', dataIndex: 'name', title: '名称', width: 150 },
    { key: 'status', title: '审核状态', width: 180, render: (_, row) => <Status value={row.status} /> },
    { key: 'total_supply', dataIndex: 'total_supply', title: '发行总量', width: 220 },
    { key: 'issue_price', title: '发行价格', width: 200, render: (_, row) => `${row.issue_price} ${row.quote_symbol}` },
    { key: 'created_at', title: '申请时间', width: 190, render: (_, row) => <TimestampText value={row.created_at} /> },
    { key: 'review_reason', title: '审核意见 / 驳回原因', width: 240, render: (_, row) => row.review_reason ?? '等待管理员审核' },
    { key: 'actions', title: '操作', fixed: 'right', width: 120, render: (_, row) => <Button size="small" onClick={() => setSelected(row.id)}>查看详情</Button> }
  ];
  if (scope === 'admin') columns.splice(1, 0, { key: 'agent_id', dataIndex: 'agent_id', title: '代理编号', width: 110 });
  return <section className="coin-application-history">
    <div className="coin-application-filters">
      <label id="coin-status-label">审核状态<AdminSelect ariaLabel="审核状态" ariaLabelledBy="coin-status-label" value={status}
        onChange={(v) => { setStatus(v as CoinStatus | ''); setPage(1); }} optionList={[{ value: '', label: '全部状态' }, ...Object.entries(coinStatuses).map(([value, label]) => ({ value, label }))]} /></label>
      <Button loading={loading} onClick={() => setReload((v) => v + 1)}>刷新记录</Button>
    </div>
    <DataTable columns={columns} data={result.applications} rowKey="id" loading={loading} error={error}
      pagination={{ currentPage: page, pageSize: size, total: result.total, onPageChange: setPage, onPageSizeChange: (v) => { setSize(v); setPage(1); } }} />
    <SideSheet title="发行申请详情" visible={selected !== null} width="min(680px, calc(100vw - 32px))" onCancel={() => { if (!flight.current) setSelected(null); }}>
      <div className="coin-application-detail">
        {detailError ? <p role="alert">{detailError}</p> : null}
        <Button loading={busy} onClick={() => void refreshDetail()}>刷新申请详情</Button>
        {detail ? <><Status value={detail.status} /><h3>{detail.symbol} · {detail.name}</h3>
          <dl><dt>申请编号</dt><dd>{detail.id}</dd>
            {scope === 'admin' ? <><dt>代理 / 申请账号</dt><dd>{detail.agent_id} / {detail.agent_admin_id}</dd></> : null}
            <dt>发行总量</dt><dd>{detail.total_supply}</dd><dt>发行价格</dt><dd>{detail.issue_price} {detail.quote_symbol}</dd>
            <dt>资产精度</dt><dd>{detail.precision_scale}</dd><dt>计划上市</dt><dd><TimestampText value={detail.listed_at} /></dd>
            <dt>申请说明</dt><dd>{detail.reason}</dd><dt>申请时间</dt><dd><TimestampText value={detail.created_at} /></dd>
            <dt>审核意见</dt><dd>{detail.review_reason ?? '等待管理员审核'}</dd><dt>审核时间</dt><dd><TimestampText value={detail.reviewed_at} /></dd>
            <dt>发布项目</dt><dd>{detail.project_id ? (scope === 'admin' ? <Link to={`/admin/new-coins/projects/${detail.project_id}`}>项目 {detail.project_id}</Link> : `项目 ${detail.project_id}`) : '尚未发布'}</dd>
          </dl>
          {canReview && detail.status === 'pending' ? <><Banner type="info" description="通过后创建币种并发布预热项目，默认关闭充提与杠杆划转。实际认购、派发和上市仍在项目中心管理。" />
            <Space><ConfirmAction actionText="通过并发布" title={`通过 ${detail.symbol} 的发行申请`} description={`发行 ${detail.total_supply} ${detail.symbol}，价格 ${detail.issue_price} ${detail.quote_symbol}。此操作将创建正式预热项目。`}
              disabled={busy || uncertain} onConfirm={(reason) => review('approve', reason)} />
              <ConfirmAction actionText="驳回申请" title="驳回发行申请" disabled={busy || uncertain} onConfirm={(reason) => review('reject', reason)} /></Space></> : null}
        </> : !detailError ? <p role="status">正在加载申请资料…</p> : null}
      </div>
    </SideSheet>
  </section>;
}
function Content({ scope, session, canReview = false }: { scope: 'agent' | 'admin'; session: AuthSession; canReview?: boolean }) {
  const [revision, setRevision] = useState(0);
  return <main className="exchange-page coin-applications-page"><PageHeader title={scope === 'agent' ? '新币发行申请' : '代理发币审核'} />
    {scope === 'agent' ? <CoinApplicationForm session={session} onSuccess={() => setRevision((v) => v + 1)} /> : !canReview ? <Banner type="info" description="当前账号仅可查看申请记录，无审核权限。" /> : null}
    <h2>{scope === 'agent' ? '我的申请记录' : '发行申请记录'}</h2>
    {scope === 'agent' ? <p>查看当前账号提交的发行资料、审核进度和驳回原因。</p> : null}
    <History scope={scope} session={session} canReview={canReview} revision={revision} />
  </main>;
}
export function AgentCoinApplicationsPage() {
  const session = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('agent'));
  return session ? <Content key={`${session.subject}:${session.generation}`} scope="agent" session={session} /> : <p role="alert">代理会话已失效</p>;
}
export function AdminCoinApplicationsPage() {
  const session = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('admin'));
  const canReview = useCanAdminRequest('/admin/api/v1/agent-coin-applications/1/review', 'POST');
  return session ? <Content key={`${session.subject}:${session.generation}`} scope="admin" session={session} canReview={canReview} /> : <p role="alert">管理员会话已失效</p>;
}
