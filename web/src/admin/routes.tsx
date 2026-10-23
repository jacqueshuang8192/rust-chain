import type { ComponentType } from 'react';
import { Navigate, type RouteObject } from 'react-router-dom';

import { AdminPermissionBoundary, adminReadPermissionForPath } from './access';

type ResourceConfigsModule = typeof import('./resources/resourceConfigs');
type ResourceConfigKey = keyof ResourceConfigsModule['resourceConfigs'];

// 后台页面全部走路由级按需加载：登录页与代理端不再打包管理端代码，
// 通用资源页共享同一份 resourceConfigs chunk，独立配置页各自成块。
// handle.resourceKey 让路由与资源配置的绑定关系保持静态可读，测试无需触发动态导入即可校验。
function resourceRoute(path: string, resourceKey: ResourceConfigKey): RouteObject {
  const permission = adminReadPermissionForPath(`/admin/${path}`);
  return {
    path,
    handle: { resourceKey, permission },
    lazy: async () => {
      const { ResourcePage, resourceConfigs } = await import('./resources/resourceConfigs');
      const Page = path.startsWith('new-coins/')
        ? (await import('./new-coins/NewCoinResourcePage')).NewCoinResourcePage
        : resourceKey === 'secondsOrders' || resourceKey === 'marginPositions'
          ? (await import('./resources/TradingResourcePage')).TradingResourcePage
          : ResourcePage;
      const Recharge = resourceKey === 'walletDeposits' || resourceKey === 'quickRechargeOrders'
        ? (await import('./deposits/RechargeWorkspace')).RechargeWorkspace : null;
      return {
        Component: function AdminResourceRoute() {
          return (
            <AdminPermissionBoundary permission={permission}>
              {Recharge ? <Recharge kind={resourceKey === 'quickRechargeOrders' ? 'quick' : 'chain'}><Page config={resourceConfigs[resourceKey]} /></Recharge> : <Page config={resourceConfigs[resourceKey]} />}
            </AdminPermissionBoundary>
          );
        }
      };
    }
  };
}

/** 独立后台页面同样在路由入口校验读取权限，避免仅靠侧栏隐藏形成前端越权入口。 */
function guardedLazyRoute(path: string, load: () => Promise<ComponentType>): RouteObject {
  const permission = adminReadPermissionForPath(`/admin/${path}`);
  return {
    path,
    handle: { permission },
    lazy: async () => {
      const Component = await load();
      const GuardedAdminPage = function GuardedAdminPage() {
        return (
          <AdminPermissionBoundary permission={permission}>
            <Component />
          </AdminPermissionBoundary>
        );
      };
      // 保留被包装页面的函数名，方便路由注册测试和开发工具识别真实业务页面。
      Object.defineProperty(GuardedAdminPage, 'name', { value: Component.name });
      return {
        Component: GuardedAdminPage
      };
    }
  };
}

function rechargeRoute(path: string, kind: 'chain' | 'quick', load: () => Promise<ComponentType>): RouteObject {
  return guardedLazyRoute(path, async () => {
    const Page = await load();
    const { RechargeWorkspace } = await import('./deposits/RechargeWorkspace');
    return function RechargePage() { return <RechargeWorkspace kind={kind}><Page /></RechargeWorkspace>; };
  });
}

