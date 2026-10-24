import { expect, it } from 'vitest';
import type { Asset, Scanner } from './contracts';
import { parseSettings } from './contracts';
import { methodBlockers, scanStatus } from './setupStatus';
const scanner: Scanner = { network: 'eth', enabled: true, endpoint_configured: true, api_key_configured: false, confirmations: 12, revision: 1, scan_fresh: true, cursor_height: 100, checked_at: 1, last_scan_at: 1, last_error: null };
const asset: Asset = { id: 1, network: 'eth', asset_symbol: 'ETH', asset_kind: 'native', token_address: '', decimals: 18, enabled: true, revision: 1 };
const catalog = [{ symbol: 'ETH', status: 'active', deposit_enabled: true, precision_scale: 18 }];
const addresses = [{ pool_id: 1, network: 'eth', address: '0x1', enabled: true }];
it('uses authoritative snapshot freshness rather than client wall clock or an enabled switch', () => {
  expect(scanStatus(scanner).ready).toBe(true);
  expect(scanStatus({ ...scanner, scan_fresh: false }).label).toBe('扫描状态已过期');
  expect(scanStatus({ ...scanner, last_scan_at: null, scan_fresh: false }).label).toBe('等待扫描确认');
  expect(scanStatus({ ...scanner, last_error: '' }).ready).toBe(false);
  expect(scanStatus({ ...scanner, endpoint_configured: false }).ready).toBe(false);
});
it('separates scan success from admission and explains all method blockers', () => {
  expect(methodBlockers(asset, scanner, addresses, catalog)).toEqual([]);
  const paused = { ...scanner, enabled: false };
  expect(scanStatus(paused).ready).toBe(true);
  expect(methodBlockers(asset, paused, addresses, catalog)).toEqual(['网络已暂停接单']);
  expect(methodBlockers({ ...asset, enabled: false }, undefined, [], [])).toEqual(['监听币种已停用', '缺少平台资产', '网络已暂停接单', '缺少启用的收款地址', '未配置节点']);
  expect(methodBlockers(asset, scanner, [{ ...addresses[0], network: 'base' }], [{ ...catalog[0], deposit_enabled: false }])).toEqual(['平台资产充值未开启', '缺少启用的收款地址']);
});
it('requires new scan metadata and rejects malformed heights instead of reporting healthy', () => {
  const data = { scanners: [scanner], assets: [], addresses: [], platform_assets: [] };
  expect(parseSettings(data).scanners[0].cursor_height).toBe(100);
  expect(() => parseSettings({ ...data, scanners: [{ ...scanner, scan_fresh: undefined }] })).toThrow();
  expect(() => parseSettings({ ...data, scanners: [{ ...scanner, cursor_height: 1.2 }] })).toThrow();
});
