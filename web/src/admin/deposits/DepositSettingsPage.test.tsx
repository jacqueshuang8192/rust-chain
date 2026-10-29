import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { ApiError, apiRequest } from '../../api/client';
import { DepositSettingsPage } from './DepositSettingsPage';
import { rpcPresetFixture } from '../../../tests/fixtures/rpcPresets';
import { parseSettings, networkOptions } from './contracts';
import { assetPresets, nodePresets } from './presets';

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
const settings = { scanners: [{ network: 'eth', providers: [], recovery_mode: null, backfill_cursor_height: null, backfill_target_height: null, backfill_last_error: null, backfill_blocked: false, enabled: true, endpoint_configured: true, api_key_configured: true, confirmations: 12, revision: 7, last_scan_at: null, last_error: null, scan_fresh: false, cursor_height: null, checked_at: 1792800000000 }], assets: [], addresses: [], platform_assets: ['ETH', 'USDT', 'USDC', 'DAI', 'SOL'].map(symbol => ({ symbol, status: 'active', deposit_enabled: true, precision_scale: 18 })) };
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
  expect(screen.queryByRole('combobox', { name: '主网币种预设' })).not.toBeInTheDocument();
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

const networkNames: Record<string, string> = { base: 'Base', btc: 'Bitcoin', eth: 'Ethereum', tron: 'Tron（TRC20）', solana: 'Solana' };
it.each(assetPresets)('applies $id only to a draft and sends the selected mainnet identity on confirmation', async (preset) => {
  const user = userEvent.setup(); show();
  await screen.findByRole('button', { name: '新增监听币种' });
  if (preset.network !== 'eth') await choose(user, '配置网络', networkNames[preset.network]);
  await user.click(screen.getByRole('button', { name: '新增监听币种' }));
  await choose(user, '主网币种预设', preset.label);
  if (preset.kind === 'token') {
    expect(screen.getByRole('textbox', { name: '合约地址或 mint' })).toHaveValue(preset.token);
    expect(screen.getByRole('textbox', { name: '链上精度' })).toHaveValue(String(preset.decimals));
  } else {
    expect(screen.getByText(`原生币无需填写合约地址，链上精度为 ${preset.decimals}。`)).toBeInTheDocument();
  }
  expect(mocked.mock.calls.every(([, init]) => !init?.method)).toBe(true);
  await user.click(screen.getByRole('button', { name: '添加监听币种' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '使用已核对主网预设');
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked).toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-settings/assets', { method: 'PUT', body: JSON.stringify({ network: preset.network, asset_symbol: preset.symbol, asset_kind: preset.kind, token_address: preset.token, decimals: preset.decimals, enabled: true, expected_revision: 0, reason: '使用已核对主网预设' }) }));
});

it.each(nodePresets.filter(preset => (!preset.account || preset.legacy)))('fills $id without enabling admission or sending unresolved credentials', async (preset) => {
  const user = userEvent.setup();
  mocked.mockResolvedValue({ ...settings, scanners: [{ ...settings.scanners[0], network: preset.network, streams: preset.network === 'base' ? ['native', 'token'].map(stream => ({ ...settings.scanners[0], stream })) : [], enabled: false, endpoint_configured: false }] });
  show();
  await screen.findByRole('combobox', { name: '配置网络' });
  if (preset.network !== 'eth') await choose(user, '配置网络', networkNames[preset.network]);
  await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  await choose(user, '主网节点预设', preset.label);
  expect(screen.getByLabelText('监听节点地址')).toHaveValue(preset.endpoint);
  expect(screen.getByRole('textbox', { name: '充值确认数' })).toHaveValue(String(preset.confirmations));
  expect(screen.getByRole('switch', { name: '接受新充值订单' })).not.toBeChecked();
  expect(mocked.mock.calls.every(([, init]) => !init?.method)).toBe(true);
  if (preset.endpoint.includes('{API_KEY}')) {
    expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
    expect(screen.getByText('请将节点地址中的 API_KEY 占位符替换为自己的真实密钥')).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('监听节点地址'), { target: { value: preset.endpoint.replace('{API_KEY}', 'local-fixture-key') } });
  }
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '保存主网节点草稿');
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked).toHaveBeenCalledWith(`/admin/api/v1/wallet/deposit-settings/scanners/${preset.network}`, { method: 'PUT', body: JSON.stringify({ enabled: false, endpoint: preset.endpoint.replace('{API_KEY}', 'local-fixture-key'), api_key: null, confirmations: preset.confirmations, expected_revision: 7, reason: '保存主网节点草稿' }) }));
});

