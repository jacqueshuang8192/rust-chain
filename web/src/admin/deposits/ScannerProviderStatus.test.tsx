import { cleanup, render, screen, within } from '@testing-library/react';
import { afterEach, expect, it } from 'vitest';
import { parseSettings, type Scanner, type ScannerProvider } from './contracts';
import { ScannerProviderStatus } from './ScannerProviderStatus';

const checked = 1000000;
const provider: ScannerProvider = { id: 1, network: 'base', name: 'Primary', enabled: true, priority: 0, request_interval_ms: 500, cooldown_until: null, last_error: null, last_success_at: checked - 1000, stream_failures: [{ stream: 'native', cooldown_until: checked + 60000, last_error: 'native method unavailable' }], keys: [{ id: 2, provider_id: 1, name: 'Key', enabled: true, priority: 0, api_key_configured: true, request_interval_ms: 500, cooldown_until: null, last_error: null, last_success_at: checked - 1000 }] };
const progress = { recovery_mode: null, backfill_cursor_height: null, backfill_target_height: null, backfill_last_error: null, backfill_blocked: false, last_scan_at: checked - 1000, last_error: null, scan_fresh: true, cursor_height: 90 };
const scanner: Scanner = { ...progress, network: 'base', providers: [provider], streams: [{ ...progress, stream: 'native' }, { ...progress, stream: 'token' }], enabled: true, endpoint_configured: true, api_key_configured: true, confirmations: 1, revision: 1, checked_at: checked };
afterEach(cleanup);

it('shows native cooldown independently while ERC20 remains eligible using server time', () => {
  render(<ScannerProviderStatus scanner={scanner} />);
  expect(screen.getByText('部分冷却')).toBeInTheDocument();
  expect(screen.getByText('ETH 原生币').parentElement).toHaveTextContent('冷却中');
  expect(screen.getByText('ERC20 / USDC').parentElement).toHaveTextContent('可尝试');
  expect(screen.getByText('可尝试渠道').parentElement).toHaveTextContent('1');
  expect(screen.getByText(/native method unavailable/)).toBeInTheDocument();
});

it('does not offer a channel when both streams or all keys are cooling', () => {
  const { rerender } = render(<ScannerProviderStatus scanner={{ ...scanner, providers: [{ ...provider, stream_failures: [...provider.stream_failures!, { stream: 'token', cooldown_until: checked + 1000, last_error: 'token unavailable' }] }] }} />);
  expect(screen.getByText('分流冷却')).toBeInTheDocument();
  expect(screen.getByText('可尝试渠道').parentElement).toHaveTextContent('0');
  rerender(<ScannerProviderStatus scanner={{ ...scanner, providers: [{ ...provider, keys: [{ ...provider.keys[0], cooldown_until: checked + 1000 }] }] }} />);
  expect(screen.getAllByText('无可用凭据')).toHaveLength(3);
  expect(screen.getByText('可尝试渠道').parentElement).toHaveTextContent('0');
});

it('treats a missing stream snapshot or a failed refresh as unknown', () => {
  const { rerender } = render(<ScannerProviderStatus scanner={{ ...scanner, providers: [{ ...provider, stream_failures: undefined }] }} />);
  expect(screen.getByText('分流状态待更新')).toBeInTheDocument();
  expect(screen.getByText('可尝试渠道').parentElement).toHaveTextContent('—');
  rerender(<ScannerProviderStatus scanner={scanner} unavailable />);
  expect(screen.getAllByText('状态待刷新').length).toBeGreaterThan(0);
  expect(screen.getByText('可尝试渠道').parentElement).toHaveTextContent('—');
});

it('orders by priority then ID and retains errors after cooldown expires', () => {
  const backup = { ...provider, id: 3, name: 'Backup', priority: 5, stream_failures: [] };
  render(<ScannerProviderStatus scanner={{ ...scanner, checked_at: checked + 120000, providers: [backup, provider] }} />);
  const rows = screen.getAllByRole('listitem');
  expect(within(rows[0]).getByText('Primary')).toBeInTheDocument();
  expect(within(rows[1]).getByText('备用 1')).toBeInTheDocument();
  expect(screen.queryByText('部分冷却')).not.toBeInTheDocument();
  expect(screen.getByText(/native method unavailable/)).toBeInTheDocument();
  expect(screen.getByText('可尝试渠道').parentElement).toHaveTextContent('2');
});

it('strictly parses stream failures and permits unknown fields only when absent', () => {
  const input = { scanners: [scanner], assets: [], addresses: [], platform_assets: [] };
  expect(parseSettings(input).scanners[0].providers[0].stream_failures).toEqual(provider.stream_failures);
  expect(parseSettings({ ...input, scanners: [{ ...scanner, providers: [{ ...provider, stream_failures: undefined }] }] }).scanners[0].providers[0].stream_failures).toBeUndefined();
  for (const stream_failures of [null, {}, [{ stream: 'other', cooldown_until: null, last_error: null }], [...provider.stream_failures!, ...provider.stream_failures!], [{ stream: 'native', cooldown_until: 'tomorrow', last_error: null }]]) {
    expect(() => parseSettings({ ...input, scanners: [{ ...scanner, providers: [{ ...provider, stream_failures }] }] })).toThrow();
  }
  expect(() => parseSettings({ ...input, scanners: [{ ...scanner, providers: [{ ...provider, network: 'eth' }] }] })).toThrow();
});
