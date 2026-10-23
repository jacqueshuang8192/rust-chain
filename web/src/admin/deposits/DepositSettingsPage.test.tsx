import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
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
const permissions = vi.hoisted(() => ({ addressOnly: false, readOnly: false }));
vi.mock('../access', () => {
  const canRequest = (path: string, method = 'GET') => (!permissions.readOnly || method === 'GET') && (!permissions.addressOnly || path.includes('/addresses'));
  return {
    AdminRequestActionBoundary: ({ children, endpoint, method }: { children: React.ReactNode; endpoint: string; method: string }) => canRequest(endpoint, method) ? children : null,
    useCanAdminRequest: canRequest
  };
});
if (!Range.prototype.getBoundingClientRect) Range.prototype.getBoundingClientRect = () => new DOMRect(0, 0, 0, 0);
const settings = { scanners: [{ network: 'eth', enabled: true, endpoint_configured: true, api_key_configured: true, confirmations: 12, revision: 7, last_scan_at: null, last_error: null }], assets: [], addresses: [], platform_assets: ['ETH', 'USDT', 'USDC', 'DAI'].map(symbol => ({ symbol, status: 'active', deposit_enabled: true, precision_scale: 18 })) };
const mocked = vi.mocked(apiRequest);
beforeEach(() => { permissions.addressOnly = false; permissions.readOnly = false; mocked.mockReset(); mocked.mockResolvedValue(settings); });
afterEach(cleanup);
function show() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const router = createMemoryRouter([{ path: '/', element: <DepositSettingsPage /> }]);
  render(<QueryClientProvider client={client}><RouterProvider router={router} /></QueryClientProvider>);
}
it('keeps node secrets blank, pins the revision and preserves a failed draft with its reason', async () => {
  const user = userEvent.setup(); show();
  await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  const confirmations = await screen.findByRole('textbox', { name: '充值确认数' });
  expect(screen.getByLabelText('监听节点地址')).toHaveValue('');
  expect(screen.getByLabelText('节点 API 密钥')).toHaveValue('');
  await user.clear(confirmations); await user.type(confirmations, '15');
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '调整确认深度');
  mocked.mockRejectedValueOnce(new ApiError(409, 'CONFLICT', 'configuration conflict'));
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked).toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-settings/scanners/eth', { method: 'PUT', body: JSON.stringify({ enabled: true, endpoint: null, api_key: null, confirmations: 15, expected_revision: 7, reason: '调整确认深度' }) }));
  expect(await screen.findByText(/数据状态已变化/)).toBeInTheDocument();
  expect(screen.getByRole('textbox', { name: '操作原因' })).toHaveValue('调整确认深度');
  expect(confirmations).toHaveValue('15');
});
it('imports addresses directly as a complete batch and preserves a rejected draft', async () => {
  const user = userEvent.setup(); show();
  await user.click(await screen.findByRole('button', { name: '添加收款地址' }));
  const input = await screen.findByRole('textbox', { name: '收款地址批量输入' });
  const first = '0x1111111111111111111111111111111111111111';
  const second = '0x2222222222222222222222222222222222222222';
  await user.type(input, `${first}\n${second}`);
  expect(screen.queryByRole('textbox', { name: '地址库存编号' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: '确认添加地址' }));
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

it('explains read-only access and exposes no write controls for network readers', async () => {
  permissions.readOnly = true;
  mocked.mockResolvedValue({ ...settings, addresses: [{ pool_id: 3, network: 'eth', address: '0x1111111111111111111111111111111111111111', enabled: true }] });
  show();
  expect(await screen.findAllByText('只读权限')).toHaveLength(3);
  for (const name of ['编辑监听配置', '新增监听币种', '添加收款地址', '停用地址']) {
    expect(screen.queryByRole('button', { name })).not.toBeInTheDocument();
  }
  expect(screen.getByRole('combobox', { name: '配置网络' })).toBeEnabled();
  expect(screen.getByRole('button', { name: '刷新配置与监听状态' })).toBeEnabled();
  expect(mocked.mock.calls.every(([, init]) => !init?.method)).toBe(true);
});

async function choose(user: ReturnType<typeof userEvent.setup>, label: string, option: string) {
  await user.click(screen.getByRole('combobox', { name: label }));
  const target = await screen.findByText(option, { selector: '.semi-select-option-text' });
  fireEvent.mouseDown(target); fireEvent.mouseUp(target); fireEvent.click(target);
}

it('adds ETH, USDT and USDC to one network and keeps an explicit entry for more assets', async () => {
  const user = userEvent.setup();
  const data = { ...settings, assets: [] as Array<{ id: number; network: string; asset_symbol: string; asset_kind: string; token_address: string; decimals: number; enabled: boolean; revision: number }> };
  mocked.mockImplementation(async (path, init) => {
    if (init?.method === 'PUT' && path.endsWith('/assets')) {
      const payload = JSON.parse(String(init.body));
      data.assets = [...data.assets, { ...payload, id: data.assets.length + 1, revision: 1 }];
      return undefined as never;
    }
    return structuredClone(data) as never;
  });
  show();
  for (const [symbol, contract, decimals] of [['ETH', '', 18], ['USDT', '0x1111111111111111111111111111111111111111', 6], ['USDC', '0x2222222222222222222222222222222222222222', 6]] as const) {
    await user.click(await screen.findByRole('button', { name: '新增监听币种' }));
    if (contract) {
      await choose(user, '平台币种', symbol);
      await user.type(screen.getByRole('textbox', { name: '合约地址或 mint' }), contract);
      await user.type(screen.getByRole('textbox', { name: '链上精度' }), String(decimals));
    } else {
      expect(screen.getByText('原生币无需填写合约地址，链上精度为 18。')).toBeInTheDocument();
      expect(screen.queryByRole('textbox', { name: '合约地址或 mint' })).not.toBeInTheDocument();
    }
    await user.click(screen.getByRole('button', { name: '添加监听币种' }));
    await user.type(screen.getByRole('textbox', { name: '操作原因' }), `添加 ${symbol}`);
    await user.click(screen.getByRole('button', { name: '确认' }));
    await waitFor(() => expect(screen.queryByRole('textbox', { name: '操作原因' })).not.toBeInTheDocument());
    expect(mocked).toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-settings/assets', { method: 'PUT', body: JSON.stringify({ network: 'eth', asset_symbol: symbol, asset_kind: contract ? 'token' : 'native', token_address: contract, decimals, enabled: true, expected_revision: 0, reason: `添加 ${symbol}` }) });
    expect(await screen.findByRole('button', { name: `编辑 ${symbol} 监听` })).toBeEnabled();
  }
  expect(screen.getByText('监听币种 · 3 个')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: '新增监听币种' }));
  await choose(user, '平台币种', 'DAI');
  expect(screen.getByRole('button', { name: '添加监听币种' })).toBeDisabled();
  expect(screen.getByText('请填写 0 至 18 的链上精度（decimals）')).toBeInTheDocument();
});