it('removes preset identity when choosing another symbol and protects cancellation', async () => {
  const user = userEvent.setup(); show();
  await user.click(await screen.findByRole('button', { name: '新增监听币种' }));
  await choose(user, '主网币种预设', 'USDT · Ethereum ERC20');
  await choose(user, '平台币种', 'DAI');
  expect(screen.getByLabelText('合约地址或 mint')).toHaveValue('');
  expect(screen.getByRole('textbox', { name: '链上精度' })).toHaveValue('');
  expect(screen.getByRole('button', { name: '添加监听币种' })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: '取消编辑' }));
  await user.click(screen.getByRole('button', { name: '放弃草稿' }));
  expect(mocked.mock.calls.every(([, init]) => !init?.method)).toBe(true);
});

it('disables presets for missing platform assets and excludes existing mappings', async () => {
  const user = userEvent.setup();
  mocked.mockResolvedValue({ ...settings, platform_assets: settings.platform_assets.filter(row => row.symbol !== 'USDC'), assets: [{ id: 1, network: 'eth', asset_symbol: 'USDT', asset_kind: 'token', token_address: assetPresets[1].token, decimals: 6, enabled: false, revision: 3 }] });
  show();
  await user.click(await screen.findByRole('button', { name: '新增监听币种' }));
  await user.click(screen.getByRole('combobox', { name: '主网币种预设' }));
  expect(screen.queryByText('USDT · Ethereum ERC20', { selector: '.semi-select-option-text' })).not.toBeInTheDocument();
  const missing = await screen.findByText('USDC · Ethereum ERC20（请先创建平台资产）');
  expect(missing.closest('[role="option"]')).toHaveAttribute('aria-disabled', 'true');
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

it('offers permission-aware setup shortcuts and explains unavailable payment methods', async () => {
  const user = userEvent.setup();
  const asset = { id: 1, network: 'eth', asset_symbol: 'ETH', asset_kind: 'native', token_address: '', decimals: 18, enabled: true, revision: 1 };
  mocked.mockResolvedValue({ ...settings, assets: [asset] }); show();
  expect(await screen.findByText('收款配置指引')).toBeInTheDocument();
  expect(screen.getByText('可收款支付方式 0 / 1')).toBeInTheDocument();
  expect(screen.getByText('待处理：缺少启用的收款地址；等待扫描确认')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: '配置节点' }));
  expect(screen.getByLabelText('监听节点地址')).toHaveValue('');
  expect(screen.getByRole('button', { name: '刷新配置与监听状态' })).toBeDisabled();
});

it('builds a credential URL from the dedicated provider key without putting it in the Bearer field', async () => {
  const user = userEvent.setup(); show();
  await user.click(await screen.findByRole('button', { name: '配置节点' }));
  await choose(user, '主网节点预设', 'Ethereum · Alchemy（需付费权限）');
  await user.type(screen.getByLabelText('服务商 API Key'), 'fixture/a?b');
  expect(screen.getByLabelText('监听节点地址')).toHaveValue('https://eth-mainnet.g.alchemy.com/v2/fixture%2Fa%3Fb');
  expect(screen.getByLabelText('节点 API 密钥')).toHaveValue('');
  expect(mocked.mock.calls.every(([, init]) => !init?.method)).toBe(true);
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '更新节点');
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked).toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-settings/scanners/eth', { method: 'PUT', body: JSON.stringify({ enabled: true, endpoint: 'https://eth-mainnet.g.alchemy.com/v2/fixture%2Fa%3Fb', api_key: null, confirmations: 12, expected_revision: 7, reason: '更新节点' }) }));
});

it('clears provider key when switching presets and cancels without any save', async () => {
  const user = userEvent.setup();
  mocked.mockResolvedValue({ ...settings, scanners: [{ ...settings.scanners[0], network: 'solana' }] }); show();
  await screen.findByRole('combobox', { name: '配置网络' });
  await choose(user, '配置网络', 'Solana');
  await user.click(screen.getByRole('button', { name: '配置节点' }));
  await choose(user, '主网节点预设', 'Solana · Helius（免费账号）');
  await user.type(screen.getByLabelText('服务商 API Key'), 'first-key');
  await choose(user, '主网节点预设', 'Solana · 公共主网（测试用）');
  expect(screen.queryByLabelText('服务商 API Key')).not.toBeInTheDocument();
  await choose(user, '主网节点预设', 'Solana · Helius（免费账号）');
  expect(screen.getByLabelText('服务商 API Key')).toHaveValue('');
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: '取消编辑' }));
  await user.click(screen.getByRole('button', { name: '放弃草稿' }));
  expect(mocked.mock.calls.every(([, init]) => !init?.method)).toBe(true);
});


