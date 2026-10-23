import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { getAgentMe, type AgentMe } from '../api/agent';
import { authStore } from '../auth/authStore';
import { AgentLayout } from './AgentLayout';

vi.mock('../api/agent', () => ({ getAgentMe: vi.fn() }));
const profile: AgentMe = {
  agent_admin_id: 9, agent_id: 3, username: '华南代理', agent_code: 'AG003',
  level: 2, agent_status: 'active', admin_status: 'active'
};

function renderAgentLayout(initialEntry = '/agent/dashboard') {
  const router = createMemoryRouter(
    [
      {
        path: '/agent',
        element: <AgentLayout />,
        children: [
          { path: 'dashboard', element: <div>代理总览内容</div> },
          { path: 'support', element: <div>在线客服内容</div> },
          { path: 'users', element: <div>团队用户内容</div> },
          { path: 'orders/:kind', element: <div>团队订单内容</div> },
          { path: 'funding/:kind', element: <div>团队充提内容</div> },
          { path: 'users/:userId/portfolio', element: <div>用户资产内容</div> },
          { path: 'users/:userId/recharge', element: <div>用户充值内容</div> },
          { path: 'recharges', element: <div>我的充值申请内容</div> }
        ]
      },
      { path: '/login', element: <div>登录页</div> }
    ],
    { initialEntries: [initialEntry] }
  );

  return render(<RouterProvider router={router} />);
}

