import { IconFilter, IconRefresh } from '@douyinfe/semi-icons';
import { Banner, Button } from '@douyinfe/semi-ui';
import type { ColumnProps } from '@douyinfe/semi-ui/lib/es/table';
import { useEffect, useMemo, useState, useSyncExternalStore } from 'react';
import { Link, useParams, useSearchParams } from 'react-router-dom';
import { fundingKinds, fundingLabels, fundingSources, fundingStatuses, getAgentFunding, type FundingKind, type FundingRecord } from '../api/agentFunding';
import { authStore } from '../auth/authStore';
import { PageHeader } from '../layouts/PageHeader';
import { AmountText } from '../shared/AmountText';
import { DataTable } from '../shared/DataTable';
import { DetailDrawer } from '../shared/DetailDrawer';
import { FilterBar, type FilterValues } from '../shared/FilterBar';
import { StatusTag } from '../shared/StatusTag';
import { TimestampText } from '../shared/TimestampText';
import { fundingFilters, fundingQuery } from './fundingModel';
import './TeamOrdersPage.css';

const primaryFilters = new Set(['team_scope', 'user_id', 'email', 'status', 'source']);
const networks = { eth: 'Ethereum', base: 'Base', tron: 'Tron', btc: 'Bitcoin', solana: 'Solana' };
const recordLabel = (row: FundingRecord) => row.order_no || `记录 ${row.id}`;
function amountColumn(key: 'amount' | 'paid_amount' | 'credited_amount' | 'fee' | 'total_reserved', title: string): ColumnProps<FundingRecord> {
  return { key, title, width: 200, render: (_, row) => <AmountText value={row[key]} asset={row.asset_symbol} appendAsset /> };
}
function columnsFor(kind: FundingKind, select: (row: FundingRecord) => void): ColumnProps<FundingRecord>[] {
  return [
    { key: 'order_no', title: kind === 'deposits' ? '充值订单号' : '提现记录', width: 220, render: (_, row) => <Button theme="borderless" onClick={() => select(row)}>{recordLabel(row)}</Button> },
    { key: 'user', title: '用户', width: 240, render: (_, row) => <div className="agent-order-identity"><Link to={`/agent/users/${row.user_id}/portfolio`} state={{ email: row.email }}>{row.email || `用户 ${row.user_id}`}</Link><small>用户ID：{row.user_id}</small></div> },
    { key: 'owner', title: '归属代理', width: 180, render: (_, row) => <div className="agent-order-identity"><span>{row.owner_agent_code}</span><small>代理ID：{row.owner_agent_id}</small></div> },
    ...(kind === 'deposits' ? [{ key: 'source', title: '充值方式', width: 130, render: (_: unknown, row: FundingRecord) => fundingSources[row.source] }] : []),
    { key: 'status', title: '状态', width: 160, render: (_, row) => <StatusTag value={row.status} label={fundingStatuses[kind][row.status]} /> },
    { key: 'network', title: '网络', width: 125, render: (_, row) => row.network ? networks[row.network as keyof typeof networks] ?? row.network : '—' },
    amountColumn('amount', kind === 'deposits' ? '应付币种数量' : '提现数量'),
    ...(kind === 'deposits' ? [amountColumn('credited_amount', '已入账净数量')] : [amountColumn('fee', '手续费'), amountColumn('total_reserved', '总冻结数量')]),
    { key: 'created_at', title: '创建时间', width: 186, render: (_, row) => <TimestampText value={row.created_at} /> },
    { key: 'completed_at', title: kind === 'deposits' ? '入账时间' : '确认 / 释放时间', width: 186, render: (_, row) => <TimestampText value={row.completed_at} /> }
  ];
}

export function AgentFundingRecordsPage() {
  const { kind = '' } = useParams();
  const [params] = useSearchParams();
  const initialUser = params.get('user_id') ?? '';
  const generation = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('agent')?.generation ?? '');
  if (!fundingKinds.includes(kind as FundingKind)) return <main className="exchange-page"><PageHeader title="资金记录类型不存在" /></main>;
  return <FundingRecords key={`${generation}:${kind}:${initialUser}`} kind={kind as FundingKind} initialUser={initialUser} />;
}

