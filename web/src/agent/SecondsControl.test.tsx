import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  getAgentSecondsUserControl,
  setAgentSecondsOrderControl,
  setAgentSecondsUserControl
} from '../api/agent';
import { SecondsControlActions, UserSecondsControlPanel } from './SecondsControl';

vi.mock('../api/agent', () => ({
  getAgentSecondsUserControl: vi.fn(),
  setAgentSecondsOrderControl: vi.fn(),
  setAgentSecondsUserControl: vi.fn()
}));

const setOrderControl = vi.mocked(setAgentSecondsOrderControl);
const setUserControl = vi.mocked(setAgentSecondsUserControl);
const getUserControl = vi.mocked(getAgentSecondsUserControl);

const controlResponse = {
  scope: 'order' as const,
  order_id: 6,
  user_id: 42,
  control_result: 'loss' as const,
  actor_type: 'agent' as const,
  actor_id: 3,
  reason: '运营控制',
  control_at: 1_735_732_800_000
};

describe('SecondsControlActions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('submits the selected result without requiring a reason and refreshes the list', async () => {
    const user = userEvent.setup();
    const onDone = vi.fn();
    setOrderControl.mockResolvedValue({ ...controlResponse, reason: null });

    render(<SecondsControlActions current={null} onDone={onDone} orderId={6} />);

    await user.click(screen.getByRole('button', { name: '控输控赢 订单 6' }));
    const confirm = await screen.findByRole('button', { name: '确认控制' });
    expect(confirm).not.toBeDisabled();
    await user.click(screen.getByRole('button', { name: '控输' }));
    await user.click(confirm);

    await waitFor(() => expect(setOrderControl).toHaveBeenCalledWith(6, 'loss', ''));
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(screen.queryByLabelText('操作原因')).not.toBeInTheDocument());
  });

  it('shows the current control state and allows clearing it', async () => {
    const user = userEvent.setup();
    const onDone = vi.fn();
    setOrderControl.mockResolvedValue({ ...controlResponse, control_result: null });

    render(<SecondsControlActions current="win" onDone={onDone} orderId={6} />);

    expect(screen.getByRole('button', { name: '控输控赢 订单 6' })).toHaveTextContent('已控赢');
    await user.click(screen.getByRole('button', { name: '控输控赢 订单 6' }));
    await user.click(screen.getByRole('button', { name: '取消控制' }));
    await user.type(await screen.findByLabelText('操作原因'), '恢复自然结算');
    await user.click(await screen.findByRole('button', { name: '确认控制' }));

    await waitFor(() => expect(setOrderControl).toHaveBeenCalledWith(6, 'clear', '恢复自然结算'));
  });

  it('keeps the dialog open and shows the server error when the order is no longer open', async () => {
    const user = userEvent.setup();
    const onDone = vi.fn();
    setOrderControl.mockRejectedValue(new Error('订单已结算'));

    render(<SecondsControlActions current={null} onDone={onDone} orderId={6} />);

    await user.click(screen.getByRole('button', { name: '控输控赢 订单 6' }));
    await user.type(await screen.findByLabelText('操作原因'), '运营控制');
    await user.click(await screen.findByRole('button', { name: '确认控制' }));

    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('订单已结算'));
    expect(onDone).not.toHaveBeenCalled();
    expect(screen.getByLabelText('操作原因')).toHaveValue('运营控制');
  });

  it('disables the entry for settled orders', () => {
    render(<SecondsControlActions current={null} disabled onDone={vi.fn()} orderId={6} />);
    expect(screen.getByRole('button', { name: '控输控赢 订单 6' })).toBeDisabled();
  });
});

describe('UserSecondsControlPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('loads the current user control and reflects a successful update', async () => {
    const user = userEvent.setup();
    const onChanged = vi.fn();
    getUserControl.mockResolvedValue({
      user_id: 42, active: false, control_result: null, actor_type: null, actor_id: null, reason: null, control_at: null
    });
    setUserControl.mockResolvedValue({ ...controlResponse, scope: 'user', order_id: null, control_result: 'loss' });

    render(<UserSecondsControlPanel onChanged={onChanged} userId={42} />);

    expect(await screen.findByText('未设置')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '设置用户级控输控赢' }));
    await user.click(screen.getByRole('button', { name: '控输' }));
    await user.type(await screen.findByLabelText('操作原因'), '默认控输');
    await user.click(await screen.findByRole('button', { name: '确认控制' }));

    await waitFor(() => expect(setUserControl).toHaveBeenCalledWith(42, 'loss', '默认控输'));
    expect(await screen.findByText('已控输')).toBeInTheDocument();
    expect(onChanged).toHaveBeenCalled();
  });

  it('surfaces a load failure without sending requests', async () => {
    getUserControl.mockRejectedValue(new Error('加载失败'));
    render(<UserSecondsControlPanel onChanged={vi.fn()} userId={42} />);
    expect(await screen.findByText('控制状态加载失败')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '设置用户级控输控赢' })).toBeDisabled();
  });
});
