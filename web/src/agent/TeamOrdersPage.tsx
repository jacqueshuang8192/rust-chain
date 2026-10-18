import { IconFilter, IconRefresh } from '@douyinfe/semi-icons';
import { Banner, Button, Tooltip } from '@douyinfe/semi-ui';
import type { ColumnProps } from '@douyinfe/semi-ui/lib/es/table';
import { useEffect, useMemo, useState, useSyncExternalStore } from 'react';
import { Link, useParams } from 'react-router-dom';
import { agentOrderKinds, getAgentTeamOrders, type AgentOrderKind, type AgentTeamOrder } from '../api/agentTeamOrders';
import { authStore } from '../auth/authStore';
import { PageHeader } from '../layouts/PageHeader';
import { AmountText } from '../shared/AmountText';
import { DataTable } from '../shared/DataTable';
import { DetailDrawer } from '../shared/DetailDrawer';
import { FilterBar, type FilterValues } from '../shared/FilterBar';
import { SecondsOrderCountdown } from '../shared/SecondsOrderCountdown';
import { StatusTag } from '../shared/StatusTag';
import { TimestampText } from '../shared/TimestampText';
import { adminEnumLabel } from '../shared/adminPresentation';
import { formatBusinessOrderNo } from '../shared/orderNo';
import { multiplyDecimalText } from '../shared/decimal';
import { formatAdminNumber } from '../shared/numberFormat';
import { teamOrderFilters, teamOrderLabels, teamOrderPrefixes, teamOrderQuery, teamOrderStatuses } from './teamOrderModel';
import './TeamOrdersPage.css';

const stringValue = (value: unknown) => typeof value === 'string' ? value : null;
const primaryFilters = new Set(['team_scope', 'user_id', 'email', 'status']);
const amountColumn = (key: string, title: string, assetKey?: string): ColumnProps<AgentTeamOrder> => ({
  key, dataIndex: key, title, width: 156,
  render: (value, record) => <AmountText value={stringValue(value)} asset={assetKey ? stringValue(record[assetKey]) ?? undefined : undefined} appendAsset={Boolean(assetKey)} />
});
const timeColumn = (key: string, title: string): ColumnProps<AgentTeamOrder> => ({
  key, dataIndex: key, title, width: 186, render: value => <TimestampText value={value} />
});