it('refreshes saved state while idle and pauses polling during edits and address confirmation', async () => {
  const user = userEvent.setup();
  const interval = vi.spyOn(window, 'setInterval'); const cleared = vi.spyOn(window, 'clearInterval');
  const latestPoll = () => {
    const index = interval.mock.calls.map(call => call[1]).lastIndexOf(15000);
    return { callback: interval.mock.calls[index][0], id: interval.mock.results[index].value };
  };
  try {
    mocked.mockResolvedValue({ ...settings, addresses: [{ pool_id: 3, network: 'eth', address: '0x1111111111111111111111111111111111111111', enabled: true }] }); show();
    await screen.findByRole('button', { name: '配置节点' });
    const first = latestPoll(); const calls = mocked.mock.calls.length;
    await act(async () => { if (typeof first.callback === 'function') first.callback(); });
    await waitFor(() => expect(mocked.mock.calls.length).toBeGreaterThan(calls));
    const editingPoll = latestPoll();
    await user.click(screen.getByRole('button', { name: '配置节点' }));
    expect(cleared).toHaveBeenCalledWith(editingPoll.id);
    await user.click(screen.getByRole('button', { name: '取消编辑' }));
    const addressPoll = latestPoll();
    await user.click(screen.getByRole('button', { name: '停用地址' }));
    expect(cleared).toHaveBeenCalledWith(addressPoll.id);
    expect(screen.getByRole('button', { name: '刷新配置与监听状态' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: '取消' }));
    expect(screen.getByRole('button', { name: '刷新配置与监听状态' })).toBeEnabled();
  } finally { cleanup(); interval.mockRestore(); cleared.mockRestore(); }
});

function withProviders() {
  return { ...settings, scanners: [{ ...settings.scanners[0], providers: [{ id: 10, network: 'eth', name: 'Primary', priority: 0, enabled: true, request_interval_ms: 250, cooldown_until: null, last_error: null, last_success_at: null, keys: [{ id: 20, provider_id: 10, name: 'Key A', priority: 0, enabled: true, api_key_configured: true, request_interval_ms: 250, cooldown_until: null, last_error: null, last_success_at: null }] }] }] };
}
it('adds a backup channel and multiple keys without revealing or replacing existing secrets', async () => {
  const user = userEvent.setup(); mocked.mockResolvedValue(withProviders()); show();
  await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  expect(screen.getByLabelText('渠道 1 节点地址')).toHaveValue('');
  expect(screen.getByLabelText('渠道 1 凭据 1 API Key')).toHaveValue('');
  await user.click(screen.getByRole('button', { name: '添加 API Key · 渠道 1' }));
  await user.type(screen.getByLabelText('渠道 1 凭据 2 API Key'), 'second-key');
  await user.click(screen.getByRole('button', { name: '添加备用渠道' }));
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  await user.type(screen.getByLabelText('渠道 2 节点地址'), 'https://backup.example/rpc');
  await user.type(screen.getByLabelText('渠道 2 凭据 1 API Key'), 'backup-key');
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '增加主备渠道');
  mocked.mockRejectedValueOnce(new ApiError(409, 'CONFLICT', 'changed'));
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked.mock.calls.some(([, init]) => init?.method === 'PUT')).toBe(true));
  const [, init] = mocked.mock.calls.find(([, init]) => init?.method === 'PUT')!;
  const body = JSON.parse(String(init?.body));
  expect(body.expected_revision).toBe(7); expect(body.providers).toHaveLength(2);
  expect(body.providers[0].endpoint).toBeNull();
  expect(body.providers[0].keys[0]).toMatchObject({ id: 20, api_key: null, clear_api_key: false });
  expect(body.providers[0].keys[1].api_key).toBe('second-key');
  expect(body.providers[1]).toMatchObject({ id: null, endpoint: 'https://backup.example/rpc', priority: 1, request_interval_ms: 250 });
  expect(await screen.findByText(/数据状态已变化/)).toBeInTheDocument();
  expect(screen.getByLabelText('渠道 2 凭据 1 API Key')).toHaveValue('backup-key');
  expect(screen.getByRole('textbox', { name: '操作原因' })).toHaveValue('增加主备渠道');
});
it('validates channel intervals and requires a key for URL credential templates', async () => {
  const user = userEvent.setup(); mocked.mockResolvedValue(withProviders()); show();
  await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  const interval = screen.getByLabelText('渠道 1 请求间隔');
  await user.clear(interval); await user.type(interval, '1e3');
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  await user.clear(interval); await user.type(interval, '500');
  await user.click(screen.getByRole('button', { name: '添加备用渠道' }));
  fireEvent.change(screen.getByLabelText('渠道 2 节点地址'), { target: { value: 'https://backup.example/rpc/{API_KEY}' } });
  expect(await screen.findByText(/渠道 2 的 URL 模板需要 API Key/)).toBeInTheDocument();
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  await user.type(screen.getByLabelText('渠道 2 凭据 1 API Key'), 'real-fixture-key');
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeEnabled();
});
it('shows persisted channel and key cooldown states for read-only users', async () => {
  permissions.readOnly = true;
  const data = withProviders();
  mocked.mockResolvedValue({ ...data, scanners: [{ ...data.scanners[0], providers: [{ ...data.scanners[0].providers[0], cooldown_until: settings.scanners[0].checked_at + 60000, last_error: 'HTTP 503 upstream unavailable', keys: [{ ...data.scanners[0].providers[0].keys[0], cooldown_until: settings.scanners[0].checked_at + 120000, last_error: 'HTTP 429 limited' }] }] }] });
  show();
  expect(await screen.findByText('渠道冷却')).toBeInTheDocument();
  expect(screen.getByText(/HTTP 429 limited/)).toBeInTheDocument();
  expect(screen.getByText(/HTTP 503 upstream unavailable/)).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: '编辑监听配置' })).not.toBeInTheDocument();
});

