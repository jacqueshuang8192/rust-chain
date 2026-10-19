import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import { beforeEach, expect, it, vi } from 'vitest';
import { ApiError, apiRequest } from '../../api/client';
import { AdminAccessProvider } from '../access';
import { NewCoinMarketSetup } from './NewCoinMarketSetup';
import { center, tokenCenter } from './__tests__/fixtures';
import { marketDraft, marketLive, marketPair, marketPreview } from './__tests__/marketFixtures';
import type { MarketDraft } from './marketSetupModel';

vi.mock('../../api/client', async () => ({ ...await vi.importActual<typeof import('../../api/client')>('../../api/client'), apiRequest: vi.fn() }));
const request = vi.mocked(apiRequest);
const endpoint = '/admin/api/v1/new-coins/7/market-setup';

function mount(saved: MarketDraft | null = marketDraft(), permissions = ['*'], saveError?: Error, initialLive = marketLive(), project = center().project) {
  let draft = saved;
  const live = initialLive;
  request.mockImplementation(async (path, options) => {
    if (path === endpoint && options?.method === 'PATCH') {
      if (saveError) throw saveError;
      const body = JSON.parse(String(options.body));
      draft = { ...marketDraft(), pair_id: body.pair_id, revision: (draft?.revision ?? 0) + 1,
        initial_price: body.initial_price, config: body.config, base_version: body.expected_market_version };
      return draft;
    }
    if (path === endpoint) return { project_id: 7, pairs: [marketPair], draft };
    if (path.endsWith('/preview')) return marketPreview();
    if (path.endsWith('/publish')) {
      if (draft) draft = { ...draft, published_version: 1 };
      live.version = 1; live.enabled = true; live.configured = true;
      return { revision: draft?.revision, published_version: 1, replayed: false };
    }
    if (path.endsWith('/default-generator')) return live;
    throw new Error(`Unexpected request ${path}`);
  });
  const dirty = vi.fn();
  const router = createMemoryRouter([{ path: '/', element: <NewCoinMarketSetup project={project} onDirty={dirty} /> }, { path: '/other', element: <p>Other</p> }]);
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
    <AdminAccessProvider access={{ admin_id: 1, username: 'operator', role_id: 1, role_name: 'operator', permissions, is_super_admin: false }}>
      <RouterProvider router={router} />
    </AdminAccessProvider>
  </QueryClientProvider>);
  return { dirty, live, router };
}

async function confirm(name: string) {
  await userEvent.click(screen.getByRole('button', { name }));
  await userEvent.type(screen.getByLabelText('操作原因'), 'reviewed configuration');
  await userEvent.click(screen.getByRole('button', { name: '确认' }));
}
async function parameters() {
  await screen.findByText('选择行情模式');
  await userEvent.click(screen.getByRole('button', { name: '下一步' }));
  await screen.findByLabelText('初始价格');
}
beforeEach(() => { request.mockReset(); });

it('saves exact draft independently and requires preview before explicit publication', async () => {
  mount(); await parameters();
  fireEvent.change(screen.getByLabelText('初始价格'), { target: { value: '9007199254740993.000000000000000001' } });
  await confirm('保存草稿');
  await screen.findByText('草稿已保存，线上行情未改变');
  const save = request.mock.calls.find(([path, options]) => path === endpoint && options?.method === 'PATCH');
  expect(JSON.parse(String(save?.[1]?.body))).toMatchObject({ initial_price: '9007199254740993.000000000000000001', expected_revision: 1, expected_market_version: 0 });
  expect(request.mock.calls.some(([path]) => path.endsWith('/publish'))).toBe(false);
  await userEvent.click(screen.getByRole('button', { name: '下一步' }));
  expect(screen.getByRole('button', { name: '确认发布行情' })).toBeDisabled();
  await userEvent.click(screen.getByRole('button', { name: '生成行情预览' }));
  await screen.findByRole('img', { name: '预览 K 线（开高低收）' });
  expect(screen.getByRole('button', { name: '确认发布行情' })).toBeEnabled();
  await confirm('确认发布行情');
  await screen.findByText('已发布 V1，实际运行状态请刷新核对');
  expect(request.mock.calls.filter(([path]) => path.endsWith('/publish'))).toHaveLength(1);
  expect(screen.getByText('等待生成器接管')).toBeInTheDocument();
});

