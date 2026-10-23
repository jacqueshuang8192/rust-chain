import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { ApiError, apiRequest } from '../../api/client';
import { DepositSettingsPage } from './DepositSettingsPage';

vi.mock('../../api/client', async () => ({ ...await vi.importActual('../../api/client'), apiRequest: vi.fn() }));
vi.mock('../../auth/authStore', () => {
  const session = { generation: 'settings-fixture' };
  return { authStore: { subscribe: () => () => {}, getSession: () => session } };
});
vi.mock('../access', () => ({ AdminRequestActionBoundary: ({ children }: { children: React.ReactNode }) => children, useCanAdminRequest: () => true }));
if (!Range.prototype.getBoundingClientRect) Range.prototype.getBoundingClientRect = () => new DOMRect(0, 0, 0, 0);
const settings = { scanners: [{ network: 'eth', enabled: true, endpoint_configured: true, api_key_configured: true, confirmations: 12, revision: 7, last_scan_at: null, last_error: null }], assets: [], addresses: [] };
const mocked = vi.mocked(apiRequest);
beforeEach(() => { mocked.mockReset(); mocked.mockResolvedValue(settings); });
afterEach(cleanup);
function show() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const router = createMemoryRouter([{ path: '/', element: <DepositSettingsPage /> }]);
  render(<QueryClientProvider client={client}><RouterProvider router={router} /></QueryClientProvider>);
}
it('keeps node secrets blank, pins the revision and preserves a failed draft with its reason', async () => {
  const user = userEvent.setup(); show();
  const confirmations = await screen.findByRole('textbox', { name: '充值确认数' });
  expect(screen.getByLabelText('监听节点地址')).toHaveValue('');
  expect(screen.getByLabelText('节点 API 密钥')).toHaveValue('');
  await user.clear(confirmations); await user.type(confirmations, '15');
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '调整确认深度');
  mocked.mockRejectedValueOnce(new ApiError(409, 'CONFLICT', 'configuration conflict'));
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked).toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-settings/scanners/eth', { method: 'PUT', body: JSON.stringify({ enabled: true, endpoint: null, api_key: null, confirmations: 15, expected_revision: 7, reason: '调整确认深度' }) }));
  expect(await screen.findByRole('alert')).toHaveTextContent('数据状态已变化');
  expect(screen.getByRole('textbox', { name: '操作原因' })).toHaveValue('调整确认深度');
  expect(confirmations).toHaveValue('15');
});
it('requires an exact inventory preview and refuses a permanently assigned address', async () => {
  mocked.mockImplementation(async (path) => path.includes('deposit-address-pool/') ? { id: 12, network: 'eth', address: '0x1111111111111111111111111111111111111111', status: 'available', memo: null, assigned_user_id: 22 } : settings);
  const user = userEvent.setup(); show();
  await user.type(await screen.findByRole('textbox', { name: '地址库存编号' }), '12');
  expect(screen.queryByRole('button', { name: '纳入共享地址池' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: '预览库存地址' }));
  expect(await screen.findByRole('button', { name: '纳入共享地址池' })).toBeDisabled();
});
