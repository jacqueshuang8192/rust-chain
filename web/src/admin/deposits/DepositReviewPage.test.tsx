import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { apiRequest } from '../../api/client';
import { DepositReviewPage } from './DepositReviewPage';
import { parseOrder, parseSettings } from './contracts';

if (!Range.prototype.getBoundingClientRect) {
  Range.prototype.getBoundingClientRect = () => new DOMRect(0, 0, 0, 0);
}

vi.mock('../../api/client', async () => ({ ...await vi.importActual('../../api/client'), apiRequest: vi.fn() }));
vi.mock('../../auth/authStore', () => {
  const session = { generation: 'review-fixture', subject: '1', accessToken: 'fixture', refreshToken: '', scope: 'admin' };
  return { authStore: { subscribe: () => () => {}, getSession: () => session } };
});
vi.mock('../access', () => ({ AdminRequestActionBoundary: ({ children }: { children: React.ReactNode }) => children }));
const now = Date.now();
const receipt = { id: 8, network: 'eth', tx_hash: 'tx-proof', event_index: 1, asset_kind: 'token', token_address: '0xcontract', address: '0xreceiver', amount: '9007199254740993.000000000000000001', block_height: 100, block_hash: 'block-proof', block_time: now, confirmations: 12, status: 'review', review_reason: 'expired_unpaid_combination_reused', order_id: 71, review_note: null };
const target = { id: 71, order_no: 'CD-review-target', user_id: 29, asset_symbol: 'USDT', asset_kind: 'token', token_address: '0xcontract', network: 'eth', address: '0xreceiver', amount: '9007199254740993', fee_amount: '0.000000000000000001', paid_amount: null, status: 'review', created_at: now - 3000, expires_at: now + 1800000, required_confirmations: 12, tx_hash: null };
const mocked = vi.mocked(apiRequest);
beforeEach(() => { mocked.mockReset(); mocked.mockImplementation(async (path) => path.includes('/deposit-orders/') ? target : { items: [receipt], total: 1 }); });
afterEach(cleanup);
function show() { const client = new QueryClient({ defaultOptions: { queries: { retry: false } } }); render(<QueryClientProvider client={client}><MemoryRouter><DepositReviewPage /></MemoryRouter></QueryClientProvider>); }
it('requires an explicit target order and reason, preserving exact received amount through review', async () => {
  const user = userEvent.setup(); show();
  await user.click(await screen.findByRole('button', { name: '核对到账归属' }));
  expect(screen.getByRole('textbox', { name: '目标充值订单编号' })).toHaveValue('');
  expect(screen.getByRole('button', { name: '确认归属并入账' })).toBeDisabled();
  await user.type(screen.getByRole('textbox', { name: '目标充值订单编号' }), '71');
  await user.click(screen.getByRole('button', { name: '读取并核对目标订单' }));
  await waitFor(() => expect(screen.getByRole('button', { name: '确认归属并入账' })).toBeEnabled());
  await user.click(screen.getByRole('button', { name: '确认归属并入账' }));
  expect(screen.getByRole('button', { name: '确认' })).toBeDisabled();
  expect(screen.getByText(/用户 29，订单 CD-review-target：实际到账 9007199254740993.000000000000000001/)).toBeInTheDocument();
  await user.type(screen.getByRole('textbox', { name: '操作原因' }), '已核对付款来源与用户归属');
  await user.click(screen.getByRole('button', { name: '确认' }));
  await waitFor(() => expect(mocked).toHaveBeenCalledWith('/admin/api/v1/wallet/deposit-receipts/8/review', { method: 'POST', body: JSON.stringify({ action: 'credit', order_id: 71, reason: '已核对付款来源与用户归属' }) }));
});
it('blocks a different asset identity even when network, address and amount look similar', async () => {
  mocked.mockImplementation(async (path) => path.includes('/deposit-orders/') ? { ...target, token_address: '0xother-contract' } : { items: [receipt], total: 1 });
  const user = userEvent.setup(); show(); await user.click(await screen.findByRole('button', { name: '核对到账归属' }));
  await user.type(screen.getByRole('textbox', { name: '目标充值订单编号' }), '71');
  await user.click(screen.getByRole('button', { name: '读取并核对目标订单' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('不符合入账条件');
  expect(screen.getByRole('button', { name: '确认归属并入账' })).toBeDisabled();
});
it('rejects malformed money and incomplete configuration responses', () => {
  expect(() => parseOrder({ ...target, amount: 100 })).toThrow();
  expect(() => parseOrder({ ...target, id: Number.MAX_SAFE_INTEGER + 1 })).toThrow();
  expect(() => parseSettings({ scanners: [], assets: [] })).toThrow();
});