it('submits dual progress with the pinned revision and shows the queued state', async () => {
  const user = userEvent.setup();
  const scanner = { ...settings.scanners[0], cursor_height: 80 };
  mocked.mockResolvedValue({ ...settings, scanners: [scanner] }); show();
  await user.click(await screen.findByRole('button', { name: '恢复到最新区块' }));
  expect(screen.getByRole('combobox', { name: '历史区块处理方式' })).toHaveTextContent('双进度');
  await user.click(screen.getByRole('button', { name: '提交恢复请求' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '恢复最新监听并补扫');
  mocked.mockResolvedValue({ ...settings, scanners: [{ ...scanner, revision: 8, recovery_mode: 'backfill' }] });
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked).toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-settings/scanners/eth/recovery', { method: 'POST', body: JSON.stringify({ mode: 'backfill', expected_revision: 7, acknowledge_skip: false, reason: '恢复最新监听并补扫' }) }));
  expect(await screen.findByText(/恢复请求已排队/)).toBeInTheDocument();
  expect(screen.getByRole('button', { name: '恢复到最新区块' })).toBeDisabled();
});
it('requires skip acknowledgement and retains failed recovery mode, reason and revision', async () => {
  const user = userEvent.setup();
  mocked.mockResolvedValue({ ...settings, scanners: [{ ...settings.scanners[0], cursor_height: 100, backfill_cursor_height: 50, backfill_target_height: 100 }] }); show();
  await user.click(await screen.findByRole('button', { name: '恢复到最新区块' }));
  await choose(user, '历史区块处理方式', '手动跳过：不再扫描历史区间');
  expect(screen.getByText(/将放弃从进度 50/)).toBeInTheDocument();
  expect(screen.getByRole('button', { name: '提交恢复请求' })).toBeDisabled();
  await user.click(screen.getByRole('switch', { name: '确认跳过历史风险' }));
  await user.click(screen.getByRole('button', { name: '提交恢复请求' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '已核对停机区间');
  mocked.mockRejectedValueOnce(new ApiError(409, 'CONFLICT', 'changed'));
  await user.click(screen.getByRole('button', { name: '确认' }));
  expect(await screen.findByText(/数据状态已变化/)).toBeInTheDocument();
  expect(mocked).toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-settings/scanners/eth/recovery', { method: 'POST', body: JSON.stringify({ mode: 'skip', expected_revision: 7, acknowledge_skip: true, reason: '已核对停机区间' }) });
  expect(screen.getByRole('textbox', { name: '操作原因' })).toHaveValue('已核对停机区间');
  await user.click(screen.getByRole('button', { name: '取消' }));
  await choose(user, '历史区块处理方式', '双进度：跳到最新并补扫历史');
  await choose(user, '历史区块处理方式', '手动跳过：不再扫描历史区间');
  expect(screen.getByRole('switch', { name: '确认跳过历史风险' })).not.toBeChecked();
});
it('shows separate historical errors to readers without recovery write access', async () => {
  permissions.readOnly = true;
  mocked.mockResolvedValue({ ...settings, scanners: [{ ...settings.scanners[0], cursor_height: 110, backfill_cursor_height: 80, backfill_target_height: 100, backfill_last_error: 'anchor mismatch', backfill_blocked: true }] }); show();
  expect(await screen.findByText('历史补扫 · 异常待核对')).toBeInTheDocument();
  expect(screen.getByText(/已扫描区块高度：80 \/ 100/)).toBeInTheDocument();
  expect(screen.getByText(/已阻止新订单准入/)).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: '恢复到最新区块' })).not.toBeInTheDocument();
});
it('does not allow recovery before the scanner has a checkpoint', async () => {
  show();
  expect(await screen.findByRole('button', { name: '恢复到最新区块' })).toBeDisabled();
});

