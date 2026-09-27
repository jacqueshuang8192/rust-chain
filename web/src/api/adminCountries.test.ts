import { beforeEach, describe, expect, it, vi } from 'vitest';
import { apiRequest } from './client';
import { listActiveAdminCountries } from './adminCountries';

vi.mock('./client', async (original) => ({ ...await original<typeof import('./client')>(), apiRequest: vi.fn() }));
const request = vi.mocked(apiRequest);
const rows = (count: number, offset = 0) => Array.from({ length: count }, (_, index) => ({ country_code: `C${offset + index}`, country_name: `Country ${offset + index}` }));

describe('complete admin country directory', () => {
  beforeEach(() => request.mockReset());

  it('loads all 205 countries with actual offsets and propagates cancellation', async () => {
    const controller = new AbortController();
    request.mockResolvedValueOnce({ countries: rows(100), total: 205 }).mockResolvedValueOnce({ countries: rows(100, 100), total: 205 }).mockResolvedValueOnce({ countries: rows(5, 200), total: 205 });
    const result = await listActiveAdminCountries(controller.signal);
    expect(result).toHaveLength(205);
    expect(result[204].country_code).toBe('C204');
    expect(request.mock.calls.map(([path]) => new URL(String(path), 'http://local').searchParams.get('offset'))).toEqual(['0', '100', '200']);
    expect(request.mock.calls.every(([, options]) => options?.signal === controller.signal)).toBe(true);
  });

  it('continues short pages when total says more rows remain', async () => {
    request.mockResolvedValueOnce({ countries: rows(2), total: 3 }).mockResolvedValueOnce({ countries: rows(1, 2), total: 3 });
    expect(await listActiveAdminCountries()).toHaveLength(3);
    expect(request.mock.calls[1][0]).toContain('offset=2');
  });

  it('rejects a failed later page without returning partial options', async () => {
    request.mockResolvedValueOnce({ countries: rows(100), total: 101 }).mockRejectedValueOnce(new Error('offline'));
    await expect(listActiveAdminCountries()).rejects.toThrow('offline');
  });

  it.each([
    { countries: [], total: 101 },
    { countries: rows(1), total: 101 },
    { countries: rows(1, 100), total: 102 },
  ])('rejects missing, overlapping or changed directory pages', async (second) => {
    request.mockResolvedValueOnce({ countries: rows(100), total: 101 }).mockResolvedValueOnce(second);
    await expect(listActiveAdminCountries()).rejects.toThrow('国家目录');
  });

  it('rejects malformed country identity and excessive totals', async () => {
    request.mockResolvedValueOnce({ countries: [{ country_code: 1, country_name: 'Invalid' }], total: 1 });
    await expect(listActiveAdminCountries()).rejects.toThrow('无效或重复');
    request.mockResolvedValueOnce({ countries: [], total: 10001 });
    await expect(listActiveAdminCountries()).rejects.toThrow('安全上限');
  });

  it('does not use a response after an ignored abort', async () => {
    const controller = new AbortController();
    request.mockImplementationOnce(async () => { controller.abort(); return { countries: rows(1), total: 1 }; });
    await expect(listActiveAdminCountries(controller.signal)).rejects.toMatchObject({ name: 'AbortError' });
    expect(request).toHaveBeenCalledTimes(1);
  });

  it('handles empty directories and legacy single pages without total', async () => {
    request.mockResolvedValueOnce({ countries: [], total: 0 }).mockResolvedValueOnce({ countries: rows(2) });
    expect(await listActiveAdminCountries()).toEqual([]);
    expect(await listActiveAdminCountries()).toHaveLength(2);
  });
});
