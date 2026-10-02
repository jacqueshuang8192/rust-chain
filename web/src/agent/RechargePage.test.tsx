import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { apiRequest, ApiError, ApiNetworkError } from '../api/client';
import { authStore } from '../auth/authStore';
import { AgentRechargePage } from './RechargePage';
import { rechargeFixture } from '../recharges/testFixtures';
import { FINANCIAL_COMMAND_STORAGE_KEY } from '../shared/idempotency';

vi.mock('../api/client', async () => ({ ...await vi.importActual<typeof import('../api/client')>('../api/client'), apiRequest: vi.fn() }));
const request = vi.mocked(apiRequest);
let precision: number;
let direct: boolean;
const writes = () => request.mock.calls.filter(([, init]) => init?.method === 'POST');
function mount(path = '/agent/users/42/recharge') {
  return render(<RouterProvider router={createMemoryRouter([
    { path: '/agent/users/:userId/recharge', element: <AgentRechargePage /> },
    { path: '/agent/recharges', element: <AgentRechargePage /> }
  ], { initialEntries: [path] })} />);
}
async function fill(amount = rechargeFixture.amount) {
  await screen.findByText('当前新申请：需管理员审核');
  fireEvent.click(screen.getByRole('combobox', { name: '充值资产' }));
  const option = await screen.findByText(`USDT（${precision} 位小数）`);
  fireEvent.mouseDown(option); fireEvent.mouseUp(option); fireEvent.click(option);
  fireEvent.change(screen.getByRole('textbox', { name: '充值金额' }), { target: { value: amount } });
  fireEvent.change(screen.getByRole('textbox', { name: '申请原因' }), { target: { value: ` ${rechargeFixture.reason} ` } });
}
beforeEach(() => {
  sessionStorage.clear();
  authStore.setSession({ scope: 'agent', subject: 'agent:3', generation: 'agent-session', accessToken: 'agent-token', refreshToken: 'refresh' });
  request.mockReset(); precision = 18; direct = false;
  request.mockImplementation(async (path, init) => {
    if (path.endsWith('/options')) return { direct_recharge_enabled: direct, assets: [{ asset_id: 1, symbol: 'USDT', precision_scale: precision }] };
    if (init?.method === 'POST') return { ...rechargeFixture, ...JSON.parse(String(init.body)) };
    return { recharges: [], total: 0 };
  });
});
afterEach(() => vi.restoreAllMocks());

