import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, expect, it, vi } from 'vitest';
import { ApiError, apiRequest } from '../../api/client';
import { AdminAccessProvider, AdminPermissionBoundary } from '../access';
import { TokenHoldingsPage, TokenBuybacksPage, TokenOperationsPage, tokenRecordConfigs } from './TokenRecordsPage';
import { tokenCenter } from './__tests__/fixtures';
import { buildAdminResourceRowContract, toCsv } from '../resources/AdminResourcePage';

vi.mock('../../api/client', async () => ({ ...await vi.importActual<typeof import('../../api/client')>('../../api/client'), apiRequest: vi.fn() }));
const request = vi.mocked(apiRequest);
const identity = { project_id: 7, symbol: 'HIP', quote_symbol: 'USDT', user_id: 42, email: 'buyer@example.test', owner_agent_id: 9, owner_agent_code: 'BUYER-B', issuer_agent_id: 8 };
const holding = { ...identity, id: 801, source_kind: 'subscription', source_id: 77, quantity: '2500', cost: '500', policy_json: tokenCenter().project.issuance_policy, status: 'held', acquired_at: 1800000000000, unlock_at: null, sold_at: null };
const buyback = { ...identity, id: 'trade-901', holding_id: 801, quantity: '2500', price: '0.400000000000000001', cost: '500', gross: '1000', fee: '100', net: '900', status: 'executed', source: 'default', source_version: 'default:2', observed_at: 1800002000000, executed_at: 1800002001000 };
function mount(kind: 'holdings' | 'buybacks' | 'operations', permissions = ['*']) {
  const Component = { holdings: TokenHoldingsPage, buybacks: TokenBuybacksPage, operations: TokenOperationsPage }[kind];
  const permission = { holdings: 'new_coin.locks.read', buybacks: 'new_coin.unlocks.read', operations: 'audit.logs.read' }[kind];
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><MemoryRouter initialEntries={[`/admin/new-coins/${kind}?project_id=7&agent_id=9&unknown=ignored`]}>
    <AdminAccessProvider access={{ admin_id: 1, username: 'reader', role_id: 1, role_name: 'reader', is_super_admin: false, permissions }}><AdminPermissionBoundary permission={permission}><Component /></AdminPermissionBoundary></AdminAccessProvider>
  </MemoryRouter></QueryClientProvider>);
}
beforeEach(() => request.mockReset());
it('shows original lock rules and distinct buyer ownership without requesting settlement', async () => {
  request.mockResolvedValue({ holdings: [holding], total: 1 }); mount('holdings');
  await screen.findByText('buyer@example.test'); expect(screen.getByText('BUYER-B')).toBeInTheDocument();
  expect(request.mock.calls[0][0]).toContain('project_id=7'); expect(request.mock.calls[0][0]).toContain('agent_id=9'); expect(request.mock.calls[0][0]).not.toContain('unknown');
  await userEvent.click(screen.getByRole('button', { name: '查看持仓 801 原始规则' }));
  await screen.findByText('持仓 801 · 原始锁仓与费用'); expect(screen.getByText(/不会自动卖出/)).toBeInTheDocument();
  expect(request.mock.calls.every(([,options]) => !options?.method || options.method === 'GET')).toBe(true);
});
it('preserves exact buyback decimals in the table contract and CSV', async () => {
  request.mockResolvedValue({ buybacks: [buyback], total: 1 }); mount('buybacks');
  await screen.findByText('trade-901');
  for (const label of ['卖出总额','扣除手续费','实际到账','结算币种']) expect(screen.getByText(label)).toBeInTheDocument();
  const contract = buildAdminResourceRowContract(tokenRecordConfigs.buybacks.columns);
  expect(contract.decimalFields).toEqual(expect.arrayContaining(['cost','gross','fee','net','price']));
  expect(toCsv(tokenRecordConfigs.buybacks.columns, [buyback])).toContain('0.400000000000000001');
  await userEvent.click(screen.getByRole('button', { name: '查看详情' }));
  await screen.findByText('回购成交 trade-901');
  for (const label of ['行情来源', '行情版本', '默认行情', 'default:2']) expect(screen.getByText(label)).toBeInTheDocument();
});
it('rejects a malformed settlement amount and permits a read retry', async () => {
  request.mockResolvedValueOnce({ buybacks: [{ ...buyback, net: 900 }], total: 1 }); mount('buybacks');
  await screen.findByText(/必须是 Decimal text/); expect(screen.queryByText('trade-901')).not.toBeInTheDocument();
  request.mockResolvedValue({ buybacks: [buyback], total: 1 });
  await userEvent.click(screen.getByRole('button', { name: /刷新/ })); await screen.findByText('trade-901');
});
it('handles empty lists and request errors without creating data', async () => {
  request.mockRejectedValueOnce(new ApiError(500, 'DATABASE_ERROR', 'database unavailable')); mount('holdings');
  await screen.findByText(/数据库服务异常/); request.mockResolvedValue({ holdings: [], total: 0 });
  await userEvent.click(screen.getByRole('button', { name: /刷新/ }));
  await waitFor(() => expect(request).toHaveBeenCalledTimes(2)); expect(screen.queryByText('buyer@example.test')).not.toBeInTheDocument();
});
it('does not mount or fetch audit records without the audit permission', () => {
  mount('operations', ['new_coin.projects.read']); expect(request).not.toHaveBeenCalled();
  expect(screen.queryByText('发行操作记录')).not.toBeInTheDocument();
});
it('keeps scheduler and user identities visible in the operation ledger', async () => {
  request.mockResolvedValue({ operations: [{ id: 1, project_id: 7, symbol: 'HIP', actor_type: 'scheduler', actor_id: null, action: 'new_coin.stage', reason: 'scheduled phase', payload_json: { target: 'purchase' }, created_at: 1800002000000 }], total: 1 });
  mount('operations', ['audit.logs.read']); await screen.findByText('scheduled phase');
  expect(screen.getAllByText('自动调度').length).toBeGreaterThan(0);expect(request.mock.calls[0][0]).not.toContain('agent_id');
});

it.each(['holdings','buybacks','operations'] as const)('fetches %s pages from the server with matching totals and scope', async kind => {
  const row = kind === 'holdings' ? holding : kind === 'buybacks' ? buyback : { id: 1, project_id: 7, symbol: 'HIP', actor_type: 'user', actor_id: 42, action: 'new_coin.trade', reason: 'confirmed', payload_json: {}, created_at: 1800002000000 };
  request.mockResolvedValueOnce({ [kind]: [row], total: 51 }).mockResolvedValueOnce({ [kind]: [], total: 51 });
  mount(kind);
  await waitFor(() => expect(screen.getByRole('button', { name: 'Next' })).toBeEnabled());
  expect(request.mock.calls[0][0]).toContain('limit=50'); expect(request.mock.calls[0][0]).toContain('offset=0');
  expect(screen.getByRole('button', { name: '导出已加载数据' })).toBeEnabled();
  await userEvent.click(screen.getByRole('button', { name: 'Next' }));
  await waitFor(() => expect(request.mock.calls.some(([path]) => path.includes('offset=50') && path.includes('project_id=7'))).toBe(true));
  await waitFor(() => expect(screen.getByRole('button', { name: '导出已加载数据' })).toBeDisabled());
});