it('shows isolated Base progress and recovers only the chosen token stream', async () => {
  const user = userEvent.setup();
  const common = { ...settings.scanners[0], network: 'base', cursor_height: 100, last_scan_at: 1792800000000 };
  const token = { ...common, stream: 'token', cursor_height: 120, scan_fresh: true };
  const native = { ...common, stream: 'native', last_error: 'RPC native method unavailable', scan_fresh: false };
  mocked.mockResolvedValue({ ...settings, scanners: [{ ...common, streams: [native, token], last_error: native.last_error }] });
  show();
  await screen.findByRole('combobox', { name: '配置网络' });
  await choose(user, '配置网络', 'Base');
  const nativeRegion = await screen.findByRole('region', { name: 'ETH 原生币监听状态' });
  const tokenRegion = screen.getByRole('region', { name: 'ERC20 代币（包括 USDC）监听状态' });
  expect(within(nativeRegion).getByText('扫描异常')).toBeInTheDocument();
  expect(within(tokenRegion).getByText('已追上区块')).toBeInTheDocument();
  expect(within(tokenRegion).getByText('已扫描区块高度：120')).toBeInTheDocument();
  await user.click(within(tokenRegion).getByRole('button', { name: '恢复 ERC20 到最新区块' }));
  expect(await screen.findByText(/恢复范围：ERC20/)).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: '提交恢复请求' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '恢复代币分流');
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked).toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-settings/scanners/base/recovery', { method: 'POST', body: JSON.stringify({ stream: 'token', mode: 'backfill', expected_revision: 7, acknowledge_skip: false, reason: '恢复代币分流' }) }));
});


it.each(['base', 'tron', 'btc', 'solana'])('prefills anonymous defaults for %s without enabling deposits', async network => {
  const user = userEvent.setup();
  mocked.mockResolvedValue({ ...settings, rpc_presets: rpcPresetFixture, scanners: [{ ...settings.scanners[0], network, streams: network === 'base' ? ['native', 'token'].map(stream => ({ ...settings.scanners[0], stream })) : [], enabled: false, endpoint_configured: false, api_key_configured: false }] });
  show(); await screen.findByRole('combobox', { name: '配置网络' });
  if (network !== 'eth') await choose(user, '配置网络', networkNames[network]);
  await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  const defaults = rpcPresetFixture.filter(preset => preset.network === network && preset.recommended);
  expect(screen.getByRole('switch', { name: '接受新充值订单' })).not.toBeChecked();
  for (const [index, preset] of defaults.entries()) {
    expect(screen.getByLabelText(`渠道 ${index + 1} 名称`)).toHaveValue(preset.name);
    expect(screen.getByText(preset.endpoint)).toBeInTheDocument();
    expect(screen.queryByLabelText(`渠道 ${index + 1} 凭据 1 API Key`)).not.toBeInTheDocument();
  }
  expect(screen.getByRole('button', { name: '补充免费默认渠道' })).toBeDisabled();
  expect(mocked.mock.calls.every(([, init]) => !init?.method)).toBe(true);
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '启用默认渠道草稿');
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked.mock.calls.some(([, init]) => init?.method === 'PUT')).toBe(true));
  const body = JSON.parse(String(mocked.mock.calls.find(([, init]) => init?.method === 'PUT')![1]?.body));
  expect(body).toMatchObject({ enabled: false, expected_revision: 7, endpoint: null, api_key: null });
  expect(body.providers.map((row: { preset_id: string }) => row.preset_id)).toEqual(defaults.map(preset => preset.id));
  for (const row of body.providers) {
    expect(row.endpoint).toBeNull(); expect(row.keys).toHaveLength(1);
    expect(row.keys[0]).toMatchObject({ api_key: null, clear_api_key: false });
  }
});

it('appends an account template once, retains custom keys, and preserves a conflicting draft', async () => {
  const user = userEvent.setup(); mocked.mockResolvedValue({ ...withProviders(), rpc_presets: rpcPresetFixture }); show();
  await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  expect(screen.getByRole('button', { name: '补充免费默认渠道' })).toBeDisabled();
  await user.click(screen.getByRole('button', { name: '添加所选账号渠道' }));
  expect(screen.getByRole('button', { name: '添加所选账号渠道' })).toBeDisabled();
  expect(screen.getByRole('button', { name: '补充免费默认渠道' })).toBeDisabled();
  expect(screen.getByLabelText('渠道 1 名称')).toHaveValue('Primary');
  expect(screen.getByLabelText('渠道 1 凭据 1 API Key')).toHaveValue('');
  expect(screen.getByLabelText('渠道 2 名称')).toHaveValue('Chainnodes · Ethereum');
  expect(screen.getByLabelText('渠道 2 节点地址')).toHaveValue('https://mainnet.chainnodes.org/{API_KEY}');
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  await user.type(screen.getByLabelText('渠道 2 凭据 1 API Key'), 'fixture/a?b');
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '补充免费备用');
  mocked.mockRejectedValueOnce(new ApiError(409, 'CONFLICT', 'changed'));
  await user.click(screen.getByRole('button', { name: '确认' }));
  expect(await screen.findByText(/数据状态已变化/)).toBeInTheDocument();
  const body = JSON.parse(String(mocked.mock.calls.find(([, init]) => init?.method === 'PUT')![1]?.body));
  expect(body.providers).toHaveLength(2);
  expect(body.providers[0]).toMatchObject({ id: 10, preset_id: '', endpoint: null, keys: [{ id: 20, api_key: null, clear_api_key: false }] });
  expect(body.providers[1]).toMatchObject({ preset_id: '', endpoint: 'https://mainnet.chainnodes.org/{API_KEY}', priority: 1, keys: [{ id: null, api_key: 'fixture/a?b', clear_api_key: false }] });
  expect(screen.getByRole('textbox', { name: '操作原因' })).toHaveValue('补充免费备用');
  expect(screen.getByLabelText('渠道 2 名称')).toHaveValue('Chainnodes · Ethereum');
  expect(screen.getByLabelText('渠道 2 凭据 1 API Key')).toHaveValue('fixture/a?b');
});

