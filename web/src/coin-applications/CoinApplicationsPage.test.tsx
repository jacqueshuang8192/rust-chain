import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { apiRequest, ApiNetworkError } from '../api/client';
import { authStore } from '../auth/authStore';
import { AdminAccessProvider, adminPermissionForRequest, adminReadPermissionForPath } from '../admin/access';
import { rechargeAccess } from '../recharges/testFixtures';
import { AdminCoinApplicationsPage, AgentCoinApplicationsPage } from './CoinApplicationsPage';
import { parseCoinApplication, validateCoinInput, submitCoin } from '../api/coinApplications';

vi.mock('../api/client', async () => ({ ...await vi.importActual<typeof import('../api/client')>('../api/client'), apiRequest: vi.fn() }));
const request = vi.mocked(apiRequest);
const row = { id: 7, agent_id: 2, agent_admin_id: 3, symbol: 'NOV', name: 'Nova', precision_scale: 18,
  total_supply: '100.000000000000000001', issue_price: '0.000000000000000001', quote_asset_id: 1, quote_symbol: 'USDT',
  listed_at: 1900000000000, reason: '项目发行资料', status: 'pending', project_id: null, reviewed_by: null, review_reason: null,
  created_at: 1800000000000, reviewed_at: null };
const writes = () => request.mock.calls.filter(([, init]) => init?.method === 'POST');
function mountAgent() { return render(<MemoryRouter><AgentCoinApplicationsPage /></MemoryRouter>); }
function mountAdmin(permissions = ['new_coin.applications.read', 'new_coin.applications.review']) {
  return render(<MemoryRouter><AdminAccessProvider access={rechargeAccess(permissions)}><AdminCoinApplicationsPage /></AdminAccessProvider></MemoryRouter>);
}
beforeEach(() => {
  sessionStorage.clear(); request.mockReset();
  authStore.setSession({ scope: 'agent', subject: 'agent:3', generation: 'a', accessToken: 'agent', refreshToken: 'refresh' });
  authStore.setSession({ scope: 'admin', subject: 'admin:1', generation: 'b', accessToken: 'admin', refreshToken: 'refresh' });
  request.mockImplementation(async (path, init) => {
    if (path.endsWith('/options')) return [{ asset_id: 1, symbol: 'USDT', precision_scale: 18 }];
    if (init?.method === 'POST') {
      const body = JSON.parse(String(init.body));
      if (path.endsWith('/review')) return { ...row, status: body.decision === 'approve' ? 'approved' : 'rejected', project_id: body.decision === 'approve' ? 8 : null, review_reason: body.reason, reviewed_by: 1, reviewed_at: 1800000000100 };
      return { ...row, ...body };
    }
    if (path.endsWith('/7')) return row;
    return { applications: [row], total: 1 };
  });
});
async function fill() {
  await waitFor(() => expect(request).toHaveBeenCalledWith(expect.stringContaining('/options'), expect.anything()));
  for (const [label, value] of [['币种代码', 'nov'], ['币种名称', 'Nova'], ['资产精度', '18'], ['发行总量', row.total_supply], ['申购价格', row.issue_price], ['认购初价', '0.2'], ['上市参考价', '0.5'], ['申请说明', row.reason]]) {
    fireEvent.change(screen.getByRole('textbox', { name: label }), { target: { value } });
  }
  for (const [label, value] of [['申购开始', '2030-01-01T09:00'], ['进入派发', '2030-01-01T10:00'], ['认购开始', '2030-01-01T11:00'], ['认购结束并上市', '2030-01-01T12:00']]) fireEvent.change(screen.getByLabelText(label), { target: { value } });
  fireEvent.click(screen.getByRole('combobox', { name: '计价资产' }));
  const option = await screen.findByText('USDT（18 位小数）');
  fireEvent.mouseDown(option); fireEvent.mouseUp(option); fireEvent.click(option);
}
describe('coin issuance application workflow', () => {
  it('submits exact strings, blocks double submit and restores the same intent after unknown response/remount', async () => {
    const base = request.getMockImplementation()!; let attempts = 0;
    request.mockImplementation((path, init) => init?.method === 'POST' && attempts++ === 0 ? Promise.reject(new ApiNetworkError('lost response')) : base(path, init));
    const view = mountAgent(); await fill();
    fireEvent.click(screen.getByRole('button', { name: '提交发行申请' }));
    const confirm = await screen.findByRole('button', { name: '确认提交' }); fireEvent.click(confirm); fireEvent.click(confirm);
    await screen.findByRole('button', { name: '按原申请重试' }); expect(writes()).toHaveLength(1);
    const original = JSON.parse(String(writes()[0][1]?.body));
    expect(original).toMatchObject({ symbol: 'NOV', total_supply: row.total_supply, issue_price: row.issue_price, precision_scale: 18, quote_asset_id: 1, reason: row.reason, idempotency_key: expect.any(String) });
    expect(original.issuance_policy).toMatchObject({ version: 2, purchase_initial_price: '0.2', listing_reference_price: '0.5', subscription: { automatic: false }, distribution: { automatic: false }, purchase: { automatic: false }, listing: { automatic: false }, lock: { kind: 'on_listing' }, fee: { kind: 'none' } });
    expect(original).not.toHaveProperty('agent_id'); expect(original).not.toHaveProperty('status');
    view.unmount(); mountAgent();
    fireEvent.click(await screen.findByRole('button', { name: '核对原申请' }));
    fireEvent.click(await screen.findByRole('button', { name: '按原申请重试' }));
    await screen.findByText(/申请 7 已提交/);
    expect(writes()).toHaveLength(2); expect(JSON.parse(String(writes()[1][1]?.body))).toEqual(original);
    expect(writes()[0][1]?.authScope).toBe('agent');
  });
  it('shows own history, exact detail, rejection reason and status filter without ownership parameters', async () => {
    request.mockImplementation(async (path) => path.endsWith('/options') ? [] : path.endsWith('/7')
      ? { ...row, status: 'rejected', reviewed_by: 1, reviewed_at: row.created_at, review_reason: '需补充资料' }
      : { applications: [{ ...row, status: 'rejected', reviewed_by: 1, reviewed_at: row.created_at, review_reason: '需补充资料' }], total: 1 });
    mountAgent();
    expect(screen.getByRole('tabpanel', { name: '新币发行申请' })).toBeVisible();
    fireEvent.change(screen.getByRole('textbox', { name: '币种名称' }), { target: { value: 'Draft coin' } });
    fireEvent.click(screen.getByRole('tab', { name: '我的申请记录' }));
    expect(screen.getAllByRole('tabpanel')).toHaveLength(1);
    expect(screen.getByRole('tabpanel', { name: '我的申请记录' })).toBeVisible();
    expect(await screen.findByText('需补充资料')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('tab', { name: '新币发行申请' }));
    expect(screen.getByRole('textbox', { name: '币种名称' })).toHaveValue('Draft coin');
    fireEvent.click(screen.getByRole('tab', { name: '我的申请记录' }));
    fireEvent.click(screen.getByRole('button', { name: '查看详情' }));
    expect(await screen.findByText('NOV · Nova')).toBeInTheDocument();
    expect(screen.getByText('尚未发布')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '通过并发布' })).not.toBeInTheDocument();
    expect(request.mock.calls.filter(([path]) => path.includes('coin-applications')).every(([path]) => !path.includes('agent_id'))).toBe(true);
    expect(writes()).toHaveLength(0);
  });
  it.each(['approve', 'reject'] as const)('reviews using independent permission with required reason: %s', async (decision) => {
    mountAdmin(); fireEvent.click(await screen.findByRole('button', { name: '查看详情' }));
    fireEvent.click(await screen.findByRole('button', { name: decision === 'approve' ? '通过并发布' : '驳回申请' }));
    expect(screen.getByRole('button', { name: '确认' })).toBeDisabled();
    fireEvent.change(screen.getByRole('textbox', { name: '操作原因' }), { target: { value: ' 审核资料完成 ' } });
    fireEvent.click(screen.getByRole('button', { name: '确认' }));
    await waitFor(() => expect(writes()).toHaveLength(1));
    expect(JSON.parse(String(writes()[0][1]?.body))).toEqual({ decision, reason: '审核资料完成' });
    await screen.findByText('审核资料完成');
    expect(screen.queryByRole('button', { name: '通过并发布' })).not.toBeInTheDocument();
  });
  it('keeps read-only roles from reviewing even with project write permission', async () => {
    mountAdmin(['new_coin.applications.read', 'new_coin.projects.write']);
    fireEvent.click(await screen.findByRole('button', { name: '查看详情' })); await screen.findByText('NOV · Nova');
    expect(screen.queryByRole('button', { name: '通过并发布' })).not.toBeInTheDocument();
    expect(writes()).toHaveLength(0);
    expect(adminReadPermissionForPath('/admin/agent-coin-applications')).toBe('new_coin.applications.read');
    expect(adminPermissionForRequest('/admin/api/v1/agent-coin-applications/7/review', 'POST')).toBe('new_coin.applications.review');
    expect(adminPermissionForRequest('/admin/api/v1/agent-coin-applications/7/review', 'PATCH')).toBe('admin.unmapped.write');
  });
  it('discards old-session history responses', async () => {
    let resolve!: (v: unknown) => void;
    request.mockImplementation(async (path) => path.endsWith('/options') ? [] : new Promise((r) => { resolve = r; }));
    mountAgent(); fireEvent.click(screen.getByRole('tab', { name: '我的申请记录' }));
    await waitFor(() => expect(resolve).toBeTypeOf('function'));
    const old = resolve;
    act(() => authStore.setSession({ scope: 'agent', subject: 'agent:4', generation: 'new', accessToken: 'other', refreshToken: 'other' }));
    await act(async () => old({ applications: [row], total: 1 }));
    expect(screen.queryByText('NOV')).not.toBeInTheDocument();
  });
  it('retains unknown results when a successful response belongs to another input', async () => {
    request.mockResolvedValue({ ...row, name: 'Other project' });
    await expect(submitCoin(row, 'same-key')).rejects.toThrow(/响应无效/);
  });
  it('rejects malformed state evidence and numeric money, preserving 18-place input', () => {
    expect(parseCoinApplication(row)).toEqual(row);
    for (const patch of [{ total_supply: 1 }, { reviewed_at: 1 }, { project_id: 8 }, { status: 'approved' }, { status: 'rejected', reviewed_at: 1, reviewed_by: 1 }]) {
      expect(() => parseCoinApplication({ ...row, ...patch })).toThrow();
    }
    expect(validateCoinInput(row).issue_price).toBe(row.issue_price);
    expect(() => validateCoinInput({ ...row, precision_scale: 2 })).toThrow(/精度/);
  });
});

it('preserves an uploaded logo in submission and rejects a mismatched receipt', async () => {
  const input = { ...row, logo_url: 'https://images.example.test/coin.png' };
  request.mockResolvedValueOnce(input);
  await expect(submitCoin(input, 'with-logo')).resolves.toMatchObject({ logo_url: input.logo_url });
  expect(JSON.parse(String(request.mock.calls.at(-1)?.[1]?.body)).logo_url).toBe(input.logo_url);
  request.mockResolvedValueOnce({ ...input, logo_url: null });
  await expect(submitCoin(input, 'with-logo')).rejects.toThrow('发币申请响应无效');
});