it('cancels a dirty form without locking the network or other configuration actions', async () => {
  const user = userEvent.setup(); show();
  await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  await user.clear(screen.getByRole('textbox', { name: '充值确认数' }));
  await user.type(screen.getByRole('textbox', { name: '充值确认数' }), '20');
  await user.click(screen.getByRole('button', { name: '取消编辑' }));
  await user.click(screen.getByRole('button', { name: '继续编辑' }));
  expect(screen.getByRole('textbox', { name: '充值确认数' })).toHaveValue('20');
  await user.click(screen.getByRole('button', { name: '取消编辑' }));
  await user.click(screen.getByRole('button', { name: '放弃草稿' }));
  await waitFor(() => expect(screen.queryByRole('textbox', { name: '充值确认数' })).not.toBeInTheDocument());
  await choose(user, '配置网络', 'Base');
  await user.click(screen.getByRole('button', { name: '添加收款地址' }));
  expect(await screen.findByText('向 Base 添加真实主网地址。可先添加 10 个，后续随时扩容。')).toBeInTheDocument();
  expect(mocked.mock.calls.every(([, init]) => !init?.method)).toBe(true);
});

it('shows immutable chain identity and only updates the selected asset state at its original revision', async () => {
  const user = userEvent.setup();
  const asset = { id: 1, network: 'eth', asset_symbol: 'USDT', asset_kind: 'token', token_address: '0x1111111111111111111111111111111111111111', decimals: 6, enabled: true, revision: 11 };
  mocked.mockResolvedValue({ ...settings, assets: [asset] }); show();
  await user.click(await screen.findByRole('button', { name: '编辑 USDT 监听' }));
  expect(screen.queryByRole('textbox', { name: '合约地址或 mint' })).not.toBeInTheDocument();
  expect(screen.queryByRole('textbox', { name: '链上精度' })).not.toBeInTheDocument();
  expect(screen.getByText(/以保护历史订单归属/)).toBeInTheDocument();
  await user.click(screen.getByRole('switch', { name: '开放该币种充值' }));
  await user.click(screen.getByRole('button', { name: '保存币种状态' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '暂停此币种');
  mocked.mockRejectedValueOnce(new ApiError(409, 'CONFLICT', 'configuration conflict'));
  await user.click(screen.getByRole('button', { name: '确认' }));
  expect(await screen.findByText(/数据状态已变化/)).toBeInTheDocument();
  expect(mocked).toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-settings/assets', { method: 'PUT', body: JSON.stringify({ network: 'eth', asset_symbol: 'USDT', asset_kind: 'token', token_address: asset.token_address, decimals: 6, enabled: false, expected_revision: 11, reason: '暂停此币种' }) });
  expect(screen.getByRole('textbox', { name: '操作原因' })).toHaveValue('暂停此币种');
});

it('preserves the selected network after a successful save and validates native identities per network', async () => {
  const user = userEvent.setup(); show();
  await screen.findByRole('button', { name: '新增监听币种' });
  await choose(user, '配置网络', 'Base');
  await user.click(screen.getByRole('button', { name: '新增监听币种' }));
  await user.click(screen.getByRole('button', { name: '添加监听币种' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '配置 Base ETH');
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(screen.queryByRole('textbox', { name: '操作原因' })).not.toBeInTheDocument());
  expect(screen.getByRole('combobox', { name: '配置网络' })).toHaveTextContent('Base');
  await choose(user, '配置网络', 'Tron（TRC20）');
  await user.click(screen.getByRole('button', { name: '新增监听币种' }));
  expect(screen.getByRole('combobox', { name: '链上资产类型' })).toHaveTextContent('TRC20 代币');
  expect(screen.queryByText('原生币 TRX')).not.toBeInTheDocument();
});
