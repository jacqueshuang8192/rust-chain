import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { beforeEach, afterEach, expect, it, vi } from 'vitest';
import { ApiError, apiRequest } from '../../api/client';
import { listAdminResource } from '../../api/adminResources';
import { authStore } from '../../auth/authStore';
import { AdminAccessProvider } from '../access';
import { CreateTokenProject } from './CreateTokenProject';

vi.mock('../../api/client', async () => ({ ...await vi.importActual<typeof import('../../api/client')>('../../api/client'), apiRequest: vi.fn() }));
vi.mock('../../api/adminResources', () => ({ listAdminResource: vi.fn() }));
const request = vi.mocked(apiRequest);
const session = { scope: 'admin' as const, subject: 'admin:1', generation: 'creation-test', accessToken: 'access', refreshToken: 'refresh' };
const writes = () => request.mock.calls.filter(([, options]) => options?.method === 'POST');
const payload = (index = 0) => JSON.parse(String(writes()[index]?.[1]?.body));
const created = vi.fn();
function mount(permissions = ['*']) {
  const router = createMemoryRouter([{ path: '/', element: <CreateTokenProject onCreated={created} /> }]);
  return render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
    <AdminAccessProvider access={{ admin_id: 1, username: 'creator', role_id: 1, role_name: 'creator', is_super_admin: false, permissions }}><RouterProvider router={router} /></AdminAccessProvider>
  </QueryClientProvider>);
}
async function select(label: string, text: string) {
  const wrapper = [...document.querySelectorAll('label')].find(e => e.textContent?.startsWith(label));
  await userEvent.click(wrapper!.querySelector('.semi-select') as HTMLElement);
  const options = await screen.findAllByText(text, { selector: '.semi-select-option-text' });
  await userEvent.click(options.at(-1)!);
}
async function prepare() {
  await userEvent.click(screen.getByRole('button', { name: '添加新币项目' }));
  await waitFor(() => expect(screen.getByRole('button', { name: '提交添加新币项目' })).toBeEnabled());
  await select('发行资产', 'HIP - Hippo（ID: 11）');
  await select('计价资产', 'USDT - Tether（ID: 3）');
  for (const [label, value] of [['发行总量','1000000'],['申购价格','0.2'],['认购初价','0.3'],['上市参考价','9.99'],['申购开始','2026-11-01T10:00'],['进入派发','2026-11-02T10:00'],['认购开始','2026-11-03T10:00'],['认购结束并上市','2026-11-04T10:00'],['创建原因','direct issuance']]) {
    fireEvent.change(screen.getByLabelText(label), { target: { value } });
  }
  await userEvent.click(screen.getByRole('switch', { name: '申购开始自动操作' }));
  await userEvent.click(screen.getByRole('switch', { name: '认购开始自动操作' }));
  await select('锁仓规则', '购买后按自然月解禁');
  fireEvent.change(screen.getByLabelText('锁仓周期'), { target: { value: '3' } });
  await select('解锁卖出手续费', '按正收益比例');
  fireEvent.change(screen.getByLabelText('退出费用'), { target: { value: '0.2' } });
  await userEvent.click(screen.getByRole('button', { name: '提交添加新币项目' }));
  return screen.findByRole('dialog', { name: '确认创建新版发行项目' });
}
beforeEach(() => {
  sessionStorage.clear(); authStore.setSession(session); request.mockReset(); created.mockReset();
  vi.mocked(listAdminResource).mockResolvedValue({ rows: [{ id: 11, symbol: 'HIP', name: 'Hippo', precision_scale: 8, status: 'active' }, { id: 3, symbol: 'USDT', name: 'Tether', precision_scale: 8, status: 'active' }], total: 2, raw: {} });
  request.mockImplementation(async (_path, options) => ({ ...JSON.parse(String(options?.body)), id: 701 }));
});
afterEach(() => { vi.restoreAllMocks(); authStore.clearSession('admin'); });

