import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { apiRequest, ApiError, ApiNetworkError } from '../../api/client';
import { authStore } from '../../auth/authStore';
import { AdminAccessProvider, adminPermissionForRequest, adminReadPermissionForPath } from '../access';
import { RechargeReviewPage } from './RechargeReviewPage';
import { RechargePolicyAction } from './RechargePolicyAction';
import { rechargeAccess, rechargeFixture } from '../../recharges/testFixtures';
import { adminNavItems } from '../navigation';
import { adminRoutes } from '../routes';

vi.mock('../../api/client', async () => ({ ...await vi.importActual<typeof import('../../api/client')>('../../api/client'), apiRequest: vi.fn() }));
const request = vi.mocked(apiRequest);
const mutations = () => request.mock.calls.filter(([, init]) => init?.method === 'POST' || init?.method === 'PATCH');
function mountReview(permissions = ['agents.recharges.read', 'agents.recharges.review']) {
  return render(<AdminAccessProvider access={rechargeAccess(permissions)}><RechargeReviewPage /></AdminAccessProvider>);
}
function mountPolicy(permissions = ['agents.read', 'agents.recharges.authorize']) {
  return render(<AdminAccessProvider access={rechargeAccess(permissions)}><RechargePolicyAction agentId="2" /></AdminAccessProvider>);
}
beforeEach(() => {
  sessionStorage.clear();
  authStore.setSession({ scope: 'admin', subject: 'admin:1', generation: 'admin-session', accessToken: 'admin', refreshToken: 'refresh' });
  request.mockReset();
  request.mockImplementation(async (path, init) => {
    if (path.includes('/recharge-policy')) return { agent_id: 2, direct_recharge_enabled: false, version: 4 };
    if (init?.method === 'POST') {
      const input = JSON.parse(String(init.body));
      return { ...rechargeFixture, status: input.decision === 'approve' ? 'credited' : 'rejected',
        review_reason: input.reason, reviewed_by: 1, reviewed_at: rechargeFixture.created_at,
        credited_at: input.decision === 'approve' ? rechargeFixture.created_at : null };
    }
    return { recharges: [rechargeFixture], total: 1 };
  });
});

