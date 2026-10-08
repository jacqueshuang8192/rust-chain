import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ApiError, apiRequest } from '../../api/client';
import { AdminAccessProvider, adminPermissionForRequest, adminReadPermissionForPath } from '../access';
import { HomeCarouselPage } from './HomeCarouselPage';
import { parseCarouselConfig, saveCarouselConfig, validateCarouselForm } from './homeCarouselModel';

vi.mock('../../api/client', async () => ({
  ...await vi.importActual<typeof import('../../api/client')>('../../api/client'),
  apiRequest: vi.fn()
}));
vi.mock('../../shared/AdminImageUpload', () => ({
  AdminImageUpload: ({ label, onChange, value }: { label: string; onChange: (value: string) => void; value: string }) =>
    <input aria-label={label} value={value} onChange={(event) => onChange(event.target.value)} />
}));

const api = vi.mocked(apiRequest);
const slide = (id = 'invite', position = 0) => ({
  id, position, enabled: true, locale: null, title: `标题 ${id}`, description: '描述',
  image_url: 'https://cdn.example.test/invite.png', target: { type: 'referrals', id: null }
});
const config = () => ({ revision: 4, slides: [slide(), slide('news', 1)] });

function mount(canWrite = true) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } } });
  const router = createMemoryRouter([{ path: '/', element: <HomeCarouselPage /> }]);
  return render(<AdminAccessProvider access={{
    admin_id: 1, username: 'editor', role_id: 1, role_name: '编辑', is_super_admin: false,
    permissions: ['content.news.read', ...(canWrite ? ['content.news.write'] : [])]
  }}><QueryClientProvider client={client}><RouterProvider router={router} /></QueryClientProvider></AdminAccessProvider>);
}

beforeEach(() => {
  api.mockReset();
  api.mockImplementation((_path, init) => {
    if (init?.method === 'PATCH') {
      const body = JSON.parse(String(init.body));
      return Promise.resolve({ revision: body.expected_revision + 1, slides: body.slides });
    }
    return Promise.resolve(config());
  });
});

describe('home carousel editor', () => {
  it('maps read and write permissions independently', () => {
    expect(adminReadPermissionForPath('/admin/home/carousel')).toBe('content.news.read');
    expect(adminPermissionForRequest('/admin/api/v1/home/carousel', 'GET')).toBe('content.news.read');
    expect(adminPermissionForRequest('/admin/api/v1/home/carousel', 'PATCH')).toBe('content.news.write');
  });

  it('edits, reorders, disables and saves the draft revision with an audit reason', async () => {
    const user = userEvent.setup();
    mount();
    fireEvent.change(await screen.findByLabelText('第 1 项标题'), { target: { value: '邀请好友' } });
    fireEvent.change(screen.getByLabelText('第 1 项轮播图片'), { target: { value: 'https://cdn.example.test/new.png' } });
    await user.click(screen.getByRole('switch', { name: '第 1 项启用' }));
    await user.click(screen.getByRole('button', { name: '下移第 1 项' }));
    await user.click(screen.getByRole('button', { name: '保存首页轮播' }));
    expect(await screen.findByRole('button', { name: '确认保存' })).toBeDisabled();
    await user.type(screen.getByLabelText('操作原因'), '首页内容更新');
    await user.click(screen.getByRole('button', { name: '确认保存' }));
    await screen.findByText('首页轮播配置已保存。');
    const request = api.mock.calls.find(([, init]) => init?.method === 'PATCH')!;
    expect(request[0]).toBe('/admin/api/v1/home/carousel');
    expect(JSON.parse(String(request[1]?.body))).toEqual({
      expected_revision: 4, reason: '首页内容更新',
      slides: [
        { ...slide('news', 0) },
        { ...slide('invite', 1), enabled: false, title: '邀请好友', image_url: 'https://cdn.example.test/new.png' }
      ]
    });
    expect(screen.getByText('版本 5 · 2/12 项')).toBeInTheDocument();
  });

  it('adds and deletes items and prevents incomplete saves', async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByLabelText('第 1 项标题');
    await user.click(screen.getByRole('button', { name: '新增轮播项' }));
    expect(screen.getByLabelText('第 3 项标题')).toHaveValue('');
    await user.click(screen.getByRole('button', { name: '删除第 3 项' }));
    expect(screen.queryByLabelText('第 3 项标题')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '删除第 2 项' }));
    await user.click(screen.getByRole('button', { name: '删除第 1 项' }));
    expect(screen.getByText('暂无轮播内容')).toBeInTheDocument();
  });

  it('keeps unsaved content after a version conflict', async () => {
    api.mockImplementation((_path, init) => init?.method === 'PATCH'
      ? Promise.reject(new ApiError(409, 'CONFLICT', 'revision changed')) : Promise.resolve(config()));
    const user = userEvent.setup();
    mount();
    fireEvent.change(await screen.findByLabelText('第 1 项标题'), { target: { value: '本地草稿' } });
    await user.click(screen.getByRole('button', { name: '保存首页轮播' }));
    await user.type(await screen.findByLabelText('操作原因'), '编辑');
    await user.click(screen.getByRole('button', { name: '确认保存' }));
    await waitFor(() => expect(screen.getAllByText(/配置已被其他管理员更新/).length).toBeGreaterThan(0));
    expect(screen.getByLabelText('第 1 项标题')).toHaveValue('本地草稿');
    expect(screen.getByText('版本 4 · 2/12 项')).toBeInTheDocument();
  });

  it('keeps read-only controls disabled and hides mutations', async () => {
    mount(false);
    expect(await screen.findByLabelText('第 1 项标题')).toBeDisabled();
    expect(screen.getByLabelText('第 1 项描述')).toBeDisabled();
    expect(screen.queryByRole('button', { name: '保存首页轮播' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '新增轮播项' })).not.toBeInTheDocument();
  });

  it('rejects malformed DTOs and unsafe form values before transport', async () => {
    for (const value of [{}, { ...config(), revision: -1 }, { ...config(), slides: [slide(), slide()] }]) {
      expect(() => parseCarouselConfig(value)).toThrow();
    }
    const form = parseCarouselConfig(config());
    for (const id of ['0', '1.5', '9007199254740992', '-1']) {
      const draft = { ...form, slides: [{ ...form.slides[0], targetType: 'news_detail' as const, newsId: id }] };
      expect(validateCarouselForm(draft)).not.toHaveLength(0);
      await expect(saveCarouselConfig(draft, 'reason')).rejects.toThrow();
    }
    for (const imageUrl of ['', 'javascript:alert(1)', 'https://name:pass@example.test/a']) {
      expect(validateCarouselForm({ ...form, slides: [{ ...form.slides[0], imageUrl }] })).not.toHaveLength(0);
    }
    expect(api).not.toHaveBeenCalled();
  });
});
