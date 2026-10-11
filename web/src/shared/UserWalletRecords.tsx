import { IconRefresh } from '@douyinfe/semi-icons';
import { Button, Space, Tabs, Tooltip } from '@douyinfe/semi-ui';
import type { ColumnProps } from '@douyinfe/semi-ui/lib/es/table';
import { useEffect, useState, useSyncExternalStore } from 'react';
import { authStore } from '../auth/authStore';
import { getUserWalletRecords, type UserWalletRecord, type UserWalletRecordPage, type WalletRecordsScope } from '../api/userWalletRecords';
import { ADMIN_FIELD_VALUE_LABELS } from './adminEnumLabels';
import { adminEnumLabel } from './adminPresentation';
import { AmountText } from './AmountText';
import { DataTable } from './DataTable';
import { FilterBar, type FilterField, type FilterValues } from './FilterBar';
import { TimestampText } from './TimestampText';

const options = (key: string) => Object.entries(ADMIN_FIELD_VALUE_LABELS[key] || {}).map(([value, label]) => ({ value, label }));
const filters: FilterField[] = [
  { key: 'account_type', label: '资金账户', type: 'select', options: options('account_type') },
  { key: 'asset_id', label: '资产ID' },
  { key: 'change_type', label: '变动类型', type: 'select', options: options('change_type') },
  { key: 'balance_type', label: '余额类型', type: 'select', options: [{ value: 'available', label: '可用余额' }, { value: 'frozen', label: '冻结余额' }, { value: 'locked', label: '锁定余额' }] },
  { key: 'ref_type', label: '来源类型', type: 'select', options: options('ref_type') },
  { key: 'ref_id', label: '来源编号' }
];
const label = (field: string, value: string) => adminEnumLabel(field, value) || value;
const amountColumn = (key: keyof UserWalletRecord, title: string): ColumnProps<UserWalletRecord> => ({
  key, dataIndex: key, title, width: 180, render: (_value, row) => <AmountText value={String(row[key])} />
});
const baseColumns: Array<ColumnProps<UserWalletRecord>> = [
  { key: 'id', dataIndex: 'id', title: '流水ID', width: 110 },
  { key: 'created_at', dataIndex: 'created_at', title: '发生时间', width: 184, render: (_value, row) => <TimestampText value={row.created_at} /> },
  { key: 'asset_symbol', dataIndex: 'asset_symbol', title: '资产', width: 96 },
  { key: 'change_type', dataIndex: 'change_type', title: '变动类型', width: 160, render: (_value, row) => label('change_type', row.change_type) },
  amountColumn('amount', '变动金额'),
  { key: 'balance_type', dataIndex: 'balance_type', title: '余额类型', width: 120, render: (_value, row) => ({ available: '可用余额', frozen: '冻结余额', locked: '锁定余额' })[row.balance_type] || row.balance_type },
  amountColumn('balance_after', '变动后余额')
];
const snapshotColumns = [...baseColumns, amountColumn('available_after', '变动后可用'), amountColumn('frozen_after', '变动后冻结'), amountColumn('locked_after', '变动后锁定')];
const flowColumns: Array<ColumnProps<UserWalletRecord>> = [...baseColumns,
  { key: 'ref_type', dataIndex: 'ref_type', title: '来源类型', width: 140, render: (_value, row) => label('ref_type', row.ref_type) },
  { key: 'ref_id', dataIndex: 'ref_id', title: '来源编号', width: 240 }
];

export function UserWalletRecords({ scope, userId }: { scope: WalletRecordsScope; userId: number }) {
  const generation = useSyncExternalStore(authStore.subscribe, () => authStore.getSession(scope)?.generation ?? '');
  return <WalletRecords key={`${scope}:${userId}:${generation}`} scope={scope} userId={userId} />;
}

function WalletRecords({ scope, userId }: { scope: WalletRecordsScope; userId: number }) {
  const [tab, setTab] = useState('changes');
  const [query, setQuery] = useState<FilterValues>({ account_type: 'spot' });
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [revision, setRevision] = useState(0);
  const key = JSON.stringify([scope, userId, query, page, pageSize, revision]);
  const [result, setResult] = useState<{ key: string; data: UserWalletRecordPage | null; loading: boolean; error: Error | null }>({ key: '', data: null, loading: true, error: null });
  useEffect(() => {
    const controller = new AbortController();
    let active = true;
    setResult({ key, data: null, loading: true, error: null });
    getUserWalletRecords(scope, userId, query, pageSize, (page - 1) * pageSize, controller.signal)
      .then((data) => { if (active) setResult({ key, data, loading: false, error: null }); })
      .catch((error: unknown) => { if (active) setResult({ key, data: null, loading: false, error: error instanceof Error ? error : new Error('资金记录加载失败') }); });
    return () => { active = false; controller.abort(); };
  }, [key, scope, userId, query, page, pageSize]);
  const current = result.key === key ? result : { data: null, error: null, loading: true };
  return <section aria-label="用户资金记录">
    <Space wrap style={{ marginBottom: 12, justifyContent: 'space-between', width: '100%' }}>
      <Tabs activeKey={tab} onChange={setTab} tabList={[{ itemKey: 'changes', tab: '钱包变化记录' }, { itemKey: 'flows', tab: '资金流水记录' }]} type="button" />
      <Tooltip content="刷新资金记录"><Button aria-label="刷新资金记录" disabled={current.loading} icon={<IconRefresh />} onClick={() => setRevision((value) => value + 1)} /></Tooltip>
    </Space>
    <FilterBar fields={filters} loading={current.loading} value={query} onChange={(value) => { setPage(1); setQuery({ ...value, account_type: value.account_type || 'spot' }); }} />
    <DataTable columns={tab === 'changes' ? snapshotColumns : flowColumns} data={current.data?.ledger || []} error={current.error} loading={current.loading}
      rowKey={(row) => `${row.account_type}:${row.id}`}
      pagination={{ currentPage: page, pageSize, total: current.data?.total || 0, onPageChange: setPage, onPageSizeChange: (value) => { setPage(1); setPageSize(value); } }} />
  </section>;
}
