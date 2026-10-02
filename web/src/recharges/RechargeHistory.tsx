import { IconRefresh, IconSearch } from '@douyinfe/semi-icons';
import { Button, Select, Space } from '@douyinfe/semi-ui';
import type { ColumnProps } from '@douyinfe/semi-ui/lib/es/table';
import { useEffect, useState, type ReactNode } from 'react';
import { listRecharges, type RechargePage, type RechargeRecord, type RechargeStatus } from '../api/recharges';
import { DataTable } from '../shared/DataTable';
import { AdminTextInput } from '../shared/SemiFormControls';
import { TimestampText } from '../shared/TimestampText';
import { requiredSafeInteger } from '../shared/integer';
import { adminErrorMessage } from '../shared/adminErrorMessage';
import './recharges.css';

export const rechargeStatuses = { pending: '待审核', credited: '已入账', rejected: '已拒绝' };
export const rechargeModes = { review: '审核入账', direct: '免审直充' };
export const rechargeColumns: Array<ColumnProps<RechargeRecord>> = [
  { key: 'id', dataIndex: 'id', title: '申请ID', width: 110 },
  { key: 'status', dataIndex: 'status', title: '状态', width: 110, render: (_, row) => rechargeStatuses[row.status] },
  { key: 'mode', dataIndex: 'mode', title: '提交路径', width: 120, render: (_, row) => rechargeModes[row.mode] },
  { key: 'user_id', dataIndex: 'user_id', title: '用户ID', width: 110 },
  { key: 'asset_symbol', dataIndex: 'asset_symbol', title: '资产', width: 100 },
  { key: 'amount', dataIndex: 'amount', title: '充值金额', width: 260 },
  { key: 'reason', dataIndex: 'reason', title: '申请原因', width: 220 },
  { key: 'created_at', dataIndex: 'created_at', title: '申请时间', width: 190, render: (_, row) => <TimestampText value={row.created_at} /> },
  { key: 'review_reason', dataIndex: 'review_reason', title: '审核原因', width: 220, render: (_, row) => row.review_reason ?? '-' },
  { key: 'reviewed_by', dataIndex: 'reviewed_by', title: '审核人ID', width: 120, render: (_, row) => row.reviewed_by ?? '-' },
  { key: 'reviewed_at', dataIndex: 'reviewed_at', title: '审核时间', width: 190, render: (_, row) => <TimestampText value={row.reviewed_at} /> },
  { key: 'credited_at', dataIndex: 'credited_at', title: '入账时间', width: 190, render: (_, row) => <TimestampText value={row.credited_at} /> }
];

export function RechargeHistory({ scope, userId, revision = 0, action }: {
  scope: 'agent' | 'admin'; userId?: number; revision?: number; action?: (row: RechargeRecord) => ReactNode;
}) {
  const [status, setStatus] = useState<RechargeStatus | ''>(scope === 'admin' ? 'pending' : '');
  const [draft, setDraft] = useState({ user: '', agent: '' });
  const [filters, setFilters] = useState<{ user_id?: number; agent_id?: number }>({});
  const [page, setPage] = useState(1);
  const [size, setSize] = useState(20);
  const [reload, setReload] = useState(0);
  const [result, setResult] = useState<RechargePage>({ recharges: [], total: 0 });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [filterError, setFilterError] = useState('');
  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError(null);
    void listRecharges(scope, { ...filters, user_id: userId ?? filters.user_id, status: status || undefined, limit: size, offset: (page - 1) * size }, controller.signal)
      .then((value) => { if (!controller.signal.aborted) setResult(value); })
      .catch((cause: unknown) => { if (!controller.signal.aborted) setError(new Error(adminErrorMessage(cause, '充值记录加载失败'))); })
      .finally(() => { if (!controller.signal.aborted) setLoading(false); });
    return () => controller.abort();
  }, [scope, userId, filters, status, page, size, reload, revision]);
  function search() {
    try {
      setFilters({
        user_id: draft.user.trim() ? requiredSafeInteger(draft.user, '用户ID', 1) : undefined,
        agent_id: scope === 'admin' && draft.agent.trim() ? requiredSafeInteger(draft.agent, '代理ID', 1) : undefined
      });
      setPage(1); setFilterError('');
    } catch (cause) { setFilterError(adminErrorMessage(cause)); }
  }
  const columns = [...rechargeColumns];
  if (scope === 'admin') columns.splice(3, 0,
    { key: 'agent_id', dataIndex: 'agent_id', title: '代理ID', width: 110 },
    { key: 'agent_admin_id', dataIndex: 'agent_admin_id', title: '申请账号ID', width: 120 });
  if (action) columns.push({ key: 'actions', title: '操作', fixed: 'right', width: 150, render: (_, row) => action(row) });
  return <section className="recharge-history">
    <div className="recharge-filters">
      <label id="recharge-status-label">状态
        <Select aria-labelledby="recharge-status-label" value={status} onChange={(value) => { setStatus(value as RechargeStatus | ''); setPage(1); }}
          onSelect={(value) => { setStatus(value as RechargeStatus | ''); setPage(1); }}
          optionList={[{ label: '全部状态', value: '' }, ...Object.entries(rechargeStatuses).map(([value, label]) => ({ value, label }))]} />
      </label>
      {userId === undefined ? <label>用户ID<AdminTextInput ariaLabel="筛选用户ID" value={draft.user} onChange={(user) => setDraft({ ...draft, user })} /></label> : null}
      {scope === 'admin' ? <label>代理ID<AdminTextInput ariaLabel="筛选代理ID" value={draft.agent} onChange={(agent) => setDraft({ ...draft, agent })} /></label> : null}
      <Space>
        <Button icon={<IconSearch />} onClick={search}>查询</Button>
        <Button icon={<IconRefresh />} loading={loading} onClick={() => setReload((value) => value + 1)}>刷新</Button>
      </Space>
    </div>
    {filterError ? <p role="alert">{filterError}</p> : null}
    <DataTable columns={columns} data={result.recharges} loading={loading} error={error} rowKey="id" pagination={{
      currentPage: page, pageSize: size, total: result.total,
      onPageChange: setPage, onPageSizeChange: (value) => { setSize(value); setPage(1); }
    }} />
  </section>;
}
