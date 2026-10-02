import { Tabs } from '@douyinfe/semi-ui';
import { useMemo, useState } from 'react';

import { AmountText } from '../../shared/AmountText';
import { SecondsOrderCountdown } from '../../shared/SecondsOrderCountdown';
import { StatusTag } from '../../shared/StatusTag';
import { ResourcePage, type ResourceConfig } from './resourceConfigs';
import './TradingResourcePage.css';

export function TradingResourcePage({ config }: { config: ResourceConfig }) {
  const [tab, setTab] = useState('active');
  const seconds = config.endpoint === '/admin/api/v1/seconds-contracts/orders';
  const scopedConfig = useMemo<ResourceConfig>(() => {
    const active = tab === 'active';
    const fixedFilters: ResourceConfig['fixedFilters'] = active ? { status: 'opened' } : undefined;
    if (fixedFilters && !seconds) fixedFilters.filled_only = 'true';
    const statusLabels: Record<string, string> = seconds
      ? { opened: '进行中', settled: '已结算', manual_review: '待人工复核', refunded: '已退款' }
      : { opened: '持仓中', closed: '已平仓', liquidated: '已强平', canceled: '已取消' };
    const columns: ResourceConfig['columns'] = config.columns.map((column) => column.key === 'status'
      ? { ...column, valueMap: statusLabels, render: (record) => <StatusTag value={String(record.status)} label={!seconds && record.status === 'opened' && record.entry_price == null ? '待成交' : statusLabels[String(record.status)]} /> }
      : column);
    if (seconds) {
      columns.splice(1, 0, { key: 'user_id', title: '用户ID' });
      columns.splice(5, 0,
        { key: 'stake_asset_symbol', title: '押注资产' },
        { key: 'duration_seconds', title: '周期（秒）' },
        {
          key: 'countdown', source: 'derived', title: '倒计时',
          render: (record) => record.status === 'opened' && typeof record.expires_at === 'number' && Number.isSafeInteger(record.expires_at) && record.expires_at > 0 && typeof record.id === 'number' && Number.isSafeInteger(record.id)
            ? <SecondsOrderCountdown key={`${record.id}:${record.expires_at}`} expiresAt={record.expires_at} orderId={record.id} />
            : <span>-</span>
        });
      const order = ['order_no', 'symbol', 'status', 'countdown', 'stake_amount', 'direction', 'user_id', 'email', 'stake_asset_symbol', 'duration_seconds', 'entry_price', 'settlement_price', 'result', 'expires_at'];
      columns.sort((left, right) => order.indexOf(left.key) - order.indexOf(right.key));
    } else {
      columns.splice(3, 0,
        { key: 'pair_id', title: '交易对ID' },
        { key: 'margin_asset', title: '保证金资产ID' },
        { key: 'margin_mode', title: '保证金模式', valueMap: { cross: '全仓', isolated: '逐仓' } },
        { key: 'leverage', title: '杠杆倍数', type: 'amount' },
        // This optional API field may be omitted for pending/unfilled orders.
        { key: 'entry_price_display', source: 'derived', title: '开仓价', render: (record) => <AmountText value={typeof record.entry_price === 'string' ? record.entry_price : null} /> });
      const order = ['id', 'user_id', 'status', 'direction', 'margin_amount', 'leverage', 'entry_price_display', 'pair_id', 'notional_amount', 'borrowed_amount', 'interest_amount', 'margin_mode', 'product_id', 'margin_asset'];
      columns.sort((left, right) => order.indexOf(left.key) - order.indexOf(right.key));
    }
    return {
      ...config,
      title: seconds ? (active ? '进行中的秒合约订单' : '全部秒合约订单') : (active ? '当前杠杆持仓' : '全部杠杆仓位'),
      columns,
      fixedFilters,
      filters: config.filters?.filter((field) => !active || field.key !== 'status').map((field) => field.key === 'status'
        ? { ...field, type: 'select', options: Object.entries(statusLabels).map(([value, label]) => ({ value, label: !seconds && value === 'opened' ? '未结束（含待成交）' : label })) }
        : field)
    };
  }, [config, seconds, tab]);

  return <Tabs className="admin-trading-tabs" activeKey={tab} onChange={setTab} type="line" keepDOM={false} tabPaneMotion={false}>
    <Tabs.TabPane itemKey="active" tab={seconds ? '进行中订单' : '当前持仓'}>
      {tab === 'active' && <ResourcePage key={`${config.endpoint}:active`} config={scopedConfig} />}
    </Tabs.TabPane>
    <Tabs.TabPane itemKey="all" tab={seconds ? '全部订单' : '全部仓位'}>
      {tab === 'all' && <ResourcePage key={`${config.endpoint}:all`} config={scopedConfig} />}
    </Tabs.TabPane>
  </Tabs>;
}
