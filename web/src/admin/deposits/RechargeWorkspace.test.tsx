import { render, screen, cleanup } from '@testing-library/react';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { afterEach, expect, it } from 'vitest';
import { AdminAccessProvider, type AdminAccess } from '../access';
import { RechargeWorkspace } from './RechargeWorkspace';

afterEach(cleanup);
function show(path: string, permissions: string[], kind: 'chain' | 'quick' = 'chain') {
  const access: AdminAccess = { admin_id: 1, username: 'operator', role_id: 2, role_name: 'operator', permissions, is_super_admin: false };
  const router = createMemoryRouter([{ path: '*', element: <RechargeWorkspace kind={kind}><p>Current page only</p></RechargeWorkspace> }], { initialEntries: [path] });
  render(<AdminAccessProvider access={access}><RouterProvider router={router} /></AdminAccessProvider>);
}
it('allows an address-only role to use the unified workspace without network or order tabs', () => {
  show('/admin/wallet/chain-recharge/addresses', ['wallet.address_pool.read']);
  expect(screen.getByRole('tab', { name: '收款地址' })).toBeInTheDocument();
  expect(screen.queryByRole('tab', { name: '网络与收款配置' })).not.toBeInTheDocument();
  expect(screen.queryByRole('tab', { name: '订单与到账核对' })).not.toBeInTheDocument();
  expect(screen.getByText('Current page only')).toBeInTheDocument();
});
it('gives a deposit reviewer access only to records and ledger', () => {
  show('/admin/wallet/chain-recharge/records', ['wallet.deposits.read']);
  expect(screen.getAllByRole('tab')).toHaveLength(2);
  expect(screen.getByRole('tab', { name: '订单与到账核对' })).toBeInTheDocument();
  expect(screen.getByRole('tab', { name: '入账流水' })).toBeInTheDocument();
  expect(screen.getAllByText('Current page only')).toHaveLength(1);
});
it('groups quick recharge configuration and orders together', () => {
  show('/admin/wallet/quick-recharge/orders', ['wallet.quick_recharge.read'], 'quick');
  expect(screen.getAllByRole('tab')).toHaveLength(2);
  expect(screen.getByRole('tab', { name: '快速充值订单' })).toHaveAttribute('aria-selected', 'true');
});