describe('agent recharge workflow', () => {
  it('pins the user, transmits exact amount, and does not double-submit', async () => {
    let resolve!: (value: unknown) => void;
    const base = request.getMockImplementation()!;
    request.mockImplementation((path, init) => init?.method === 'POST' ? new Promise((done) => { resolve = done; }) : base(path, init));
    mount();
    await fill();
    expect(screen.getByRole('textbox', { name: '充值用户ID' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: '提交充值申请' }));
    const submit = await screen.findByRole('button', { name: '确认提交' });
    fireEvent.click(submit); fireEvent.click(submit);
    await waitFor(() => expect(writes()).toHaveLength(1));
    expect(JSON.parse(String(writes()[0][1]?.body))).toEqual({
      asset_id: 1, amount: rechargeFixture.amount, reason: rechargeFixture.reason, idempotency_key: expect.any(String)
    });
    expect(writes()[0][0]).toBe('/agent/api/v1/users/42/recharge');
    expect(writes()[0][1]?.authScope).toBe('agent');
    await act(async () => resolve(rechargeFixture));
    expect(await screen.findByText(/申请 7：待审核/)).toBeInTheDocument();
    expect(sessionStorage.getItem(FINANCIAL_COMMAND_STORAGE_KEY)).toBeNull();
  });
  it('restores immutable pending input after remount and retries the same key after precision tightens', async () => {
    const base = request.getMockImplementation()!;
    let count = 0;
    request.mockImplementation((path, init) => init?.method === 'POST' && count++ === 0
      ? Promise.reject(new ApiNetworkError('lost response')) : base(path, init));
    const view = mount(); await fill();
    fireEvent.click(screen.getByRole('button', { name: '提交充值申请' }));
    fireEvent.click(await screen.findByRole('button', { name: '确认提交' }));
    await screen.findByRole('button', { name: '按原请求重试' });
    const firstBody = writes()[0][1]?.body;
    view.unmount(); precision = 2;
    mount();
    await screen.findByText('当前新申请：需管理员审核');
    expect(screen.getByRole('textbox', { name: '充值金额' })).toHaveValue(rechargeFixture.amount);
    expect(screen.getByRole('textbox', { name: '申请原因' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: '核对原请求' }));
    fireEvent.click(await screen.findByRole('button', { name: '按原请求重试' }));
    await waitFor(() => expect(writes()).toHaveLength(2));
    expect(writes()[1][1]?.body).toBe(firstBody);
    expect(await screen.findByText(/申请 7：待审核/)).toBeInTheDocument();
  });
  it.each(['100000000000000000000', '0.0000000000000000001', '0', '-1'])('rejects capacity/positivity violation %s before HTTP', async (value) => {
    mount(); await fill(value);
    fireEvent.click(screen.getByRole('button', { name: '提交充值申请' }));
    expect(await screen.findByText(/充值金额必须为正数/)).toBeInTheDocument();
    expect(writes()).toHaveLength(0);
  });
  it.each([401, 403, 404])('retains the original key after a lost response followed by HTTP %s', async (status) => {
    const base = request.getMockImplementation()!;
    let count = 0;
    request.mockImplementation((path, init) => {
      if (init?.method !== 'POST') return base(path, init);
      count += 1;
      if (count === 1) return Promise.reject(new ApiNetworkError('lost response'));
      if (count === 2) return Promise.reject(new ApiError(status, 'DENIED', 'scope changed'));
      return base(path, init);
    });
    mount(); await fill();
    fireEvent.click(screen.getByRole('button', { name: '提交充值申请' }));
    fireEvent.click(await screen.findByRole('button', { name: '确认提交' }));
    await waitFor(() => expect(screen.getByRole('button', { name: '按原请求重试' })).not.toBeDisabled());
    fireEvent.click(screen.getByRole('button', { name: '按原请求重试' }));
    await waitFor(() => expect(writes()).toHaveLength(2));
    await waitFor(() => expect(screen.getByRole('button', { name: '按原请求重试' })).not.toBeDisabled());
    expect(screen.getByRole('textbox', { name: '充值金额' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: '按原请求重试' }));
    expect(await screen.findByText(/申请 7：待审核/)).toBeInTheDocument();
    expect(writes()).toHaveLength(3);
    expect(writes().map(([, init]) => init?.body)).toEqual(Array(3).fill(writes()[0][1]?.body));
  });
  it('rejects new overprecision input and accepts meaningful precision with trailing zeros', async () => {
    precision = 2;
    mount(); await fill('1.001');
    fireEvent.click(screen.getByRole('button', { name: '提交充值申请' }));
    expect(await screen.findByText(/该资产最多支持 2 位小数/)).toBeInTheDocument();
    expect(writes()).toHaveLength(0);
    fireEvent.change(screen.getByRole('textbox', { name: '充值金额' }), { target: { value: '1.2300' } });
    fireEvent.click(screen.getByRole('button', { name: '提交充值申请' }));
    fireEvent.click(await screen.findByRole('button', { name: '确认提交' }));
    await waitFor(() => expect(writes()).toHaveLength(1));
    expect(JSON.parse(String(writes()[0][1]?.body)).amount).toBe('1.23');
  });
  it('fails closed when request identity cannot be persisted', async () => {
    mount(); await fill();
    fireEvent.click(screen.getByRole('button', { name: '提交充值申请' }));
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('full'); });
    fireEvent.click(await screen.findByRole('button', { name: '确认提交' }));
    await waitFor(() => expect(screen.getAllByRole('alert').some((node) => node.textContent?.includes('本次未发送'))).toBe(true));
    expect(writes()).toHaveLength(0);
  });
  it('separates drafts by agent account and session generation', async () => {
    const base = request.getMockImplementation()!;
    request.mockImplementation((path, init) => init?.method === 'POST' ? Promise.reject(new ApiNetworkError('lost')) : base(path, init));
    mount(); await fill();
    fireEvent.click(screen.getByRole('button', { name: '提交充值申请' }));
    fireEvent.click(await screen.findByRole('button', { name: '确认提交' }));
    await screen.findByRole('button', { name: '按原请求重试' });
    act(() => { authStore.setSession({ scope: 'agent', subject: 'agent:other', accessToken: 'new', refreshToken: 'new' }); });
    await waitFor(() => expect(screen.getByRole('textbox', { name: '充值金额' })).toHaveValue(''));
    expect(screen.queryByRole('button', { name: '核对原请求' })).not.toBeInTheDocument();
  });
  it.each([null, '{broken'])('never replaces a lost original key when frozen input survives: %s', async (stored) => {
    const base = request.getMockImplementation()!;
    request.mockImplementation((path, init) => init?.method === 'POST' ? Promise.reject(new ApiNetworkError('lost')) : base(path, init));
    const view = mount(); await fill();
    fireEvent.click(screen.getByRole('button', { name: '提交充值申请' }));
    fireEvent.click(await screen.findByRole('button', { name: '确认提交' }));
    await waitFor(() => expect(screen.getByRole('button', { name: '按原请求重试' })).not.toBeDisabled());
    view.unmount();
    if (stored === null) sessionStorage.removeItem(FINANCIAL_COMMAND_STORAGE_KEY);
    else sessionStorage.setItem(FINANCIAL_COMMAND_STORAGE_KEY, stored);
    mount(); await screen.findByText('当前新申请：需管理员审核');
    fireEvent.click(screen.getByRole('button', { name: '核对原请求' }));
    expect(await screen.findByText(/原充值请求标识缺失/)).toBeInTheDocument();
    expect(writes()).toHaveLength(1);
    expect(screen.getByRole('textbox', { name: '充值金额' })).toBeDisabled();
  });
  it('does not send when storage silently drops the financial key', async () => {
    mount(); await fill();
    const original = Storage.prototype.setItem;
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(function (this: Storage, key, value) {
      if (key !== FINANCIAL_COMMAND_STORAGE_KEY) original.call(this, key, value);
    });
    fireEvent.click(screen.getByRole('button', { name: '提交充值申请' }));
    fireEvent.click(await screen.findByRole('button', { name: '确认提交' }));
    await waitFor(() => expect(screen.getAllByRole('alert').some((node) => node.textContent?.includes('请求标识未能保存'))).toBe(true));
    expect(writes()).toHaveLength(0);
  });
  it('shows own-account history with server pagination, exact amounts and Chinese modes/statuses', async () => {
    const base = request.getMockImplementation()!;
    request.mockImplementation((path, init) => path.startsWith('/agent/api/v1/recharges?')
      ? Promise.resolve({ recharges: [
        rechargeFixture,
        { ...rechargeFixture, id: 8, status: 'credited', mode: 'direct', credited_at: rechargeFixture.created_at },
        { ...rechargeFixture, id: 9, status: 'rejected', reviewed_by: 1, reviewed_at: rechargeFixture.created_at, review_reason: '核验不符' }
      ], total: 41 }) : base(path, init));
    mount('/agent/recharges');
    expect(await screen.findByText('免审直充')).toBeInTheDocument();
    expect(screen.getAllByText(rechargeFixture.amount)).toHaveLength(3);
    expect(screen.getByRole('link', { name: '选择团队用户充值' })).toHaveAttribute('href', '/agent/users');
    expect(screen.getByText(/不含下级代理账号的历史/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Next' }));
    await waitFor(() => expect(request.mock.calls.some(([path]) => path.endsWith('limit=20&offset=20'))).toBe(true));
    expect(request.mock.calls.every(([path]) => !path.includes('agent_id='))).toBe(true);
    fireEvent.click(screen.getByRole('combobox', { name: '状态' }));
    await waitFor(() => expect(document.querySelectorAll('.semi-select-option').length).toBeGreaterThan(0));
    const option = [...document.querySelectorAll('.semi-select-option')].find((item) => item.textContent === '已入账')!;
    fireEvent.mouseDown(option); fireEvent.mouseUp(option); fireEvent.click(option);
    await waitFor(() => expect(request.mock.lastCall?.[0]).toBe('/agent/api/v1/recharges?status=credited&limit=20&offset=0'));
  });
});
