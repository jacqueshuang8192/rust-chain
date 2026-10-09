import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { getAgentTeamOrders, type AgentOrderKind, type AgentTeamOrder } from '../api/agentTeamOrders';
import { authStore } from '../auth/authStore';
import { AgentTeamOrdersPage } from './TeamOrdersPage';
import { teamOrderFixtures } from '../../tests/fixtures/agent-team-orders';

vi.mock('../api/agentTeamOrders', async importOriginal => ({
  ...await importOriginal<typeof import('../api/agentTeamOrders')>(), getAgentTeamOrders: vi.fn()
}));
const getOrders = vi.mocked(getAgentTeamOrders);

function renderPage(kind = 'seconds') {
  const router = createMemoryRouter([{ path: '/agent/orders/:kind', element: <AgentTeamOrdersPage /> }],
    { initialEntries: [`/agent/orders/${kind}`] });
  return { ...render(<RouterProvider router={router} />), router };
}

describe('AgentTeamOrdersPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getOrders.mockReset();
    authStore.setSession({ scope: 'agent', subject: 'agent:1', accessToken: 'token', refreshToken: 'refresh' });
    getOrders.mockImplementation(async kind => ({ orders: [teamOrderFixtures[kind]], total: 1 }));
  });
  afterEach(() => { vi.useRealTimers(); vi.restoreAllMocks(); });

  it.each<[AgentOrderKind, string]>([
    ['seconds', '进行中'], ['margin', '待成交'], ['earn', '持有中'], ['spot', '部分成交']
  ])('shows %s user/agent identity and read-only Chinese details', async (kind, status) => {
    const user = userEvent.setup();
    renderPage(kind);
    expect(await screen.findByText(status)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'team@example.test' })).toHaveAttribute('href', '/agent/users/42/portfolio');
    expect(screen.getByText('TEAM-003')).toBeInTheDocument();
    expect(getOrders).toHaveBeenCalledWith(kind, { limit: 20, offset: 0 });
    if (kind === 'earn') expect(screen.getByText('8.00%')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /^(SC|MG|EA|SP)\d/ }));
    expect(await screen.findByText('记录ID', { selector: 'dt' })).toBeInTheDocument();
    expect(screen.getByText('归属代理编号')).toBeInTheDocument();
    expect(screen.getAllByText(status)).toHaveLength(2);
    if (kind === 'earn') {
      expect(screen.getByText('理财产品', { selector: 'dt' })).toBeInTheDocument();
      expect(screen.getByText('赎回时间', { selector: 'dt' })).toBeInTheDocument();
      expect(screen.queryByText('贷款产品')).not.toBeInTheDocument();
      expect(screen.queryByText('redeemed_at')).not.toBeInTheDocument();
    }
    expect(screen.queryByRole('button', { name: /结算|赎回|取消订单/ })).not.toBeInTheDocument();
  });

  it('submits draft filters, resets paging, and clears them on reset', async () => {
    const user = userEvent.setup();
    getOrders.mockResolvedValue({ orders: [teamOrderFixtures.seconds], total: 41 });
    renderPage();
    await screen.findByText('TEAM-003');
    await user.click(screen.getByRole('button', { name: 'Next' }));
    await waitFor(() => expect(getOrders).toHaveBeenLastCalledWith('seconds', { limit: 20, offset: 20 }));
    await screen.findByText('TEAM-003');
    await user.click(screen.getByRole('button', { name: '更多筛选' }));
    fireEvent.change(screen.getByLabelText('用户ID'), { target: { value: '42' } });
    fireEvent.change(screen.getByLabelText('代理分支ID'), { target: { value: '3' } });
    fireEvent.change(screen.getByLabelText('开始时间'), { target: { value: '2026-10-10T09:00' } });
    expect(getOrders).toHaveBeenCalledTimes(2);
    await user.click(screen.getByRole('button', { name: '查询' }));
    await waitFor(() => expect(getOrders).toHaveBeenLastCalledWith('seconds', {
      user_id: 42, agent_id: 3, created_from: new Date('2026-10-10T09:00').getTime(), limit: 20, offset: 0
    }));
    await screen.findByText('TEAM-003');
    await user.click(screen.getByRole('button', { name: '收起筛选（2）' }));
    expect(screen.queryByLabelText('代理分支ID')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '查询' }));
    await waitFor(() => expect(getOrders).toHaveBeenLastCalledWith('seconds', {
      user_id: 42, agent_id: 3, created_from: new Date('2026-10-10T09:00').getTime(), limit: 20, offset: 0
    }));
    await screen.findByText('TEAM-003');
    await user.click(screen.getByRole('button', { name: '重置' }));
    await waitFor(() => expect(getOrders).toHaveBeenLastCalledWith('seconds', { limit: 20, offset: 0 }));
    expect(screen.getByLabelText('用户ID')).toHaveValue('');
  });

  it.each<[AgentOrderKind, string, string]>([
    ['seconds', 'up', '看涨'], ['seconds', 'down', '看跌'],
    ['margin', 'long', '做多'], ['margin', 'short', '做空'],
    ['spot', 'buy', '买入'], ['spot', 'sell', '卖出']
  ])('localizes %s %s direction without changing the order snapshot', async (kind, direction, label) => {
    const field = kind === 'spot' ? 'side' : 'direction';
    const order = { ...teamOrderFixtures[kind], [field]: direction };
    getOrders.mockResolvedValue({ orders: [order], total: 1 });
    renderPage(kind);
    expect(await screen.findByText(label)).toBeInTheDocument();
    expect(screen.queryByText(direction)).not.toBeInTheDocument();
    expect(order[field]).toBe(direction);
  });

  it('uses real status and scope selects and rejects invalid ID drafts', async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByText('TEAM-003');
    for (const [label, option] of [['团队范围', '下级代理归属用户'], ['订单状态', '已退还本金']]) {
      await user.click(screen.getByRole('combobox', { name: label }));
      const target = await screen.findByText(option);
      fireEvent.mouseDown(target); fireEvent.mouseUp(target); fireEvent.click(target);
    }
    fireEvent.change(screen.getByLabelText('用户ID'), { target: { value: '1e3' } });
    await user.click(screen.getByRole('button', { name: '查询' }));
    expect(await screen.findByText('用户ID必须为正安全整数')).toBeInTheDocument();
    expect(getOrders).toHaveBeenCalledTimes(1);
    fireEvent.change(screen.getByLabelText('用户ID'), { target: { value: '42' } });
    await user.click(screen.getByRole('button', { name: '查询' }));
    await waitFor(() => expect(getOrders).toHaveBeenLastCalledWith('seconds', {
      team_scope: 'descendants', status: 'refunded', user_id: 42, limit: 20, offset: 0
    }));
  });

  it('shows loading, empty/error states and allows retry', async () => {
    const user = userEvent.setup();
    getOrders.mockRejectedValueOnce(new Error('订单接口异常')).mockResolvedValueOnce({ orders: [], total: 0 });
    renderPage();
    expect(await screen.findByRole('alert')).toHaveTextContent('订单接口异常');
    await user.click(screen.getByRole('button', { name: '刷新订单' }));
    expect(await screen.findByText('暂无数据')).toBeInTheDocument();
  });

  it('ignores an older business response after navigation', async () => {
    let resolve!: (data: { orders: AgentTeamOrder[]; total: number }) => void;
    getOrders.mockImplementationOnce(() => new Promise(done => { resolve = done; }));
    const { router } = renderPage();
    expect(screen.getByRole('status')).toHaveTextContent('加载中');
    await act(async () => router.navigate('/agent/orders/earn'));
    expect(await screen.findByText('稳健理财')).toBeInTheDocument();
    await act(async () => resolve({ orders: [teamOrderFixtures.seconds], total: 1 }));
    expect(screen.queryByText('BTC-USDT')).not.toBeInTheDocument();
    expect(screen.getByText('稳健理财')).toBeInTheDocument();
  });

  it('clears rows and details on session change and ignores previous-session responses', async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByText('TEAM-003');
    await user.click(screen.getByRole('button', { name: /^SC\d/ }));
    let resolve!: (data: { orders: AgentTeamOrder[]; total: number }) => void;
    getOrders.mockImplementationOnce(() => new Promise(done => { resolve = done; }));
    act(() => authStore.setSession({ scope: 'agent', subject: 'agent:2', accessToken: 'second', refreshToken: 'second' }));
    expect(screen.queryByText('TEAM-003')).not.toBeInTheDocument();
    expect(screen.queryByText('记录ID', { selector: 'dt' })).not.toBeInTheDocument();
    getOrders.mockResolvedValueOnce({ orders: [], total: 0 });
    act(() => authStore.setSession({ scope: 'agent', subject: 'agent:3', accessToken: 'third', refreshToken: 'third' }));
    expect(await screen.findByText('暂无数据')).toBeInTheDocument();
    await act(async () => resolve({ orders: [teamOrderFixtures.seconds], total: 1 }));
    expect(screen.queryByText('TEAM-003')).not.toBeInTheDocument();
  });

  it('countdown expiry never writes or polls and business switches clear timers', async () => {
    vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
    vi.setSystemTime(teamOrderFixtures.seconds.created_at);
    const intervals = vi.spyOn(window, 'setInterval');
    const clear = vi.spyOn(window, 'clearInterval');
    const { router, unmount } = renderPage();
    expect(await screen.findByRole('timer')).toHaveTextContent('01:00');
    await act(async () => vi.advanceTimersByTime(60_000));
    expect(screen.getByRole('timer')).toHaveTextContent('待结算');
    expect(getOrders).toHaveBeenCalledTimes(1);
    const countdownIndex = intervals.mock.calls.findIndex(call => call[1] === 1000);
    expect(countdownIndex).toBeGreaterThanOrEqual(0);
    expect(clear).toHaveBeenCalledWith(intervals.mock.results[countdownIndex].value);
    await act(async () => router.navigate('/agent/orders/earn'));
    expect(await screen.findByText('稳健理财')).toBeInTheDocument();
    unmount();
    expect(screen.queryByRole('timer')).not.toBeInTheDocument();
  });
});