function FundingRecords({ kind, initialUser }: { kind: FundingKind; initialUser: string }) {
  const [filters, setFilters] = useState<FilterValues>(initialUser ? { user_id: initialUser } : {});
  const [expanded, setExpanded] = useState(false);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [revision, setRevision] = useState(0);
  const [filterError, setFilterError] = useState<string | null>(null);
  const [selected, setSelected] = useState<FundingRecord | null>(null);
  const [state, setState] = useState<{ key: string; items: FundingRecord[]; total: number; loading: boolean; error: Error | null }>({ key: '', items: [], total: 0, loading: true, error: null });
  const key = JSON.stringify([filters, page, pageSize, revision]);
  const current = key === state.key ? state : { items: [], total: 0, loading: true, error: null };
  const columns = useMemo(() => columnsFor(kind, setSelected), [kind]);
  const advancedCount = Object.keys(filters).filter(field => !primaryFilters.has(field)).length;
  useEffect(() => {
    let active = true;
    setState({ key, items: [], total: 0, loading: true, error: null });
    const load = async () => {
      try {
        const result = await getAgentFunding(kind, { ...fundingQuery(kind, filters), limit: pageSize, offset: (page - 1) * pageSize });
        if (active) setState({ key, ...result, loading: false, error: null });
      } catch (error) { if (active) setState({ key, items: [], total: 0, loading: false, error: error instanceof Error ? error : new Error('加载失败') }); }
    };
    void load(); return () => { active = false; };
  }, [filters, key, kind, page, pageSize]);
  function apply(next: FilterValues) {
    try { fundingQuery(kind, next); setFilterError(null); setSelected(null); setPage(1); setFilters(next); setRevision(value => value + 1); }
    catch (error) { setFilterError(error instanceof Error ? error.message : '筛选条件无效'); }
  }
  return <main className="exchange-page agent-team-orders">
    <PageHeader title={fundingLabels[kind]} description={kind === 'deposits' ? '查看本级及下级代理归属用户的快速充值和链上充值订单。金额以币种数量展示，法币报价在详情中单独列出。' : '查看本级及下级代理归属用户的提现申请及处理状态。此页面仅提供查询。'} actions={<>
      <Button icon={<IconFilter aria-hidden="true" />} aria-expanded={expanded} onClick={() => setExpanded(value => !value)}>{expanded ? '收起筛选' : '更多筛选'}{advancedCount ? `（${advancedCount}）` : ''}</Button>
      <Button aria-label="刷新记录" icon={<IconRefresh />} disabled={current.loading} onClick={() => { setSelected(null); setRevision(value => value + 1); }} />
    </>} />
    <FilterBar fields={fundingFilters(kind).filter(field => expanded || primaryFilters.has(field.key))} value={filters} onChange={apply} loading={current.loading} />
    {filterError ? <Banner type="danger" description={filterError} /> : null}
    <DataTable columns={columns} data={current.items.map(row => ({ ...row, record_key: `${row.source}:${row.id}` }))} rowKey="record_key" loading={current.loading} error={current.error}
      pagination={{ currentPage: page, pageSize, total: current.total, onPageChange: next => {
        if ((next - 1) * pageSize > 100_000) { setFilterError('查询范围过大，请缩小时间或用户范围'); return; }
        setSelected(null); setPage(next);
      }, onPageSizeChange: size => { setSelected(null); setPage(1); setPageSize(size); } }} />
    <DetailDrawer onClose={() => setSelected(null)} detail={selected ? {
      title: `${fundingLabels[kind]}详情`,
      data: Object.fromEntries(Object.entries(selected).filter(([field]) => field !== 'record_key').filter(([field]) => kind === 'deposits' ? field !== 'total_reserved' && (selected.source === 'quick' || !['fiat_amount', 'fiat_currency'].includes(field)) : !['order_no', 'paid_amount', 'credited_amount', 'fiat_amount', 'fiat_currency', 'expires_at'].includes(field))),
      fieldMeta: {
        labels: { id: '记录ID', order_no: '充值订单号', source: '记录来源', email: '用户邮箱', owner_agent_code: '归属代理编号', owner_agent_id: '归属代理ID',
          amount: kind === 'deposits' ? '应付币种数量' : '提现数量', paid_amount: '实际收到币种数量', credited_amount: '已入账净数量（当前状态）',
          fee: '手续费', total_reserved: '总冻结数量', fiat_amount: '法币报价金额', fiat_currency: '法币报价币种',
          completed_at: kind === 'deposits' ? '入账时间' : '确认 / 释放时间', expires_at: '付款截止时间' },
        types: { amount: 'amount', paid_amount: 'amount', credited_amount: 'amount', fee: 'amount', total_reserved: 'amount', fiat_amount: 'amount', created_at: 'timestamp', completed_at: 'timestamp', expires_at: 'timestamp' },
        assets: { amount: selected.asset_symbol, paid_amount: selected.asset_symbol, credited_amount: selected.asset_symbol, fee: selected.asset_symbol, total_reserved: selected.asset_symbol, fiat_amount: selected.fiat_currency ?? undefined },
        valueMaps: { status: fundingStatuses[kind], source: fundingSources, network: networks }
      }
    } : null} />
  </main>;
}
