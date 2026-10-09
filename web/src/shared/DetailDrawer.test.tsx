import { render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { DetailDrawer, DetailFieldTable } from './DetailDrawer';

describe('DetailDrawer tables', () => {
  it('localizes nested strategy fields without interpreting user content as status codes', () => {
    render(<DetailFieldTable record={{ username: 'active', title: 'pending', status: 'paused', generator: { scenario: 'trend_up', seed_mode: 'fixed', seed: 'active', volume_shape: 'end_spike' }, unknown_field: 'unknown_code' }} />);
    expect(screen.getByText('用户名')).toBeInTheDocument();
    expect(screen.getAllByText('active', { exact: true })).toHaveLength(2);
    expect(screen.getByText('pending', { exact: true })).toBeInTheDocument();
    expect(screen.getByText('暂停')).toBeInTheDocument();
    expect(screen.getByText('生成模型')).toBeInTheDocument();
    expect(screen.getByText('行情场景')).toBeInTheDocument();
    expect(screen.getByText('稳步上涨')).toBeInTheDocument();
    expect(screen.getByText('固定')).toBeInTheDocument();
    expect(screen.getByText('尾段放量')).toBeInTheDocument();
    expect(screen.getByText('unknown_code')).toBeInTheDocument();
  });
  it('uses semantic wrapping fields instead of a fixed-width table and retains exact source values', () => {
    render(<DetailFieldTable record={{ id: 7, status: 'active', stake_amount: '9007199254740993.000000000000000001', settlement_price_version: 'redis:'.padEnd(100, 'a') }} />);
    const fields = screen.getByRole('region', { name: '详情字段' });
    expect(fields.querySelector('dl')).toBeInTheDocument();
    expect(within(fields).queryByRole('table')).not.toBeInTheDocument();
    expect(screen.getByTitle('9007199254740993.000000000000000001')).toBeInTheDocument();
    expect(screen.getByText('结算行情证据版本').closest('div')).toHaveClass('admin-detail-field--wide');
  });

  it('renders nested lists, localized evidence and empty fields without changing raw or custom values', () => {
    render(<DetailFieldTable fieldMeta={{ labels: { username: '账户名称' }, valueMaps: { status: { active: '已认证' } } }}
      record={{ username: 'active', status: 'active', settlement_price_source: 'redis', receipt: { policy_version: 2, eligible_at: null }, cycles: [{ duration_seconds: 60, payout_rate: '0.4' }], custom_key: 'raw-value', empty: [] }} />);
    expect(screen.getByText('账户名称')).toBeInTheDocument();
    expect(screen.getByText('已认证')).toBeInTheDocument();
    expect(screen.getByText('Redis 缓存报价')).toBeInTheDocument();
    expect(screen.getByText('可执行时间')).toBeInTheDocument();
    expect(screen.getByRole('list')).toBeInTheDocument();
    expect(screen.getByText('custom_key')).toBeInTheDocument();
    expect(screen.getByText('raw-value')).toBeInTheDocument();
  });

  it('adds handles to every dynamic record column in array details', () => {
    render(
      <DetailDrawer
        detail={{
          data: [{ id: 7, name: '测试记录', status: 'active' }],
          title: '动态详情'
        }}
        onClose={vi.fn()}
      />
    );

    const sheet = screen.getByText('动态详情').closest('.semi-sidesheet-inner');
    expect(sheet).toBeInTheDocument();
    expect(within(sheet as HTMLElement).getAllByRole('separator')).toHaveLength(3);
    expect(within(sheet as HTMLElement).getByRole('separator', { name: '调整ID列宽' })).toBeInTheDocument();
    expect(within(sheet as HTMLElement).getByRole('separator', { name: '调整名称列宽' })).toBeInTheDocument();
    expect(within(sheet as HTMLElement).getByRole('separator', { name: '调整状态列宽' })).toBeInTheDocument();
  });
});
