import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { listAdminResource } from '../../api/adminResources';
import { resourceConfigs } from './resourceConfigs';
import { TradingResourcePage } from './TradingResourcePage';

vi.mock('../../api/adminResources', () => ({ listAdminResource: vi.fn() }));
const list = vi.mocked(listAdminResource);
const now = 1_800_000_000_000;
const secondsOrder = {
  id: 12, user_id: 42, email: 'test@example.invalid', symbol: 'BTC/USDT',
  direction: 'up', stake_amount: '1.000000000000000001', stake_asset_symbol: 'USDT',
  duration_seconds: 60, entry_price: '60000', settlement_price: null, result: null,
  status: 'opened', expires_at: now + 2000
};

function mount(kind: 'secondsOrders' | 'marginPositions') {
  return render(<MemoryRouter><TradingResourcePage config={{ ...resourceConfigs[kind], rowActions: undefined, showJsonAction: false }} /></MemoryRouter>);
}

describe('administrator active trading views', () => {
  beforeEach(() => list.mockReset());
  afterEach(() => vi.useRealTimers());

  it('defaults to opened seconds orders, counts down and keeps status after reset', async () => {
    vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
    vi.setSystemTime(now);
    list.mockResolvedValue({ rows: [secondsOrder], total: 1, raw: {} });
    const view = mount('secondsOrders');
    const timer = await screen.findByRole('timer', { name: '订单 12 倒计时' });
    expect(timer).toHaveTextContent('00:02');
    expect(list.mock.lastCall?.[2]).toEqual({ status: 'opened', limit: 50, offset: 0 });
    await act(async () => { vi.advanceTimersByTime(2000); });
    expect(timer).toHaveTextContent('待结算');
    expect(list).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole('button', { name: '重置' }));
    await waitFor(() => expect(list).toHaveBeenCalledTimes(2));
    expect(list.mock.lastCall?.[2]).toEqual({ status: 'opened', limit: 50, offset: 0 });
    expect(list.mock.lastCall?.[3]?.rowContract?.requiredFields).not.toContain('countdown');
    expect(list.mock.lastCall?.[3]?.rowContract?.requiredFields).toContain('expires_at');
    expect(screen.getByRole('tabpanel', { name: '进行中订单' })).toContainElement(await screen.findByRole('timer', { name: '订单 12 倒计时' }));
    const headers = screen.getAllByRole('columnheader').map(node => node.textContent);
    expect(headers.indexOf('倒计时')).toBeLessThan(headers.indexOf('用户邮箱'));
    fireEvent.click(screen.getByRole('button', { name: '刷新' }));
    await waitFor(() => expect(list).toHaveBeenCalledTimes(3));
    expect(list.mock.lastCall?.[2]).toEqual({ status: 'opened', limit: 50, offset: 0 });
    view.unmount();
    expect(screen.queryByRole('timer')).not.toBeInTheDocument();
  });

  it('switches to all orders without active restriction and never times settled orders', async () => {
    list.mockResolvedValue({ rows: [], total: 0, raw: {} });
    mount('secondsOrders');
    await waitFor(() => expect(list).toHaveBeenCalledTimes(1));
    list.mockResolvedValue({ rows: [{ ...secondsOrder, status: 'settled', result: 'win' }], total: 1, raw: {} });
    fireEvent.click(screen.getByRole('tab', { name: '全部订单' }));
    await waitFor(() => expect(list.mock.lastCall?.[2]).toEqual({ limit: 50, offset: 0 }));
    expect(await screen.findByText('已结算')).toBeInTheDocument();
    expect(screen.queryByRole('timer')).not.toBeInTheDocument();
  });

  it('shows open margin positions and resets pagination when switching scopes', async () => {
    list.mockResolvedValue({ rows: [{
      id: 9, user_id: 42, product_id: 3, pair_id: 7, margin_asset: 11, margin_mode: 'isolated',
      leverage: '10', direction: 'long', margin_amount: '10', notional_amount: '100',
      borrowed_amount: '90', interest_amount: '0.000000000000000001', entry_price: '60000', status: 'opened'
    }], total: 101, raw: {} });
    mount('marginPositions');
    expect(await screen.findByText('持仓中')).toBeInTheDocument();
    expect(list.mock.lastCall?.[2]).toEqual({ status: 'opened', filled_only: 'true', limit: 50, offset: 0 });
    expect(screen.getByText('杠杆倍数')).toBeInTheDocument();
    expect(screen.getByText('开仓价')).toBeInTheDocument();
    expect(screen.getByText('逐仓')).toBeInTheDocument();
    const pageTwo = [...document.querySelectorAll('.semi-page-item')].find((node) => node.textContent?.trim() === '2');
    expect(pageTwo).toBeDefined();
    fireEvent.click(pageTwo!);
    await waitFor(() => expect(list.mock.lastCall?.[2]).toEqual({ status: 'opened', filled_only: 'true', limit: 50, offset: 50 }));
    fireEvent.click(screen.getByRole('tab', { name: '全部仓位' }));
    await waitFor(() => expect(list.mock.lastCall?.[2]).toEqual({ limit: 50, offset: 0 }));
    fireEvent.click(screen.getByRole('tab', { name: '当前持仓' }));
    await waitFor(() => expect(list.mock.lastCall?.[2]).toEqual({ status: 'opened', filled_only: 'true', limit: 50, offset: 0 }));
    fireEvent.click(screen.getByRole('button', { name: '重置' }));
    await waitFor(() => expect(list).toHaveBeenCalledTimes(5));
    expect(list.mock.lastCall?.[2]).toEqual({ status: 'opened', filled_only: 'true', limit: 50, offset: 0 });
  });

  it('labels unfilled orders as pending in all margin records', async () => {
    list.mockResolvedValue({ rows: [], total: 0, raw: {} });
    mount('marginPositions');
    await waitFor(() => expect(list).toHaveBeenCalledTimes(1));
    list.mockResolvedValue({ rows: [{
      id: 10, user_id: 42, product_id: 3, pair_id: 7, margin_asset: 11, margin_mode: 'isolated',
      leverage: '10', direction: 'long', margin_amount: '10', notional_amount: '100',
      borrowed_amount: '90', interest_amount: '0', entry_price: null, status: 'opened'
    }], total: 1, raw: {} });
    fireEvent.click(screen.getByRole('tab', { name: '全部仓位' }));
    expect(await screen.findByText('待成交')).toBeInTheDocument();
    expect(screen.queryByText('持仓中')).not.toBeInTheDocument();
    expect(list.mock.lastCall?.[2]).toEqual({ limit: 50, offset: 0 });
  });
});