function columnsFor(kind: AgentOrderKind, detail: (row: AgentTeamOrder) => void): Array<ColumnProps<AgentTeamOrder>> {
  const columns: Array<ColumnProps<AgentTeamOrder>> = [
    { key: 'order_no', title: '订单号', width: 200, render: (_, row) => <Button theme="borderless" onClick={() => detail(row)}>{formatBusinessOrderNo(teamOrderPrefixes[kind], row)}</Button> },
    { key: 'user', title: '用户', width: 260, render: (_, row) => <div className="agent-order-identity">
      <Link to={`/agent/users/${row.user_id}/portfolio`} state={{ email: row.email }}>{row.email || `用户 ${row.user_id}`}</Link><small>用户ID：{row.user_id}</small>
    </div> },
    { key: 'owner', title: '归属代理', width: 180, render: (_, row) => <div className="agent-order-identity"><span>{row.owner_agent_code}</span><small>代理ID：{row.owner_agent_id}</small></div> },
    { key: 'status', title: '状态', width: 120, render: (_, row) => {
      const status = kind === 'margin' && row.status === 'opened' && row.entry_price === null ? 'pending' : row.status;
      return <StatusTag value={status} label={(teamOrderStatuses[kind] as Record<string, string>)[status]} />;
    } }
  ];
  if (kind === 'new-coin-subscriptions' || kind === 'new-coin-purchases') {
    columns.push({ key: 'symbol', title: '新币项目', width: 180, render: (_, row) => <div className="agent-order-identity"><span>{String(row.symbol)}</span><small>项目ID：{String(row.project_id)}</small></div> });
    if (kind === 'new-coin-subscriptions') {
      columns.push(amountColumn('issue_price', '申购发行价', 'quote_symbol'),
        amountColumn('quote_amount', '申购金额', 'quote_symbol'),
        amountColumn('requested_quantity', '申购数量', 'symbol'), amountColumn('allocated_quantity', '获配数量', 'symbol'),
        amountColumn('frozen_quote_amount', '待结算冻结', 'quote_symbol'),
        amountColumn('settled_quote_amount', '实际扣款', 'quote_symbol'), amountColumn('refunded_quote_amount', '退款金额', 'quote_symbol'));
    } else {
      columns.push(amountColumn('price', '成交价', 'quote_symbol'), amountColumn('quantity', '认购数量', 'symbol'),
        amountColumn('quote_amount', '支付金额', 'quote_symbol'));
    }
  } else if (kind === 'earn') {
    columns.push({ key: 'product_name', dataIndex: 'product_name', title: '理财产品', width: 220 },
      amountColumn('amount', '申购本金', 'asset_symbol'),
      { key: 'apr_rate', title: '年化收益率', width: 140, render: (_, row) => {
        const percent = multiplyDecimalText(stringValue(row.apr_rate) ?? '', '100');
        return percent === null ? '-' : `${formatAdminNumber(percent)}%`;
      } },
      { key: 'term_days', title: '期限', width: 100, render: (_, row) => `${row.term_days} 天` },
      timeColumn('matures_at', '到期时间'), timeColumn('redeemed_at', '赎回时间'));
  } else {
    columns.push({ key: 'symbol', dataIndex: 'symbol', title: '交易对', width: 150 },
      { key: 'direction', title: '方向', width: 100, render: (_, row) => {
        const field = kind === 'spot' ? 'side' : 'direction';
        return <StatusTag value={stringValue(row[field])} label={adminEnumLabel(field, row[field]) ?? undefined} />;
      } });
    if (kind === 'seconds') {
      columns.push({ key: 'countdown', title: '倒计时', width: 140, render: (_, row) => row.status === 'opened'
        ? <SecondsOrderCountdown orderId={row.id} expiresAt={Number(row.expires_at)} /> : '-' },
        amountColumn('stake_amount', '本金', 'stake_asset_symbol'),
        { key: 'duration_seconds', title: '周期', width: 100, render: (_, row) => `${row.duration_seconds} 秒` },
        amountColumn('entry_price', '开仓价'), amountColumn('settlement_price', '结算价'),
        { key: 'result', title: '结算结果', width: 120, render: (_, row) => <StatusTag value={stringValue(row.result)} /> },
        timeColumn('expires_at', '到期时间'));
    } else if (kind === 'margin') {
      columns.push(amountColumn('margin_amount', '保证金', 'margin_asset_symbol'), amountColumn('leverage', '杠杆倍数'),
        amountColumn('entry_price', '开仓价'), amountColumn('limit_price', '委托价'),
        amountColumn('exit_price', '平仓价'), amountColumn('realized_pnl', '已实现盈亏', 'margin_asset_symbol'));
    } else {
      columns.push(amountColumn('price', '委托价'), amountColumn('quantity', '委托数量'), amountColumn('filled_quantity', '已成交数量'));
    }
  }
  columns.push(timeColumn('created_at', kind === 'earn' ? '申购时间' : '创建时间'));
  return columns;
}

export function AgentTeamOrdersPage() {
  const { kind = '' } = useParams();
  const generation = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('agent')?.generation ?? '');
  if (!agentOrderKinds.includes(kind as AgentOrderKind)) return <main className="exchange-page"><PageHeader title="订单类型不存在" /></main>;
  return <TeamOrders key={`${generation}:${kind}`} kind={kind as AgentOrderKind} />;
}

