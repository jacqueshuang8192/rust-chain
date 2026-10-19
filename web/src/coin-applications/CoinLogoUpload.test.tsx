import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { CoinLogoUpload } from './CoinLogoUpload';
import { apiRequest } from '../api/client';
import { authStore } from '../auth/authStore';

vi.mock('../api/client', async () => ({ ...await vi.importActual<typeof import('../api/client')>('../api/client'), apiRequest: vi.fn() }));
const request = vi.mocked(apiRequest);
const session = { scope: 'agent' as const, subject: 'agent:3', generation: 'logo-test', accessToken: 'access', refreshToken: 'refresh' };
const change = vi.fn(), busy = vi.fn();
function selectFile(container: HTMLElement) {
  fireEvent.change(container.querySelector('input[type="file"]')!, { target: { files: [new File(['GIF89aDATA!'], 'coin.gif', { type: 'image/gif' })] } });
}
beforeEach(() => { request.mockReset(); change.mockReset(); busy.mockReset(); authStore.setSession(session); });
afterEach(() => { authStore.clearSession('agent'); authStore.clearSession('admin'); });

it('uploads with agent authentication and only updates the logo after success', async () => {
  let resolve!: (value: unknown) => void;
  request.mockImplementation(() => new Promise(r => { resolve = r; }));
  const view = render(<CoinLogoUpload scope="agent" value="" disabled={false} onChange={change} onBusy={busy} />);
  selectFile(view.container);
  await waitFor(() => expect(request).toHaveBeenCalledOnce());
  expect(request.mock.calls[0][0]).toBe('/agent/api/v1/coin-applications/logo');
  expect(request.mock.calls[0][1]?.authScope).toBe('agent');
  expect(request.mock.calls[0][1]?.body).toBeInstanceOf(FormData);
  expect(screen.getByText('上传币种 Logo').closest('button')).toBeDisabled();
  expect(change).not.toHaveBeenCalled(); expect(busy).toHaveBeenLastCalledWith(true);
  await act(async () => resolve({ download_url: 'https://images.example.test/coin.gif' }));
  expect(change).toHaveBeenCalledWith('https://images.example.test/coin.gif');
  expect(busy).toHaveBeenLastCalledWith(false);
});
it('retains the existing image after failed upload and allows retry', async () => {
  request.mockRejectedValueOnce(new Error('upload failed'));
  const view = render(<CoinLogoUpload scope="agent" value="https://images.example.test/original.png" disabled={false} onChange={change} onBusy={busy} />);
  selectFile(view.container);
  await screen.findByRole('alert'); expect(change).not.toHaveBeenCalled();
  expect(screen.getByText('更换币种 Logo').closest('button')).toBeEnabled();
});
it('ignores a response after the authenticated session changes', async () => {
  let resolve!: (value: unknown) => void;
  request.mockImplementation(() => new Promise(r => { resolve = r; }));
  const view = render(<CoinLogoUpload scope="agent" value="" disabled={false} onChange={change} onBusy={busy} />);
  selectFile(view.container); await waitFor(() => expect(request).toHaveBeenCalledOnce());
  authStore.setSession({ ...session, generation: 'replacement' });
  await act(async () => resolve({ download_url: 'https://images.example.test/coin.gif' }));
  expect(change).not.toHaveBeenCalled();
});
it('uses the existing admin upload route and respects disabled input', async () => {
  request.mockResolvedValue({ download_url: 'https://images.example.test/admin.gif' });
  authStore.setSession({ ...session, scope: 'admin', subject: 'admin:1' });
  const view = render(<CoinLogoUpload scope="admin" value="" disabled={true} onChange={change} onBusy={busy} />);
  expect(screen.getByText('上传币种 Logo').closest('button')).toBeDisabled();
  view.rerender(<CoinLogoUpload scope="admin" value="" disabled={false} onChange={change} onBusy={busy} />);
  selectFile(view.container); await waitFor(() => expect(change).toHaveBeenCalled());
  expect(request.mock.calls[0][0]).toBe('/admin/api/v1/uploads/images');
});