it('explicitly switches a custom provider to anonymous preset and back to custom', async () => {
  const user = userEvent.setup(); mocked.mockResolvedValue({ ...withProviders(), rpc_presets: rpcPresetFixture }); show();
  await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  await choose(user, '渠道 1 来源', 'dRPC · Ethereum');
  expect(screen.getByText(/会清除本渠道原有密钥/)).toBeInTheDocument();
  expect(screen.queryByLabelText('渠道 1 凭据 1 API Key')).not.toBeInTheDocument();
  await choose(user, '渠道 1 来源', '自定义 RPC / 自有 API Key');
  expect(screen.getByLabelText('渠道 1 节点地址')).toHaveValue('https://eth.drpc.org');
  expect(screen.getByRole('switch', { name: '清除渠道 1 凭据 1 密钥' })).toBeChecked();
  await choose(user, '渠道 1 来源', 'dRPC · Ethereum');
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '切换免费渠道');
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked.mock.calls.some(([, init]) => init?.method === 'PUT')).toBe(true));
  const body = JSON.parse(String(mocked.mock.calls.find(([, init]) => init?.method === 'PUT')![1]?.body));
  expect(body.providers[0]).toMatchObject({ id: 10, preset_id: 'eth-drpc', endpoint: null, keys: [{ id: 20, api_key: null, clear_api_key: true }] });
});

it.each([undefined, '800'])('keeps saved free-channel identities and synchronizes an edited interval %s', async interval => {
  const data = withProviders(); const provider = data.scanners[0].providers[0];
  mocked.mockResolvedValue({ ...data, rpc_presets: rpcPresetFixture, scanners: [{ ...data.scanners[0], providers: [{ ...provider, preset_id: 'eth-drpc', keys: [{ ...provider.keys[0], api_key_configured: false }] }] }] });
  const user = userEvent.setup(); show();
  expect(await screen.findByText(/内置免费渠道 · dRPC/)).toBeInTheDocument();
  expect(screen.getByText('尚未验证')).toBeInTheDocument();
  await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  expect(screen.getByRole('button', { name: '补充免费默认渠道' })).toBeDisabled();
  if (interval) fireEvent.change(screen.getByLabelText('渠道 1 请求间隔'), { target: { value: interval } });
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '保存原有设置');
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked.mock.calls.some(([, init]) => init?.method === 'PUT')).toBe(true));
  const body = JSON.parse(String(mocked.mock.calls.find(([, init]) => init?.method === 'PUT')![1]?.body));
  expect(body.providers[0]).toMatchObject({ id: 10, preset_id: 'eth-drpc', endpoint: null, request_interval_ms: Number(interval ?? 250), keys: [{ id: 20, api_key: null, clear_api_key: false, request_interval_ms: Number(interval ?? 250) }] });
});

it('rejects malformed catalogs and accepts absent catalogs for older responses', () => {
  expect(parseSettings(settings).rpc_presets).toEqual([]);
  expect(parseSettings({ ...settings, rpc_presets: rpcPresetFixture }).rpc_presets).toHaveLength(8);
  for (const rpc_presets of [null, {}, [{ ...rpcPresetFixture[0], source: 'javascript:alert(1)' }], [{ ...rpcPresetFixture[0], recommended: 'true' }], [rpcPresetFixture[0], rpcPresetFixture[0]]]) {
    expect(() => parseSettings({ ...settings, rpc_presets })).toThrow();
  }
});