describe('admin recharge permissions and workflow', () => {
  it('maps exact independent queue, review, authorization and policy-read permissions', () => {
    expect(adminReadPermissionForPath('/admin/agent-recharges')).toBe('agents.recharges.read');
    expect(adminPermissionForRequest('/admin/api/v1/agent-recharges?status=pending', 'GET')).toBe('agents.recharges.read');
    expect(adminPermissionForRequest('/admin/api/v1/agent-recharges/7', 'GET')).toBe('agents.recharges.read');
    expect(adminPermissionForRequest('/admin/api/v1/agent-recharges/9007199254740992', 'GET')).toBe('admin.unmapped.read');
    expect(adminPermissionForRequest('/admin/api/v1/agent-recharges/7/review', 'POST')).toBe('agents.recharges.review');
    expect(adminPermissionForRequest('/admin/api/v1/agents/2/recharge-policy', 'PATCH')).toBe('agents.recharges.authorize');
    expect(adminPermissionForRequest('/admin/api/v1/agents/2/recharge-policy', 'GET')).toBe('agents.read');
    for (const path of ['/agent-recharges/7/review/extra', '/agents/2/recharge-policy/extra', '/agents/9007199254740992/recharge-policy']) {
      expect(adminPermissionForRequest(`/admin/api/v1${path}`, 'PATCH')).toBe('admin.unmapped.write');
    }
    expect(adminPermissionForRequest('/admin/api/v1/agent-recharges/7/review', 'PATCH')).toBe('admin.unmapped.write');
    expect(adminRoutes.find((route) => route.path === 'agent-recharges')?.handle.permission).toBe('agents.recharges.read');
    expect(adminNavItems.flatMap((item) => item.children ?? []).find((item) => item.path === '/admin/agent-recharges')?.label).toBe('代理充值审核');
  });
  it('keeps read-only queues readable and omits mutations', async () => {
    mountReview(['agents.recharges.read', 'agents.write', 'agents.recharges.authorize']);
    expect(await screen.findByText(rechargeFixture.amount)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '审核' })).not.toBeInTheDocument();
    expect(screen.getByText(/当前角色仅可查看充值申请/)).toBeInTheDocument();
    expect(mutations()).toHaveLength(0);
  });
  it.each(['approve', 'reject'] as const)('replays the immutable %s decision and reason after remount without adding an API key', async (decision) => {
    const base = request.getMockImplementation()!;
    let attempts = 0;
    request.mockImplementation((path, init) => init?.method === 'POST' && attempts++ === 0
      ? Promise.reject(new ApiNetworkError('lost response')) : base(path, init));
    const view = mountReview();
    fireEvent.click(await screen.findByRole('button', { name: '审核' }));
    if (decision === 'reject') {
      fireEvent.click(screen.getByRole('combobox', { name: '审核决定' }));
      const option = await screen.findByText('拒绝申请');
      fireEvent.mouseDown(option); fireEvent.mouseUp(option); fireEvent.click(option);
    }
    fireEvent.change(screen.getByRole('textbox', { name: '审核原因' }), { target: { value: ' 核验凭证 ' } });
    const submit = screen.getByRole('button', { name: '确认审核' });
    fireEvent.click(submit); fireEvent.click(submit);
    await screen.findByRole('button', { name: '按原决定重试' });
    expect(mutations()).toHaveLength(1);
    view.unmount();
    mountReview();
    expect(await screen.findByRole('textbox', { name: '审核原因' })).toHaveValue('核验凭证');
    expect(screen.getByRole('textbox', { name: '审核原因' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: '按原决定重试' }));
    await waitFor(() => expect(mutations()).toHaveLength(2));
    expect(mutations()[0][1]?.body).toBe(mutations()[1][1]?.body);
    expect(JSON.parse(String(mutations()[1][1]?.body))).toEqual({ decision, reason: '核验凭证' });
    expect(await screen.findByText(`申请 7：${decision === 'approve' ? '已入账' : '已拒绝'}`)).toBeInTheDocument();
  });
  it('keeps a conflicting policy draft and its original version until explicit reload', async () => {
    const base = request.getMockImplementation()!;
    let version = 4;
    request.mockImplementation((path, init) => init?.method === 'PATCH'
      ? Promise.reject(new ApiError(409, 'CONFLICT', 'changed')) : path.includes('/recharge-policy')
        ? Promise.resolve({ agent_id: 2, direct_recharge_enabled: false, version }) : base(path, init));
    mountPolicy();
    expect(request).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: '充值授权' }));
    expect(await screen.findByText(/版本：4/)).toBeInTheDocument();
    expect(screen.getByText(/不向下级继承/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole('switch', { name: '允许免审直充' }));
    fireEvent.change(screen.getByRole('textbox', { name: '授权变更原因' }), { target: { value: '免审评估通过' } });
    fireEvent.click(screen.getByRole('button', { name: '保存授权' }));
    expect(await screen.findByText(/授权已被他人修改，输入已保留/)).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: '允许免审直充' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('textbox', { name: '授权变更原因' })).toHaveValue('免审评估通过');
    expect(screen.getByRole('button', { name: '保存授权' })).toBeDisabled();
    expect(JSON.parse(String(mutations()[0][1]?.body))).toEqual({ direct_recharge_enabled: true, expected_version: 4, reason: '免审评估通过' });
    version = 6;
    fireEvent.click(screen.getByRole('button', { name: '放弃输入并重新读取' }));
    expect(await screen.findByText(/版本：6/)).toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: '授权变更原因' })).toHaveValue('');
  });
  it.each(['credited', 'rejected'] as const)('reconciles another administrator final %s decision after conflict', async (status) => {
    const base = request.getMockImplementation()!;
    request.mockImplementation((path, init) => {
      if (init?.method === 'POST') return Promise.reject(new ApiError(409, 'CONFLICT', 'already reviewed'));
      if (path === '/admin/api/v1/agent-recharges/7') return Promise.resolve({
        ...rechargeFixture, status, reviewed_by: 9, review_reason: '另一管理员核验',
        reviewed_at: rechargeFixture.created_at, credited_at: status === 'credited' ? rechargeFixture.created_at : null
      });
      return base(path, init);
    });
    mountReview(); fireEvent.click(await screen.findByRole('button', { name: '审核' }));
    fireEvent.change(screen.getByRole('textbox', { name: '审核原因' }), { target: { value: '我的决定' } });
    fireEvent.click(screen.getByRole('button', { name: '确认审核' }));
    await waitFor(() => expect(screen.getAllByRole('button', { name: '查询最终审核结果' })[0]).not.toBeDisabled());
    fireEvent.click(screen.getAllByRole('button', { name: '查询最终审核结果' })[0]);
    expect(await screen.findByRole('status')).toHaveTextContent(`申请 7：${status === 'credited' ? '已入账' : '已拒绝'}；审核人ID：9，原因：另一管理员核验`);
    expect(screen.queryByRole('button', { name: '按原决定重试' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: '审核' })).not.toBeDisabled();
    expect(mutations()).toHaveLength(1);
  });
  it.each(['pending', 'network'] as const)('keeps immutable review when reconciliation returns %s', async (outcome) => {
    const base = request.getMockImplementation()!;
    request.mockImplementation((path, init) => {
      if (init?.method === 'POST') return Promise.reject(new ApiNetworkError('lost response'));
      if (path === '/admin/api/v1/agent-recharges/7') return outcome === 'pending'
        ? Promise.resolve(rechargeFixture) : Promise.reject(new ApiNetworkError('query failed'));
      return base(path, init);
    });
    mountReview(); fireEvent.click(await screen.findByRole('button', { name: '审核' }));
    fireEvent.change(screen.getByRole('textbox', { name: '审核原因' }), { target: { value: '我的决定' } });
    fireEvent.click(screen.getByRole('button', { name: '确认审核' }));
    await waitFor(() => expect(screen.getAllByRole('button', { name: '查询最终审核结果' })[0]).not.toBeDisabled());
    fireEvent.click(screen.getAllByRole('button', { name: '查询最终审核结果' })[0]);
    await waitFor(() => expect(request.mock.calls.some(([path]) => path === '/admin/api/v1/agent-recharges/7')).toBe(true));
    expect(screen.getByRole('textbox', { name: '审核原因' })).toHaveValue('我的决定');
    expect(screen.getByRole('textbox', { name: '审核原因' })).toBeDisabled();
    expect(screen.getByRole('button', { name: '审核' })).toBeDisabled();
    expect(mutations()).toHaveLength(1);
  });
  it('allows policy read with agents.read but never infers authorization from agents.write', async () => {
    mountPolicy(['agents.read', 'agents.write', 'agents.recharges.review']);
    fireEvent.click(screen.getByRole('button', { name: '充值授权' }));
    expect(await screen.findByText(/版本：4/)).toBeInTheDocument();
    expect(screen.queryByRole('switch')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '保存授权' })).not.toBeInTheDocument();
    expect(mutations()).toHaveLength(0);
  });
  it('does not request policy with authorization-only permission', () => {
    mountPolicy(['agents.recharges.authorize']);
    expect(screen.queryByRole('button', { name: '充值授权' })).not.toBeInTheDocument();
    expect(request).not.toHaveBeenCalled();
  });
  it('does not reuse a prior administrator review after session replacement', async () => {
    const base = request.getMockImplementation()!;
    request.mockImplementation((path, init) => init?.method === 'POST' ? Promise.reject(new ApiNetworkError('lost')) : base(path, init));
    mountReview(); fireEvent.click(await screen.findByRole('button', { name: '审核' }));
    fireEvent.change(screen.getByRole('textbox', { name: '审核原因' }), { target: { value: '核验' } });
    fireEvent.click(screen.getByRole('button', { name: '确认审核' }));
    await screen.findByRole('button', { name: '按原决定重试' });
    act(() => { authStore.setSession({ scope: 'admin', subject: 'admin:2', accessToken: 'new', refreshToken: 'new' }); });
    await waitFor(() => expect(screen.queryByRole('button', { name: '按原决定重试' })).not.toBeInTheDocument());
  });
});