export const adminRoutes: RouteObject[] = [
  { index: true, element: <Navigate to="dashboard" replace /> },
  guardedLazyRoute('dashboard', async () => (await import('./dashboard/DashboardPage')).DashboardPage),
  guardedLazyRoute('system/legal-documents', async () => (await import('./actions/RegistrationLegalPage')).RegistrationLegalPage),
  guardedLazyRoute(
    'config-center',
    async () => (await import('./config-center/ConfigCenterPage')).ConfigCenterPage
  ),
  guardedLazyRoute(
    'governance/financial-idempotency',
    async () =>
      (await import('./governance/FinancialIdempotencyPage')).FinancialIdempotencyPage
  ),
  guardedLazyRoute(
    'governance/financial-retries',
    async () => (await import('./governance/FinancialRetriesPage')).FinancialRetriesPage
  ),
  guardedLazyRoute(
    'financial-reconciliation',
    async () => (await import('./governance/FinancialReconciliationPage')).FinancialReconciliationPage
  ),
  guardedLazyRoute(
    'seconds-contract/orders/:orderId',
    async () => (await import('./governance/FinancialRetriesPage')).FinancialRetrySecondsOrderPage
  ),
  guardedLazyRoute(
    'support',
    async () => (await import('./support/AdminSupportPage')).AdminSupportPage
  ),
  resourceRoute('users', 'users'),
  guardedLazyRoute(
    'users/kyc/settings',
    async () => (await import('./actions/KycManagementPage')).KycSettingsPage
  ),
  guardedLazyRoute(
    'users/kyc/reviews',
    async () => (await import('./actions/KycManagementPage')).KycReviewsPage
  ),
  { path: 'users/kyc', element: <Navigate to="/admin/users/kyc/reviews" replace /> },
  guardedLazyRoute('agents', async () => (await import('./actions/AgentManagementPage')).AgentManagementPage),
  guardedLazyRoute('agent-coin-applications', async () => (await import('../coin-applications/CoinApplicationsPage')).AdminCoinApplicationsPage),
  guardedLazyRoute('agent-recharges', async () => (await import('./recharges/RechargeReviewPage')).RechargeReviewPage),
  resourceRoute('agent-commissions', 'agentCommissions'),
  resourceRoute('agent-commission-rules', 'agentCommissionRules'),
  resourceRoute('news', 'news'),
  guardedLazyRoute('home/carousel', async () => (await import('./actions/HomeCarouselPage')).HomeCarouselPage),
  resourceRoute('assets', 'assets'),
  resourceRoute('wallet/accounts', 'walletAccounts'),
  resourceRoute('wallet/withdrawal-networks', 'depositNetworkConfigs'),
  rechargeRoute('wallet/quick-recharge', 'quick', async () => (await import('./actions/QuickRechargeConfigPage')).QuickRechargeConfigPage),
  resourceRoute('wallet/quick-recharge/orders', 'quickRechargeOrders'),
  rechargeRoute('wallet/chain-recharge', 'chain', async () => (await import('./deposits/DepositSettingsPage')).DepositSettingsPage),
  rechargeRoute('wallet/chain-recharge/addresses', 'chain', async () => (await import('./deposits/DepositSettingsPage')).DepositSettingsPage),
  rechargeRoute('wallet/chain-recharge/records', 'chain', async () => (await import('./deposits/DepositReviewPage')).DepositReviewPage),
  resourceRoute('wallet/chain-recharge/ledger', 'walletDeposits'),
  ...[
    ['deposit-network-configs', 'withdrawal-networks'],
    ['deposit-address-pool', 'chain-recharge/addresses'],
    ['deposit-settings', 'chain-recharge'],
    ['deposit-receipts', 'chain-recharge/records'],
    ['deposits', 'chain-recharge/ledger'],
    ['quick-recharge-orders', 'quick-recharge/orders']
  ].map(([old, current]) => ({ path: `wallet/${old}`, element: <Navigate to={`/admin/wallet/${current}`} replace /> })),
  resourceRoute('wallet/withdrawals', 'walletWithdrawals'),
  guardedLazyRoute('wallet/withdrawal-policies', async () => (await import('./actions/WithdrawalPolicyPage')).WithdrawalPolicyPage),
  resourceRoute('wallet/ledger', 'walletLedger'),
  resourceRoute('loan/products', 'loanProducts'),
  resourceRoute('loan/orders', 'loanOrders'),
  guardedLazyRoute(
    'prediction/settings',
    async () => (await import('./actions/PredictionConfigPage')).PredictionSettingsPage
  ),
  {
    path: 'prediction/assets',
    element: <Navigate to="/admin/prediction/settings?tab=assets" replace />
  },
  resourceRoute('prediction/markets', 'predictionMarkets'),
  resourceRoute('prediction/orders', 'predictionOrders'),
  guardedLazyRoute(
    'prediction/sync',
    async () => (await import('./actions/PredictionConfigPage')).PredictionSyncPage
  ),
  { path: 'prediction/sync-logs', element: <Navigate to="/admin/prediction/sync" replace /> },
  resourceRoute('spot/orders', 'spotOrders'),
  resourceRoute('spot/trades', 'spotTrades'),
  resourceRoute('new-coins/projects', 'newCoinProjects'),
  guardedLazyRoute('new-coins/projects/:projectId', async () => (await import('./new-coins/NewCoinProjectPage')).NewCoinProjectPage),
  guardedLazyRoute(
    'new-coins/reconciliation/:projectId',
    async () => (await import('./new-coins/NewCoinReconciliationPage')).NewCoinReconciliationPage
  ),
  guardedLazyRoute('new-coins/actions', async () => (await import('./actions/NewCoinActions')).NewCoinActions),
  resourceRoute('new-coins/subscriptions', 'newCoinSubscriptions'),
  resourceRoute('new-coins/distributions', 'newCoinDistributions'),
  resourceRoute('new-coins/purchases', 'newCoinPurchases'),
  guardedLazyRoute('new-coins/holdings', async () => (await import('./new-coins/TokenRecordsPage')).TokenHoldingsPage),
  guardedLazyRoute('new-coins/buybacks', async () => (await import('./new-coins/TokenRecordsPage')).TokenBuybacksPage),
  guardedLazyRoute('new-coins/operations', async () => (await import('./new-coins/TokenRecordsPage')).TokenOperationsPage),
  guardedLazyRoute('new-coins/lock-positions', async () => (await import('./new-coins/NewCoinResourcePage')).NewCoinLocksPage),
  guardedLazyRoute('new-coins/unlocks', async () => (await import('./new-coins/NewCoinResourcePage')).NewCoinLocksPage),
  resourceRoute('market/pairs', 'marketPairs'),
  resourceRoute('market/strategies', 'marketStrategies'),
  {
    path: 'market/strategies/actions',
    element: <Navigate to="/admin/market/strategies" replace />
  },
  guardedLazyRoute(
    'market/feed-config',
    async () => (await import('./actions/MarketFeedConfigPage')).MarketFeedConfigPage
  ),
  resourceRoute('convert/pairs', 'convertPairs'),
  resourceRoute('convert/orders', 'convertOrders'),
  resourceRoute('seconds-contract/products', 'secondsProducts'),
  resourceRoute('seconds-contract/orders', 'secondsOrders'),
  resourceRoute('margin/products', 'marginProducts'),
  resourceRoute('margin/positions', 'marginPositions'),
  resourceRoute('margin/liquidations', 'marginLiquidations'),
  resourceRoute('margin/interest', 'marginInterest'),
  resourceRoute('earn/categories', 'earnCategories'),
  resourceRoute('earn/products', 'earnProducts'),
  resourceRoute('earn/subscriptions', 'earnSubscriptions'),
  resourceRoute('risk', 'riskRules'),
  resourceRoute('risk/events', 'riskEvents'),
  resourceRoute('system/countries', 'countries'),
  guardedLazyRoute(
    'system/security-policy',
    async () => (await import('./actions/SecurityPolicyPage')).SecurityPolicyPage
  ),
  guardedLazyRoute(
    'account/security',
    async () => (await import('./actions/AdminTwoFactorPage')).AdminTwoFactorPage
  ),
  { path: 'system/two-factor', element: <Navigate to="/admin/account/security" replace /> },
  guardedLazyRoute('system/brand', async () => (await import('./actions/PlatformBrandPage')).PlatformBrandPage),
  guardedLazyRoute('system/smtp', async () => (await import('./actions/SmtpConfigPage')).SmtpConfigPage),
  guardedLazyRoute('system/uploads', async () => (await import('./actions/UploadConfigPage')).UploadConfigPage),
  guardedLazyRoute('audit-logs', async () => (await import('./audit/AuditLogsPage')).AuditLogsPage)
];