describe('AgentLayout', () => {
  beforeEach(() => {
    vi.mocked(getAgentMe).mockReset().mockResolvedValue(profile);
    authStore.setSession({ accessToken: 'admin-token', refreshToken: 'admin-refresh', scope: 'admin', subject: 'admin:1' });
    authStore.setSession({ accessToken: 'agent-token', refreshToken: 'agent-refresh', scope: 'agent', subject: 'agent:unknown' });
  });

  afterEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  it('renders the real account identity instead of the internal subject', async () => {
    renderAgentLayout();

    ['总览', '在线客服', '团队用户', '秒合约订单', '合约订单', '理财订单', '现货订单', '新币申购', '新币认购', '邀请码', '佣金记录', '闪兑统计', '团队树', '下级代理'].forEach((label) => {
      expect(screen.getByRole('menuitem', { name: label })).toBeInTheDocument();
    });
    expect(await screen.findByText('华南代理')).toBeInTheDocument();
    expect(screen.getByText('二级代理 · 代理编号 AG003')).toBeInTheDocument();
    expect(screen.queryByText('agent:unknown')).not.toBeInTheDocument();
    expect(screen.getByText('代理总览内容')).toBeInTheDocument();
  });

  it.each(['秒合约订单', '合约订单', '理财订单', '现货订单', '新币申购', '新币认购'])('opens %s from navigation', async (label) => {
    renderAgentLayout();
    fireEvent.click(screen.getByRole('menuitem', { name: label }));
    expect(await screen.findByText('团队订单内容')).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: label })).toHaveClass('semi-navigation-item-selected');
  });

  it('navigates between agent menu items', async () => {
    renderAgentLayout();

    fireEvent.click(screen.getByRole('menuitem', { name: '团队用户' }));

    expect(await screen.findByText('团队用户内容')).toBeInTheDocument();
  });

  it.each(['充值记录', '提现记录'])('opens %s and selects its navigation item', async (label) => {
    renderAgentLayout();
    fireEvent.click(screen.getByRole('menuitem', { name: label }));
    expect(await screen.findByText('团队充提内容')).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: label })).toHaveClass('semi-navigation-item-selected');
    expect(screen.getByRole('menuitem', { name: '我的充值申请' })).not.toHaveClass('semi-navigation-item-selected');
  });

  it('opens the online support workbench from the sidebar', async () => {
    renderAgentLayout();

    fireEvent.click(screen.getByRole('menuitem', { name: '在线客服' }));

    expect(await screen.findByText('在线客服内容')).toBeInTheDocument();
  });

  it('keeps the team-user navigation item selected on nested portfolio routes', async () => {
    renderAgentLayout('/agent/users/42/portfolio');
    await screen.findByText('华南代理');

    expect(screen.getByText('用户资产内容')).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: '团队用户' })).toHaveClass('semi-navigation-item-selected');
  });

  it('clears only the agent session on logout', async () => {
    renderAgentLayout();

    fireEvent.click(screen.getByRole('button', { name: /退出登录/ }));

    expect(await screen.findByText('登录页')).toBeInTheDocument();
    expect(authStore.getSession()).toMatchObject({
      accessToken: 'admin-token',
      refreshToken: 'admin-refresh',
      scope: 'admin',
      subject: 'admin:1'
    });
    expect(authStore.getSession()?.generation).toEqual(expect.any(String));
    expect(authStore.getSession('agent')).toBeNull();
  });
  it('provides a narrow-screen menu that closes on navigation and reaches recharge history', async () => {
    renderAgentLayout('/agent/users/42/recharge');
    expect(screen.getByRole('menuitem', { name: '团队用户' })).toHaveClass('semi-navigation-item-selected');
    expect(screen.getByLabelText('代理侧边栏')).toHaveAttribute('data-expanded', 'false');
    // jsdom uses desktop CSS; viewport visibility is verified in the browser.
    fireEvent.click(screen.getByLabelText('展开代理菜单'));
    expect(screen.getByLabelText('代理侧边栏')).toHaveAttribute('data-expanded', 'true');
    fireEvent.click(screen.getByRole('menuitem', { name: '我的充值申请' }));
    expect(await screen.findByText('我的充值申请内容')).toBeInTheDocument();
    expect(screen.getByLabelText('展开代理菜单')).toHaveAttribute('aria-expanded', 'false');
  });

  it('shows a neutral loading state and retries failed identity reads', async () => {
    vi.mocked(getAgentMe).mockRejectedValueOnce(new Error('offline'));
    renderAgentLayout();
    expect(screen.getByText('账号资料加载中')).toBeInTheDocument();
    expect(screen.queryByText('agent:unknown')).not.toBeInTheDocument();
    expect(await screen.findByRole('alert')).toHaveTextContent('账号资料暂不可用');
    fireEvent.click(screen.getByRole('button', { name: '重新加载代理账号资料' }));
    expect(await screen.findByText('华南代理')).toBeInTheDocument();
    expect(getAgentMe).toHaveBeenCalledTimes(2);
  });

  it('clears the previous account immediately and ignores late responses after session changes', async () => {
    let resolveOld!: (value: AgentMe) => void;
    vi.mocked(getAgentMe).mockResolvedValueOnce(profile)
      .mockImplementationOnce(() => new Promise((resolve) => { resolveOld = resolve; }))
      .mockResolvedValueOnce({ ...profile, username: '新代理', agent_code: 'AG004', level: 3 });
    renderAgentLayout();
    await screen.findByText('华南代理');
    const switchSession = (token: string) => act(() => {
      authStore.setSession({ accessToken: token, refreshToken: token, scope: 'agent', subject: 'agent:unknown' });
    });
    switchSession('second-session');
    expect(screen.queryByText('华南代理')).not.toBeInTheDocument();
    expect(screen.getByText('账号资料加载中')).toBeInTheDocument();
    const oldSignal = vi.mocked(getAgentMe).mock.calls[1][0];
    switchSession('third-session');
    expect(oldSignal?.aborted).toBe(true);
    await screen.findByText('新代理');
    await act(async () => resolveOld({ ...profile, username: '迟到的旧代理' }));
    expect(screen.queryByText('迟到的旧代理')).not.toBeInTheDocument();
    expect(screen.getByText('三级代理 · 代理编号 AG004')).toBeInTheDocument();
  });

  it('aborts identity reads when the layout unmounts', async () => {
    vi.mocked(getAgentMe).mockImplementation(() => new Promise(() => {}));
    const view = renderAgentLayout();
    await waitFor(() => expect(getAgentMe).toHaveBeenCalledOnce());
    const signal = vi.mocked(getAgentMe).mock.calls[0][0];
    view.unmount();
    expect(signal?.aborted).toBe(true);
  });
});
