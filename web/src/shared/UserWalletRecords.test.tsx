import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { getUserWalletRecords, type UserWalletRecordPage } from '../api/userWalletRecords';
import { UserWalletRecords } from './UserWalletRecords';

vi.mock('../api/userWalletRecords', () => ({ getUserWalletRecords: vi.fn() }));
const response = (user = 42): UserWalletRecordPage => ({ total: 1, ledger: [{
  id: user, user_id: user, asset_id: 2, account_type: 'spot', asset_symbol: `ASSET${user}`,
  change_type: 'seconds_contract_settle_win', balance_type: 'available', amount: '0.000000000000000001',
  balance_after: '9007199254740993.000000000000000001', available_after: '9007199254740993.000000000000000001',
  frozen_after: '2', locked_after: '3', ref_type: 'seconds_contract', ref_id: `REF-${user}`, created_at: 1791684000000
}] });
beforeEach(() => { vi.clearAllMocks(); vi.mocked(getUserWalletRecords).mockResolvedValue(response()); });

it('shows Chinese wallet changes and business references in two read-only views', async () => {
  render(<UserWalletRecords scope="agent" userId={42} />);
  expect(await screen.findByText('ASSET42')).toBeInTheDocument();
  expect(screen.getAllByText('变动后可用').length).toBeGreaterThan(0);
  expect(screen.getAllByText('秒合约盈利结算').length).toBeGreaterThan(0);
  fireEvent.click(screen.getByText('资金流水记录'));
  expect(await screen.findByText('REF-42')).toBeInTheDocument();
  expect(getUserWalletRecords).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole('button', { name: '刷新资金记录' }));
  await waitFor(() => expect(getUserWalletRecords).toHaveBeenCalledTimes(2));
  expect(screen.queryByText('提交')).not.toBeInTheDocument();
});

it('ignores responses and errors from a previously selected user', async () => {
  let resolve!: (page: UserWalletRecordPage) => void;
  vi.mocked(getUserWalletRecords).mockImplementationOnce(() => new Promise((yes) => { resolve = yes; }));
  const view = render(<UserWalletRecords scope="admin" userId={42} />);
  vi.mocked(getUserWalletRecords).mockResolvedValue(response(43));
  view.rerender(<UserWalletRecords scope="admin" userId={43} />);
  expect(await screen.findByText('ASSET43')).toBeInTheDocument();
  await act(async () => resolve(response()));
  expect(screen.queryByText('ASSET42')).not.toBeInTheDocument();
  expect(screen.getByText('ASSET43')).toBeInTheDocument();
});

it('applies exact asset/reference filters and exposes a retryable error', async () => {
  render(<UserWalletRecords scope="agent" userId={42} />);
  await screen.findByText('ASSET42');
  fireEvent.change(screen.getByLabelText('资产ID'), { target: { value: '2' } });
  fireEvent.change(screen.getByLabelText('来源编号'), { target: { value: 'REF-42' } });
  vi.mocked(getUserWalletRecords).mockRejectedValueOnce(new Error('资金记录暂时不可用'));
  fireEvent.click(screen.getByRole('button', { name: /查询/ }));
  expect(await screen.findByText(/资金记录暂时不可用/)).toBeInTheDocument();
  expect(getUserWalletRecords).toHaveBeenLastCalledWith('agent', 42, { account_type: 'spot', asset_id: '2', ref_id: 'REF-42' }, 20, 0, expect.any(AbortSignal));
  fireEvent.click(screen.getByRole('button', { name: '刷新资金记录' }));
  expect(await screen.findByText('ASSET42')).toBeInTheDocument();
});
