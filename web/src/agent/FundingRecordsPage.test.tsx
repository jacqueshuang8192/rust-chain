import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { beforeEach, expect, it, vi } from 'vitest';
import { fundingFixture } from '../../tests/fixtures/agent-funding';
import { getAgentFunding, type FundingRecord } from '../api/agentFunding';
import { authStore } from '../auth/authStore';
import { AgentFundingRecordsPage } from './FundingRecordsPage';
vi.mock('../api/agentFunding', async original => ({ ...await original<typeof import('../api/agentFunding')>(), getAgentFunding: vi.fn() }));
const getRecords = vi.mocked(getAgentFunding);
function show(path = '/agent/funding/deposits') {
  const router = createMemoryRouter([{ path: '/agent/funding/:kind', element: <AgentFundingRecordsPage /> }], { initialEntries: [path] });
  render(<RouterProvider router={router} />); return router;
}
beforeEach(() => {
  getRecords.mockReset(); getRecords.mockResolvedValue({ items: [fundingFixture], total: 1 });
  authStore.setSession({ scope: 'agent', subject: 'agent:1', accessToken: 'agent', refreshToken: 'a' });
});
it('shows scoped identity and read-only deposit details, separating fiat from coin amounts', async () => {
  const user = userEvent.setup();
  const quick: FundingRecord = { ...fundingFixture, source: 'quick', status: 'credited', credited_amount: fundingFixture.amount, fiat_amount: '100', fiat_currency: 'CNY' };
  getRecords.mockResolvedValue({ items: [quick], total: 1 }); show();
  expect(await screen.findByText('快速充值')).toBeInTheDocument();
  expect(screen.getByRole('link', { name: 'team@example.test' })).toHaveAttribute('href', '/agent/users/42/portfolio');
  expect(screen.getByText('TEAM-003')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: fundingFixture.order_no! }));
  expect(await screen.findByText('法币报价金额', { selector: 'dt' })).toBeInTheDocument();
  expect(screen.getByText('已入账净数量（当前状态）', { selector: 'dt' })).toBeInTheDocument();
  expect(screen.queryByText('record_key')).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: /批准|广播|入账|核对/ })).not.toBeInTheDocument();
});
it('opens user-scoped withdrawal records with the real pending-review state and no recharge actions', async () => {
  const user = userEvent.setup();
  getRecords.mockResolvedValue({ items: [{ ...fundingFixture, source: 'withdrawal', order_no: null, status: 'pending_review', total_reserved: '10.01' }], total: 1 });
  show('/agent/funding/withdrawals?user_id=42');
  expect(await screen.findByText('待审核')).toBeInTheDocument();
  expect(getRecords).toHaveBeenCalledWith('withdrawals', { user_id: 42, limit: 20, offset: 0 });
  expect(screen.queryByRole('combobox', { name: '充值方式' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: '记录 7' }));
  expect(await screen.findByText('总冻结数量', { selector: 'dt' })).toBeInTheDocument();
  expect(screen.queryByText('法币报价金额', { selector: 'dt' })).not.toBeInTheDocument();
});
it('uses real select filters, bounds IDs, resets paging and retains advanced filters when collapsed', async () => {
  const user = userEvent.setup(); getRecords.mockResolvedValue({ items: [fundingFixture], total: 42 }); show();
  await screen.findByText('TEAM-003'); await user.click(screen.getByRole('button', { name: 'Next' }));
  await waitFor(() => expect(getRecords).toHaveBeenLastCalledWith('deposits', { limit: 20, offset: 20 }));
  await screen.findByText('TEAM-003');
  await user.click(screen.getByRole('combobox', { name: '充值方式' }));
  const option = await screen.findByText('快速充值', { selector: '.semi-select-option-text' }); fireEvent.mouseDown(option); fireEvent.mouseUp(option); fireEvent.click(option);
  await user.click(screen.getByRole('button', { name: '更多筛选' }));
  fireEvent.change(screen.getByLabelText('代理分支ID'), { target: { value: '3' } });
  fireEvent.change(screen.getByLabelText('用户ID'), { target: { value: '1e3' } });
  await user.click(screen.getByRole('button', { name: '查询' }));
  expect(screen.getByText('用户ID必须为正安全整数')).toBeInTheDocument(); expect(getRecords).toHaveBeenCalledTimes(2);
  fireEvent.change(screen.getByLabelText('用户ID'), { target: { value: '42' } });
  await user.click(screen.getByRole('button', { name: '查询' }));
  await waitFor(() => expect(getRecords).toHaveBeenLastCalledWith('deposits', { source: 'quick', agent_id: 3, user_id: 42, limit: 20, offset: 0 }));
  await user.click(screen.getByRole('button', { name: '收起筛选（1）' }));
  await user.click(screen.getByRole('button', { name: '查询' }));
  await waitFor(() => expect(getRecords).toHaveBeenCalledTimes(4));
  expect(getRecords).toHaveBeenLastCalledWith('deposits', { source: 'quick', agent_id: 3, user_id: 42, limit: 20, offset: 0 });
  await screen.findByText('TEAM-003'); await user.click(screen.getByRole('button', { name: '重置' }));
  await waitFor(() => expect(getRecords).toHaveBeenLastCalledWith('deposits', { limit: 20, offset: 0 }));
});
it('hides previous rows during requests and rejects late results after kind and session changes', async () => {
  let resolveOld!: (value: { items: FundingRecord[]; total: number }) => void;
  getRecords.mockImplementationOnce(() => new Promise(resolve => { resolveOld = resolve; }));
  const router = show();
  await waitFor(() => expect(getRecords).toHaveBeenCalledTimes(1));
  getRecords.mockResolvedValue({ items: [], total: 0 });
  await act(async () => { await router.navigate('/agent/funding/withdrawals'); });
  await waitFor(() => expect(getRecords).toHaveBeenCalledWith('withdrawals', { limit: 20, offset: 0 }));
  await act(async () => { resolveOld({ items: [fundingFixture], total: 1 }); });
  expect(screen.queryByText('TEAM-003')).not.toBeInTheDocument();
  getRecords.mockImplementationOnce(() => new Promise(resolve => { resolveOld = resolve; }));
  await act(async () => { await router.navigate('/agent/funding/deposits'); });
  await waitFor(() => expect(getRecords).toHaveBeenCalledTimes(3));
  getRecords.mockResolvedValue({ items: [], total: 0 });
  await act(async () => { authStore.setSession({ scope: 'agent', subject: 'agent:2', accessToken: 'next', refreshToken: 'b' }); });
  await waitFor(() => expect(getRecords).toHaveBeenCalledTimes(4));
  await act(async () => { resolveOld({ items: [fundingFixture], total: 1 }); });
  expect(screen.queryByText('TEAM-003')).not.toBeInTheDocument();
});
it('shows failures without inventing empty success or keeping old rows', async () => {
  const user = userEvent.setup(); show(); await screen.findByText('TEAM-003');
  getRecords.mockRejectedValue(new Error('记录读取失败'));
  await user.click(screen.getByRole('button', { name: '刷新记录' }));
  expect(await screen.findByText(/加载失败：记录读取失败/)).toBeInTheDocument(); expect(screen.queryByText('TEAM-003')).not.toBeInTheDocument();
});
