import { policyIdentity, validatePolicy, type IssuancePolicy } from '../coin-applications/issuancePolicy';
import { apiRequest, ContractError } from './client';
import { compareDecimalText, decimalFitsPrecision, decimalFitsStorage, isPositiveDecimalText } from '../shared/decimal';
import { isUnixMillis, requiredSafeInteger } from '../shared/integer';

export type CoinInput = { logo_url?: string | null; issuance_policy?: IssuancePolicy | null; symbol: string; name: string; precision_scale: number; total_supply: string; issue_price: string; quote_asset_id: number; listed_at: number; reason: string };
export type CoinStatus = 'pending' | 'approved' | 'rejected';
export type CoinApplication = CoinInput & Record<string, unknown> & {
  id: number; agent_id: number; agent_admin_id: number; quote_symbol: string; status: CoinStatus;
  project_id: number | null; reviewed_by: number | null; review_reason: string | null; created_at: number; reviewed_at: number | null;
};
export type CoinAsset = { asset_id: number; symbol: string; precision_scale: number };
export type CoinPage = { applications: CoinApplication[]; total: number };
export const coinStatuses = { pending: '待审核', approved: '已通过 · 已发布', rejected: '已驳回' };
export const coinPath = (scope: 'admin' | 'agent') => scope === 'agent' ? '/agent/api/v1/coin-applications' : '/admin/api/v1/agent-coin-applications';
function object(value: unknown): value is Record<string, unknown> { return Boolean(value) && typeof value === 'object' && !Array.isArray(value); }
function integer(value: unknown, min = 1): value is number { return typeof value === 'number' && Number.isSafeInteger(value) && value >= min; }
function assert(value: unknown): asserts value { if (!value) throw new ContractError('发币申请响应无效，请刷新核对', { path: 'coin-applications' }); }
export function validateCoinInput(input: CoinInput): CoinInput {
  const symbol = input.symbol.trim().toUpperCase(), name = input.name.trim(), reason = input.reason.trim();
  if (!/^[A-Z0-9]{1,32}$/.test(symbol)) throw new Error('币种代码需为 1 至 32 位英文字母或数字');
  if (!name || new TextEncoder().encode(name).length > 128) throw new Error('币种名称不能为空且最多 128 字节');
  requiredSafeInteger(input.precision_scale, '资产精度', 0, 18);
  requiredSafeInteger(input.quote_asset_id, '计价资产', 1);
  if (![input.total_supply, input.issue_price].every((value) => decimalFitsStorage(value) && isPositiveDecimalText(value))) throw new Error('总量和价格必须是正数，最多 20 位整数、18 位小数');
  if (!decimalFitsPrecision(input.total_supply, input.precision_scale)) throw new Error('发行总量超过新币精度');
  if (!isUnixMillis(input.listed_at)) throw new Error('请选择有效的计划上市时间');
  if (!reason || [...reason].length > 500) throw new Error('申请说明需为 1 至 500 字');
  if (input.issuance_policy) { validatePolicy(input.issuance_policy); if (input.issuance_policy.listing.at !== input.listed_at) throw new Error("计划上市时间与发行规则不一致"); }
  if (input.logo_url != null && (typeof input.logo_url !== 'string' || [...input.logo_url].length > 2048)) throw new Error('币种 Logo 地址无效');
  return { ...input, symbol, name, reason, ...(input.logo_url != null ? { logo_url: input.logo_url.trim() || null } : {}) };
}
export function parseCoinApplication(value: unknown): CoinApplication {
  assert(object(value));
  assert(['id', 'agent_id', 'agent_admin_id', 'quote_asset_id'].every((key) => integer(value[key])));
  assert(integer(value.precision_scale, 0) && value.precision_scale <= 18);
  assert(['symbol', 'name', 'quote_symbol', 'reason'].every((key) => typeof value[key] === 'string' && String(value[key]).trim()));
  assert([value.total_supply, value.issue_price].every((v) => decimalFitsStorage(v) && isPositiveDecimalText(v)));
  assert(value.status === 'pending' || value.status === 'approved' || value.status === 'rejected');
  assert(isUnixMillis(value.created_at) && isUnixMillis(value.listed_at));
  assert(value.project_id === null || integer(value.project_id));
  assert((value.status === 'approved') === (value.project_id !== null));
  assert(value.status === 'pending'
    ? value.reviewed_by === null && value.review_reason === null && value.reviewed_at === null
    : integer(value.reviewed_by) && typeof value.review_reason === 'string' && value.review_reason.trim() && isUnixMillis(value.reviewed_at));
  assert(value.logo_url == null || (typeof value.logo_url === 'string' && [...value.logo_url].length <= 2048));
  if (value.issuance_policy != null) validatePolicy(value.issuance_policy);
  return value as CoinApplication;
}
export async function coinOptions(signal?: AbortSignal): Promise<CoinAsset[]> {
  const value = await apiRequest(`${coinPath('agent')}/options`, { authScope: 'agent', signal });
  assert(Array.isArray(value));
  assert(value.every((v) => object(v) && integer(v.asset_id) && typeof v.symbol === 'string' && v.symbol.trim() && integer(v.precision_scale, 0) && v.precision_scale <= 18));
  return value as CoinAsset[];
}
export async function coinList(scope: 'agent' | 'admin', status: CoinStatus | '', page: number, signal?: AbortSignal, size = 20): Promise<CoinPage> {
  const value = await apiRequest(`${coinPath(scope)}?limit=${requiredSafeInteger(size, '每页条数', 1, 100)}&offset=${requiredSafeInteger((requiredSafeInteger(page, '页码', 1, 100001) - 1) * size, '分页偏移', 0, 100000)}${status ? `&status=${status}` : ''}`, { authScope: scope, signal });
  assert(object(value) && Array.isArray(value.applications) && integer(value.total, 0));
  return { applications: value.applications.map(parseCoinApplication), total: value.total };
}
export async function coinDetail(scope: 'agent' | 'admin', id: number, signal?: AbortSignal): Promise<CoinApplication> {
  const row = parseCoinApplication(await apiRequest(`${coinPath(scope)}/${requiredSafeInteger(id, '申请ID', 1)}`, { authScope: scope, signal }));
  assert(row.id === id); return row;
}
export async function submitCoin(input: CoinInput, key: string): Promise<CoinApplication> {
  const payload = validateCoinInput(input);
  if (!key || key.length > 128) throw new Error('申请请求标识无效');
  const row = parseCoinApplication(await apiRequest(coinPath('agent'), { authScope: 'agent', method: 'POST', body: JSON.stringify({ ...payload, idempotency_key: key }) }));
  assert(row.symbol === payload.symbol && (row.logo_url ?? null) === (payload.logo_url ?? null) && row.name === payload.name && row.precision_scale === payload.precision_scale &&
    row.quote_asset_id === payload.quote_asset_id && row.listed_at === payload.listed_at && row.reason === payload.reason &&
    compareDecimalText(row.total_supply, payload.total_supply) === 0 && compareDecimalText(row.issue_price, payload.issue_price) === 0 && policyIdentity(row.issuance_policy) === policyIdentity(payload.issuance_policy));
  return row;
}
export async function reviewCoin(id: number, decision: 'approve' | 'reject', reason: string): Promise<CoinApplication> {
  if (!reason.trim() || [...reason.trim()].length > 500) throw new Error('审核原因需为 1 至 500 字');
  const row = parseCoinApplication(await apiRequest(`${coinPath('admin')}/${requiredSafeInteger(id, '申请ID', 1)}/review`, { authScope: 'admin', method: 'POST', body: JSON.stringify({ decision, reason: reason.trim() }) }));
  assert(row.id === id && row.status === (decision === 'approve' ? 'approved' : 'rejected') && row.review_reason === reason.trim());
  return row;
}
