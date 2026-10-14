import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { beforeEach, expect, it, vi } from 'vitest';
import { apiRequest } from '../../api/client';
import { AdminAccessProvider } from '../access';
import { RegistrationLegalPage } from './RegistrationLegalPage';

vi.mock('../../api/client', async () => ({
  ...await vi.importActual<typeof import('../../api/client')>('../../api/client'),
  apiRequest: vi.fn()
}));

const api = vi.mocked(apiRequest);
const body = 'Reviewed test-only legal document content for publication and version evidence. '.repeat(2);
let edition = 0;

function mount(canWrite = true) {
  const router = createMemoryRouter([{ path: '/', element: <RegistrationLegalPage /> }]);
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(<AdminAccessProvider access={{
    admin_id: 1, username: 'editor', role_id: 1, role_name: 'editor', is_super_admin: false,
    permissions: ['system.legal.read', ...(canWrite ? ['system.legal.write'] : [])]
  }}><QueryClientProvider client={queryClient}><RouterProvider router={router} /></QueryClientProvider></AdminAccessProvider>);
}

beforeEach(() => {
  edition = 0;
  api.mockReset();
  api.mockImplementation((path, init) => {
    if (path === '/api/v1/countries') return Promise.resolve({ countries: [{ country_code: 'US', country_name: 'United States', supported_locales: ['en'] }] });
    if (init?.method === 'POST') { edition = 1; return Promise.resolve({ publication_id: 9 }); }
    return Promise.resolve({
      current: edition ? { publication_id: 9, scope_code: 'GLOBAL', locale: 'en', edition,
        terms: { id: 10, title: 'Terms', body, effective_on: '2026-10-14' },
        privacy: { id: 11, title: 'Privacy', body, effective_on: '2026-10-14' } } : null,
      editions: edition ? [{ id: 9, edition, published_at: '2026-10-14T00:00:00Z', published_by: 1 }] : []
    });
  });
});

it('publishes terms and privacy together with expected edition and audit reason', async () => {
  const user = userEvent.setup();
  mount();
  fireEvent.change(await screen.findByLabelText('用户协议标题'), { target: { value: 'Terms' } });
  fireEvent.change(screen.getByLabelText('隐私政策标题'), { target: { value: 'Privacy' } });
  fireEvent.change(screen.getByLabelText('用户协议正文'), { target: { value: body } });
  fireEvent.change(screen.getByLabelText('隐私政策正文'), { target: { value: body } });
  fireEvent.change(screen.getByLabelText('发布原因'), { target: { value: 'Legal review approved' } });
  expect(screen.getByRole('button', { name: '发布两份文件' })).toBeEnabled();
  await user.click(screen.getByRole('button', { name: '发布两份文件' }));
  await user.click(await screen.findByRole('button', { name: '发布法律文件' }));
  await waitFor(() => expect(api.mock.calls.some(([, init]) => init?.method === 'POST')).toBe(true));
  const [path, init] = api.mock.calls.find(([, options]) => options?.method === 'POST')!;
  expect(path).toBe('/admin/api/v1/legal-documents');
  expect(JSON.parse(String(init?.body))).toMatchObject({
    scope_code: 'GLOBAL', locale: 'en', expected_edition: 0,
    terms_title: 'Terms', privacy_title: 'Privacy',
    terms_body: body, privacy_body: body, reason: 'Legal review approved'
  });
  expect(await screen.findByText('两份文件已同时发布。')).toBeInTheDocument();
});

it('disables editing for a read-only administrator', async () => {
  mount(false);
  expect(await screen.findByLabelText('用户协议标题')).toBeDisabled();
  expect(screen.getByLabelText('用户协议正文')).toBeDisabled();
  expect(screen.queryByRole('button', { name: '发布两份文件' })).not.toBeInTheDocument();
});