it('starts Ethereum with an unsaved account template and requires its own key', async () => {
  const user = userEvent.setup();
  mocked.mockResolvedValue({ ...settings, rpc_presets: rpcPresetFixture, scanners: [{ ...settings.scanners[0], enabled: false, endpoint_configured: false, api_key_configured: false }] });
  show(); await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  expect(screen.getByRole('switch', { name: '接受新充值订单' })).not.toBeChecked();
  expect(screen.getByLabelText('渠道 1 名称')).toHaveValue('Chainnodes · Ethereum');
  expect(screen.getByLabelText('渠道 1 节点地址')).toHaveValue('https://mainnet.chainnodes.org/{API_KEY}');
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  expect(screen.getByRole('button', { name: '添加所选账号渠道' })).toBeDisabled();
  expect(screen.getByRole('link', { name: '开通渠道 / 获取凭据' })).toHaveAttribute('href', 'https://app.chainnodes.org/');
  await choose(user, '账号渠道模板', 'Ethereum · SolidRPC（免费备用）');
  expect(screen.getByRole('link', { name: '开通渠道 / 获取凭据' })).toHaveAttribute('href', 'https://solidrpc.io/register');
  await user.click(screen.getByRole('button', { name: '添加所选账号渠道' }));
  expect(screen.getByLabelText('渠道 2 节点地址')).toHaveValue('https://rpc.solidrpc.io/{API_KEY}/evm/1');
  await user.type(screen.getByLabelText('渠道 1 凭据 1 API Key'), 'chainnodes-fixture');
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  await user.type(screen.getByLabelText('渠道 2 凭据 1 API Key'), 'solid-fixture');
  expect(mocked.mock.calls.every(([, init]) => !init?.method)).toBe(true);
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '配置免费账号主备');
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked.mock.calls.some(([, init]) => init?.method === 'PUT')).toBe(true));
  const body = JSON.parse(String(mocked.mock.calls.find(([, init]) => init?.method === 'PUT')![1]?.body));
  expect(body).toMatchObject({ enabled: false, expected_revision: 7, endpoint: null, api_key: null });
  expect(body.providers).toHaveLength(2);
  expect(body.providers[0]).toMatchObject({ preset_id: '', priority: 0, endpoint: 'https://mainnet.chainnodes.org/{API_KEY}', keys: [{ api_key: 'chainnodes-fixture' }] });
  expect(body.providers[1]).toMatchObject({ preset_id: '', priority: 1, endpoint: 'https://rpc.solidrpc.io/{API_KEY}/evm/1', request_interval_ms: 1000, keys: [{ api_key: 'solid-fixture', request_interval_ms: 1000 }] });
  expect(body.providers.every((provider: Record<string, unknown>) => !('template_id' in provider))).toBe(true);
});

it('replaces template credentials without reusing a saved key and cancels without writing', async () => {
  const user = userEvent.setup(); mocked.mockResolvedValue({ ...withProviders(), rpc_presets: rpcPresetFixture }); show();
  await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  await choose(user, '渠道 1 服务商模板', 'Ethereum · Chainnodes（免费账号）');
  expect(screen.getByLabelText('渠道 1 凭据 1 API Key')).toHaveValue('');
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  expect(screen.queryByRole('switch', { name: '清除渠道 1 凭据 1 密钥' })).not.toBeInTheDocument();
  await user.type(screen.getByLabelText('渠道 1 凭据 1 API Key'), 'first-provider-fixture');
  await choose(user, '渠道 1 服务商模板', 'Ethereum · SolidRPC（免费备用）');
  expect(screen.getByLabelText('渠道 1 凭据 1 API Key')).toHaveValue('');
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  await user.type(screen.getByLabelText('渠道 1 凭据 1 API Key'), 'replacement-fixture');
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '更换账号渠道');
  mocked.mockRejectedValueOnce(new ApiError(409, 'CONFLICT', 'changed'));
  await user.click(screen.getByRole('button', { name: '确认' }));
  expect(await screen.findByText(/数据状态已变化/)).toBeInTheDocument();
  const body = JSON.parse(String(mocked.mock.calls.find(([, init]) => init?.method === 'PUT')![1]?.body));
  expect(body.providers).toHaveLength(1);
  expect(body.providers[0]).toMatchObject({ id: 10, endpoint: 'https://rpc.solidrpc.io/{API_KEY}/evm/1', keys: [{ id: null, api_key: 'replacement-fixture', clear_api_key: false }] });
  await user.click(screen.getByRole('button', { name: '取消' }));
  await user.click(screen.getByRole('button', { name: '取消编辑' }));
  await user.click(screen.getByRole('button', { name: '放弃草稿' }));
  expect(mocked.mock.calls.filter(([, init]) => init?.method === 'PUT')).toHaveLength(1);
});

it('previews priority changes and preserves credentials when channel forms are collapsed', async () => {
  const user = userEvent.setup();
  const data = withProviders(); const first = data.scanners[0].providers[0];
  mocked.mockResolvedValue({ ...data, scanners: [{ ...data.scanners[0], providers: [first, { ...first, id: 11, name: 'Standby', priority: 1, keys: [{ ...first.keys[0], id: 21, provider_id: 11 }] }] }] });
  show(); await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  expect(screen.getByLabelText('渠道 1 名称')).toBeVisible();
  expect(screen.getByLabelText('渠道 2 名称')).not.toBeVisible();
  await user.click(screen.getByRole('button', { name: '展开全部渠道' }));
  fireEvent.change(screen.getByLabelText('渠道 1 优先级'), { target: { value: '5' } });
  await user.type(screen.getByLabelText('渠道 2 凭据 1 API Key'), 'independent-fixture');
  const preview = screen.getByRole('region', { name: '主备尝试顺序' });
  expect(within(preview).getAllByRole('listitem')[0]).toHaveTextContent('首选Standby');
  await user.click(screen.getByRole('button', { name: '收起全部渠道' }));
  expect(screen.getByLabelText('渠道 2 凭据 1 API Key')).not.toBeVisible();
  await user.click(screen.getByRole('button', { name: '展开全部渠道' }));
  expect(screen.getByLabelText('渠道 2 凭据 1 API Key')).toHaveValue('independent-fixture');
  expect(screen.getByLabelText('渠道 1 凭据 1 API Key')).toHaveValue('');
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '调整主备顺序');
  mocked.mockRejectedValueOnce(new ApiError(409, 'CONFLICT', 'changed'));
  await user.click(screen.getByRole('button', { name: '确认' }));
  expect(await screen.findByText(/数据状态已变化/)).toBeInTheDocument();
  const body = JSON.parse(String(mocked.mock.calls.find(([, init]) => init?.method === 'PUT')![1]?.body));
  expect(body.expected_revision).toBe(7);
  expect(body.providers[0]).toMatchObject({ id: 10, priority: 5, endpoint: null, keys: [{ id: 20, api_key: null }] });
  expect(body.providers[1]).toMatchObject({ id: 11, priority: 1, endpoint: null, keys: [{ id: 21, api_key: 'independent-fixture' }] });
  expect(screen.getByLabelText('渠道 2 凭据 1 API Key')).toHaveValue('independent-fixture');
});


