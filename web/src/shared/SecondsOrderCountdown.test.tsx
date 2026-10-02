import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { SecondsOrderCountdown } from './SecondsOrderCountdown';

describe('shared seconds order countdown', () => {
  afterEach(() => vi.useRealTimers());

  it('cleans up its interval and focus listener when unmounted', () => {
    vi.useFakeTimers();
    const remove = vi.spyOn(window, 'removeEventListener');
    const view = render(<SecondsOrderCountdown orderId={1} expiresAt={Date.now() + 60_000} />);
    expect(vi.getTimerCount()).toBe(1);
    view.unmount();
    expect(vi.getTimerCount()).toBe(0);
    expect(remove).toHaveBeenCalledWith('focus', expect.any(Function));
    remove.mockRestore();
  });

  it('resynchronizes after a clock jump and never asserts settlement', () => {
    vi.useFakeTimers();
    const start = Date.now();
    render(<SecondsOrderCountdown orderId={2} expiresAt={start + 60_000} />);
    act(() => vi.setSystemTime(start + 70_000));
    fireEvent.focus(window);
    expect(screen.getByRole('timer')).toHaveTextContent('待结算');
    expect(vi.getTimerCount()).toBe(0);
  });
});