it('submits the complete v2 contract with independent nodes and local instants', async () => {
  mount(); const dialog = await prepare();
  expect(writes()).toHaveLength(0);
  await userEvent.click(within(dialog).getByRole('button', { name: '确认创建' }));
  await screen.findByRole('link', { name: '查看已创建项目 701' });
  expect(writes()).toHaveLength(1); expect(writes()[0][0]).toBe('/admin/api/v1/new-coins/issuance');
  expect(payload()).toMatchObject({ asset_id: 11, quote_asset_id: 3, total_supply: '1000000', issue_price: '0.2', reason: 'direct issuance', issuance_policy: {
    version: 2, timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    subscription: { automatic: true, at: new Date('2026-11-01T10:00').getTime() }, distribution: { automatic: false }, purchase: { automatic: true }, listing: { automatic: false },
    lock: { kind: 'months', count: 3 }, fee: { kind: 'profit', rate: '0.2' }, purchase_initial_price: '0.3', listing_reference_price: '9.99',
  } });
  expect(payload().idempotency_key).toBeTruthy(); expect(payload()).not.toHaveProperty('lifecycle_status'); expect(created).toHaveBeenCalledOnce();
});
it('keeps the original intent after lost response and recovers across remounts', async () => {
  request.mockRejectedValueOnce(new Error('response lost'));
  const view = mount(); const dialog = await prepare();
  await userEvent.click(within(dialog).getByRole('button', { name: '确认创建' }));
  await screen.findByRole('button', { name: '按原请求重试' }); const original = payload(); view.unmount();
  mount(); await userEvent.click(screen.getByRole('button', { name: '添加新币项目' }));
  expect(screen.queryByLabelText('申购价格')).not.toBeInTheDocument();
  await userEvent.click(screen.getByRole('button', { name: '核对原创建请求' }));
  await userEvent.click(await screen.findByRole('button', { name: '按原请求重试' }));
  await screen.findByRole('link', { name: '查看已创建项目 701' });
  expect(payload(1)).toEqual(original); expect(created).toHaveBeenCalledOnce();
});
it('retains a mismatched returned contract and locks cancellation while sending', async () => {
  let resolve!: (value: unknown) => void;
  request.mockImplementation(() => new Promise(r => { resolve = r; }));
  mount(); const dialog = await prepare();
  await userEvent.click(within(dialog).getByRole('button', { name: '确认创建' }));
  await waitFor(() => expect(writes()).toHaveLength(1));
  expect(within(dialog).getByRole('button', { name: '取消' })).toBeDisabled();
  const original = payload();
  await act(async () => resolve({ ...original, id: 701, issuance_policy: { ...original.issuance_policy, lock: { kind: 'none' } } }));
  await waitFor(() => expect(screen.getAllByText('返回的发行合同不符，请按原请求重试核对').length).toBeGreaterThan(0));
  expect(created).not.toHaveBeenCalled(); expect(sessionStorage.getItem('admin-token-issuance:admin:1:creation-test')).toContain(original.idempotency_key);
});
it('does not send if durable recovery storage fails', async () => {
  mount(); const dialog = await prepare();
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('storage unavailable'); });
  await userEvent.click(within(dialog).getByRole('button', { name: '确认创建' }));
  await waitFor(() => expect(screen.getAllByText('无法保存创建请求标识，本次未发送。').length).toBeGreaterThan(0));
  expect(writes()).toHaveLength(0);
});
it('allows correcting a definitive duplicate but retains ambiguous conflicts', async () => {
  request.mockRejectedValueOnce(new ApiError(409, 'TOKEN_ISSUANCE_EXISTS', 'already exists'));
  mount(); const dialog = await prepare();
  await userEvent.click(within(dialog).getByRole('button', { name: '确认创建' }));
  await waitFor(() => expect(screen.getAllByText(/already exists/).length).toBeGreaterThan(0));
  await userEvent.click(within(dialog).getByRole('button', { name: '取消' }));
  expect(screen.getByLabelText('申购价格')).toBeEnabled();
});
it('hides creation without write permission and isolates a replacement session', async () => {
  const view = mount(['new_coin.projects.read']); expect(screen.queryByRole('button', { name: '添加新币项目' })).not.toBeInTheDocument(); view.unmount();
  mount(); await prepare();
  await act(async () => authStore.setSession({ ...session, subject: 'admin:2', generation: 'replacement' }));
  expect(screen.queryByRole('dialog', { name: '确认创建新版发行项目' })).not.toBeInTheDocument();
  expect(writes()).toHaveLength(0);
});