it.each(nodePresets.filter(preset => preset.account))('configures $id with fresh credentials and the correct mainnet endpoint', async preset => {
  const data = withProviders();
  data.scanners[0].network = preset.network;
  data.scanners[0].providers[0].network = preset.network;
  mocked.mockResolvedValue({ ...data, rpc_presets: rpcPresetFixture, scanners: data.scanners.map(scanner => ({ ...scanner, streams: preset.network === 'base' ? ['native', 'token'].map(stream => ({ ...scanner, stream })) : [] })) });
  const user = userEvent.setup(); show();
  await screen.findByRole('combobox', { name: '配置网络' });
  await choose(user, '配置网络', networkOptions.find(row => row.value === preset.network)!.label);
  await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  await choose(user, '渠道 1 服务商模板', preset.label);
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  expect(screen.getByLabelText('渠道 1 凭据 1 API Key')).toHaveValue('');
  expect(screen.queryByRole('switch', { name: '清除渠道 1 凭据 1 密钥' })).not.toBeInTheDocument();
  const endpoint = preset.account!.credential === 'endpoint' ? 'https://node.fixture.example/project-token' : preset.endpoint;
  if (preset.account!.credential === 'endpoint') fireEvent.change(screen.getByLabelText('渠道 1 节点地址'), { target: { value: endpoint } });
  else fireEvent.change(screen.getByLabelText('渠道 1 凭据 1 API Key'), { target: { value: 'new-provider-fixture' } });
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeEnabled();
  await user.click(screen.getByRole('button', { name: '保存监听配置' }));
  fireEvent.change(screen.getByRole('textbox', { name: '操作原因' }), { target: { value: '配置独立备用服务商' } });
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked.mock.calls.some(([, init]) => init?.method === 'PUT')).toBe(true));
  const call = mocked.mock.calls.find(([, init]) => init?.method === 'PUT')!;
  expect(call[0]).toBe(`/admin/api/v1/wallet/deposit-settings/scanners/${preset.network}`);
  const body = JSON.parse(String(call[1]?.body));
  expect(body).toMatchObject({ expected_revision: 7, endpoint: null, api_key: null });
  expect(body.providers[0]).toMatchObject({ id: 10, preset_id: '', endpoint, keys: [{ id: null, api_key: preset.account!.credential === 'endpoint' ? null : 'new-provider-fixture', clear_api_key: false }] });
  expect(body.providers[0]).not.toHaveProperty('template_id');
  expect(body.providers[0]).not.toHaveProperty('replacementEndpoint');
});

it('requires a replacement URL after selecting a dashboard template even when its selection is cleared', async () => {
  const user = userEvent.setup(); mocked.mockResolvedValue({ ...withProviders(), rpc_presets: rpcPresetFixture }); show();
  await user.click(await screen.findByRole('button', { name: '编辑监听配置' }));
  await choose(user, '渠道 1 服务商模板', 'Ethereum · Google Cloud（免费账号）');
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  for (const marker of ['YOUR_API_KEY', '<API_KEY>', '%7BAPI_KEY%7D', '{api_key}']) {
    fireEvent.change(screen.getByLabelText('渠道 1 节点地址'), { target: { value: `https://node.fixture.example/rpc?key=${marker}` } });
    expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  }
  fireEvent.change(screen.getByLabelText('渠道 1 节点地址'), { target: { value: 'https://node.fixture.example/project-key' } });
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeEnabled();
  fireEvent.change(screen.getByLabelText('渠道 1 节点地址'), { target: { value: '' } });
  await choose(user, '渠道 1 服务商模板', '选择模板填入地址');
  expect(screen.getByRole('button', { name: '保存监听配置' })).toBeDisabled();
  expect(mocked.mock.calls.every(([, init]) => !init?.method)).toBe(true);
});
