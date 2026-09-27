import { listAdminResource } from './adminResources';
import { ContractError } from './client';
import type { ApiRecord } from './types';

export type AdminCountryRecord = ApiRecord & { country_code: string; country_name: string };

/** 国家目录必须完整返回；后续页失败或目录变化时拒绝整次读取，不展示残缺选项。 */
export async function listActiveAdminCountries(signal?: AbortSignal): Promise<AdminCountryRecord[]> {
  const endpoint = '/admin/api/v1/countries';
  const countries: AdminCountryRecord[] = [];
  const seen = new Set<string>();
  let expectedTotal: number | undefined;
  for (let page = 0; page < 100; page += 1) {
    signal?.throwIfAborted();
    const result = await listAdminResource<AdminCountryRecord>(endpoint, 'countries', {
      status: 'active', limit: 100, offset: countries.length
    }, { signal, rowContract: { requiredFields: ['country_code', 'country_name'] } });
    signal?.throwIfAborted();
    if (result.total !== undefined) {
      if (result.total > 10_000 || (expectedTotal !== undefined && expectedTotal !== result.total)) {
        throw new ContractError('国家目录数量变化或超出安全上限，请重新加载', { path: endpoint });
      }
      expectedTotal = result.total;
    }
    for (const row of result.rows) {
      const code = typeof row.country_code === 'string' ? row.country_code.trim().toUpperCase() : '';
      if (!code || typeof row.country_name !== 'string' || seen.has(code)) {
        throw new ContractError('国家目录包含无效或重复记录，请重新加载', { path: endpoint });
      }
      seen.add(code);
      countries.push(row);
    }
    if (expectedTotal !== undefined) {
      if (countries.length > expectedTotal || (result.rows.length === 0 && countries.length < expectedTotal)) {
        throw new ContractError('国家目录分页不完整，请重新加载', { path: endpoint });
      }
      if (countries.length === expectedTotal) return countries;
    } else if (result.rows.length < 100) {
      return countries;
    }
  }
  throw new ContractError('国家目录超过分页安全上限，请缩小配置范围', { path: endpoint });
}
