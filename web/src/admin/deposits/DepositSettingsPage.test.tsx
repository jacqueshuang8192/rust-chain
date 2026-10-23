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
const permissions = vi.hoisted(() => ({ addressOnly: false }));
vi.mock('../access', () => ({ AdminRequestActionBoundary: ({ children }: { children: React.ReactNode }) => children, useCanAdminRequest: (path: string) => !permissions.addressOnly || path.includes('/addresses') }));
if (!Range.prototype.getBoundingClientRect) Range.prototype.getBoundingClientRect = () => new DOMRect(0, 0, 0, 0);
const settings = { scanners: [{ network: 'eth', enabled: true, endpoint_configured: true, api_key_configured: true, confirmations: 12, revision: 7, last_scan_at: null, last_error: null }], assets: [], addresses: [] };
const mocked = vi.mocked(apiRequest);
beforeEach(() => { permissions.addressOnly = false; mocked.mockReset(); mocked.mockResolvedValue(settings); });
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
it('imports addresses directly as a complete batch and preserves a rejected draft', async () => {
  const user = userEvent.setup(); show();
  const input = await screen.findByRole('textbox', { name: '收款地址批量输入' });
  const first = '0x1111111111111111111111111111111111111111';
  const second = '0x2222222222222222222222222222222222222222';
  await user.type(input, `${first}\n${second}`);
  expect(screen.queryByRole('textbox', { name: '地址库存编号' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: '添加收款地址' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '扩容收款地址');
  mocked.mockRejectedValueOnce(new ApiError(400, 'VALIDATION_ERROR', 'invalid address'));
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked).toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-settings/addresses', { method: 'POST', body: JSON.stringify({ network: 'eth', addresses: [first, second], reason: '扩容收款地址' }) }));
  expect(input).toHaveValue(`${first}\n${second}`);
  expect(screen.getByRole('textbox', { name: '操作原因' })).toHaveValue('扩容收款地址');
  expect(mocked.mock.calls.every(([path]) => !path.includes('deposit-address-pool'))).toBe(true);
});

it('loads the address endpoint for address-only operators without requesting network settings', async () => {
  permissions.addressOnly = true;
  mocked.mockResolvedValue([{ pool_id: 3, network: 'eth', address: '0x1111111111111111111111111111111111111111', enabled: true }]);
  show();
  expect(await screen.findByText('0x1111111111111111111111111111111111111111')).toBeInTheDocument();
  expect(mocked).toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-settings/addresses');
  expect(mocked).not.toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-settings');
  expect(screen.queryByLabelText('监听节点地址')).not.toBeInTheDocument();
});
