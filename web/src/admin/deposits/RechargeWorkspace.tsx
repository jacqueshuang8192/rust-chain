import { Tabs, TabPane } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useCanAdminRequest } from '../access';
import './RechargeWorkspace.css';

/** 充值只有快速与链上两个工作区；各标签独立鉴权，只挂载当前页面。 */
export function RechargeWorkspace({ kind, children }: { kind: 'chain' | 'quick'; children: ReactNode }) {
  const { pathname } = useLocation();
  const navigate = useNavigate();
  const canNetwork = useCanAdminRequest('/admin/api/v1/wallet/deposit-settings', 'GET');
  const canAddress = useCanAdminRequest('/admin/api/v1/wallet/deposit-settings/addresses', 'GET');
  const canRecords = useCanAdminRequest('/admin/api/v1/wallet/deposit-orders', 'GET');
  const canQuick = useCanAdminRequest('/admin/api/v1/quick-recharge/config', 'GET');
  const tabs = kind === 'quick' ? [
    { path: '/admin/wallet/quick-recharge', label: '快速充值配置', allowed: canQuick },
    { path: '/admin/wallet/quick-recharge/orders', label: '快速充值订单', allowed: canQuick }
  ] : [
    { path: canNetwork ? '/admin/wallet/chain-recharge' : '/admin/wallet/chain-recharge/addresses', label: canNetwork ? '网络与收款配置' : '收款地址', allowed: canNetwork || canAddress },
    { path: '/admin/wallet/chain-recharge/records', label: '订单与到账核对', allowed: canRecords },
    { path: '/admin/wallet/chain-recharge/ledger', label: '入账流水', allowed: canRecords }
  ];
  const active = pathname === '/admin/wallet/chain-recharge/addresses' && canNetwork ? '/admin/wallet/chain-recharge' : pathname;
  return <Tabs className="recharge-workspace" activeKey={active} keepDOM={false} tabPaneMotion={false} onChange={(path) => navigate(path)}>
    {tabs.filter((tab) => tab.allowed).map((tab) => <TabPane key={tab.path} itemKey={tab.path} tab={tab.label}>{tab.path === active ? children : null}</TabPane>)}
  </Tabs>;
}