function TeamOrders({ kind }: { kind: AgentOrderKind }) {
  const [filters, setFilters] = useState<FilterValues>({});
  const [expandedFilters, setExpandedFilters] = useState(false);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [revision, setRevision] = useState(0);
  const [filterError, setFilterError] = useState<string | null>(null);
  const [selected, setSelected] = useState<AgentTeamOrder | null>(null);
  const [state, setState] = useState<{ key: string; orders: AgentTeamOrder[]; total: number; loading: boolean; error: Error | null }>({
    key: '', orders: [], total: 0, loading: true, error: null
  });
  const key = JSON.stringify([filters, page, pageSize, revision]);
  const current = state.key === key ? state : { orders: [], total: 0, loading: true, error: null };
  const columns = useMemo(() => columnsFor(kind, setSelected), [kind]);
  const advancedCount = Object.keys(filters).filter(field => !primaryFilters.has(field)).length;
  useEffect(() => {
    let active = true;
    setState({ key, orders: [], total: 0, loading: true, error: null });
    getAgentTeamOrders(kind, { ...teamOrderQuery(kind, filters), limit: pageSize, offset: (page - 1) * pageSize })
      .then(data => { if (active) setState({ key, ...data, loading: false, error: null }); })
      .catch((error: unknown) => { if (active) setState({ key, orders: [], total: 0, loading: false, error: error instanceof Error ? error : new Error('加载失败') }); });
    return () => { active = false; };
  }, [filters, key, kind, page, pageSize]);

  function applyFilters(next: FilterValues) {
    try {
      teamOrderQuery(kind, next);
      setFilterError(null); setSelected(null); setPage(1); setFilters(next); setRevision(value => value + 1);
    } catch (error) { setFilterError(error instanceof Error ? error.message : '筛选条件无效'); }
  }
  return <main className="exchange-page agent-team-orders">
    <PageHeader title={teamOrderLabels[kind]} actions={<>
      <Button icon={<IconFilter aria-hidden="true" />} aria-expanded={expandedFilters}
        onClick={() => setExpandedFilters(value => !value)}>
        {expandedFilters ? '收起筛选' : '更多筛选'}{advancedCount ? `（${advancedCount}）` : ''}
      </Button>
      <Tooltip content="刷新订单"><Button
      icon={<IconRefresh />} aria-label="刷新订单" disabled={current.loading}
      onClick={() => { setSelected(null); setRevision(value => value + 1); }} /></Tooltip>
    </>} />
    {kind === 'new-coin-purchases' ? <p className="muted">查看团队用户上市后购买新币的认购记录。</p> : null}
    <FilterBar fields={teamOrderFilters(kind).filter(field => expandedFilters || primaryFilters.has(field.key))}
      value={filters} onChange={applyFilters} loading={current.loading} />
    {filterError ? <Banner type="danger" description={filterError} /> : null}
    <DataTable columns={columns} data={current.orders} error={current.error} loading={current.loading} rowKey="id"
      pagination={{ currentPage: page, pageSize, total: current.total, onPageChange: next => {
        if ((next - 1) * pageSize > 100_000) { setFilterError('查询范围过大，请缩小时间或用户范围'); return; }
        setSelected(null); setPage(next);
      }, onPageSizeChange: size => { setSelected(null); setPage(1); setPageSize(size); } }} />
    <DetailDrawer detail={selected ? {
      title: `${teamOrderLabels[kind]}详情`,
      data: { order_no: formatBusinessOrderNo(teamOrderPrefixes[kind], selected), ...selected },
      fieldMeta: {
        labels: {
          id: '记录ID', owner_agent_code: '归属代理编号', owner_agent_id: '归属代理ID',
          email: '用户邮箱', apr_rate: '年化收益率（比例）',
          ...(kind.startsWith('new-coin-') ? { symbol: '新币代码', project_id: '项目ID', quote_symbol: '支付币种', quote_asset: '支付资产ID',
            quote_amount: kind === 'new-coin-subscriptions' ? '申购金额' : '支付金额', issue_price: '申购发行价',
            requested_quantity: '申购数量', allocated_quantity: '获配数量', frozen_quote_amount: '待结算冻结',
            settled_quote_amount: '实际扣款', refunded_quote_amount: '退款金额', settlement_mode: '结算模式',
            price: '成交价', quantity: '认购数量', lock_position_id: '锁仓ID' } : {}),
          ...(kind === 'earn' ? { product_name: '理财产品', amount: '申购本金', created_at: '申购时间', redeemed_at: '赎回时间' } : {})
        },
        types: { redeemed_at: 'timestamp' },
        valueMaps: { settlement_mode: { manual_distribution: '后台派发', legacy_instant: '历史即时成交' }, status: { ...teamOrderStatuses[kind],
          ...(kind === 'margin' && selected.entry_price === null ? { opened: '待成交' } : {}) } }
      }
    } : null} onClose={() => setSelected(null)} />
  </main>;
}