it('invalidates previews on edits and rejects overprecision without silently rounding', async () => {
  mount(); await parameters();
  await userEvent.click(screen.getByRole('button', { name: '下一步' }));
  await userEvent.click(screen.getByRole('button', { name: '生成行情预览' }));
  await screen.findByRole('img', { name: '预览 K 线（开高低收）' });
  await userEvent.click(screen.getByRole('button', { name: '上一步' }));
  fireEvent.change(screen.getByLabelText('初始价格'), { target: { value: '0.0000000000000000001' } });
  expect(screen.getByRole('button', { name: '保存草稿' })).toBeDisabled();
  expect(screen.queryByRole('img', { name: '预览 K 线（开高低收）' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: '下一步' })).toBeDisabled();
});

it('retains a conflicted draft and reason without upgrading its base version', async () => {
  mount(marketDraft(), ['*'], new ApiError(409, 'CONFLICT', 'version changed')); await parameters();
  fireEvent.change(screen.getByLabelText('初始价格'), { target: { value: '2.000000000000000001' } });
  await confirm('保存草稿');
  await screen.findByText('草稿或线上版本已变化，当前草稿不会覆盖新配置。');
  expect(screen.getByLabelText('初始价格')).toHaveValue('2.000000000000000001');
  expect(screen.getByLabelText('操作原因')).toHaveValue('reviewed configuration');
});

it('hides writes for read-only operators and does not fetch unauthorized market data', async () => {
  mount(marketDraft(), ['new_coin.projects.read']);
  expect(screen.getByText('缺少行情读取权限。')).toBeInTheDocument();
  expect(request).not.toHaveBeenCalled();
});

it('requires querying persisted state after an unknown save result', async () => {
  mount(marketDraft(), ['*'], new Error('connection lost')); await parameters();
  fireEvent.change(screen.getByLabelText('初始价格'), { target: { value: '3' } });
  await confirm('保存草稿');
  await screen.findByRole('button', { name: '查询最新草稿与发布状态' });
  expect(screen.getByRole('button', { name: '保存草稿' })).toBeDisabled();
});

it('loads current live parameters after the saved draft has already been published', async () => {
  const live = marketLive();
  live.version = 3; live.configured = true; live.enabled = true;
  live.initial_price = '8.000000000000000001';
  mount({ ...marketDraft(), published_version: 1 }, ['*'], undefined, live);
  await parameters();
  expect(screen.getByLabelText('初始价格')).toHaveValue(live.initial_price);
});

it('allows explicit conflict recovery without silently discarding local input', async () => {
  mount(marketDraft(), ['*'], new ApiError(409, 'CONFLICT', 'version changed'));
  await parameters();
  fireEvent.change(screen.getByLabelText('初始价格'), { target: { value: '2' } });
  await confirm('保存草稿');
  await screen.findByText('草稿或线上版本已变化，当前草稿不会覆盖新配置。');
  await userEvent.click(screen.getByRole('button', { name: '取消' }));
  await confirm('放弃编辑并加载最新草稿');
  await parameters();
  expect(screen.getByLabelText('初始价格')).toHaveValue(marketDraft().initial_price);
});

it('keeps a read-only operator on read-only controls', async () => {
  mount(marketDraft(), ['new_coin.projects.read', 'market.pairs.read']);
  await parameters();
  expect(screen.getByLabelText('初始价格')).toBeDisabled();
  expect(screen.queryByRole('button', { name: '保存草稿' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: '暂停全部行情' })).not.toBeInTheDocument();
});


it('pins v2 pair and repairs an old draft seed while preventing premature publication',async()=>{
  const project=tokenCenter('preheat').project;
  mount(marketDraft(),['*'],undefined,marketLive(),project);
  await parameters();
  expect(screen.getByLabelText('初始价格')).toHaveValue('0.3');
  expect(screen.getByLabelText('初始价格')).toBeDisabled();
  await confirm('保存草稿');
  const call=request.mock.calls.find(([,options])=>options?.method==='PATCH');
  expect(JSON.parse(String(call?.[1]?.body))).toMatchObject({pair_id:11,initial_price:'0.3'});
  expect(request.mock.calls.some(([path])=>path.endsWith('/publish'))).toBe(false);
});
