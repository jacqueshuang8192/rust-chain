import { apiRequest, ContractError } from './client';
import { compareDecimalText, decimalFitsStorage, isPositiveDecimalText } from '../shared/decimal';
import { isUnixMillis, requiredSafeInteger } from '../shared/integer';

export type RechargeStatus = 'pending' | 'credited' | 'rejected';
export type RechargeRecord = Record<string, unknown> & {
  id: number;
  agent_id: number;
  agent_admin_id: number;
  user_id: number;
  asset_id: number;
  asset_symbol: string;
  amount: string;
  reason: string;
  mode: 'review' | 'direct';
  status: RechargeStatus;
  reviewed_by: number | null;
  review_reason: string | null;
  created_at: number;
  reviewed_at: number | null;
  credited_at: number | null;
};
export type RechargeOptions = {
  direct_recharge_enabled: boolean;
  assets: Array<{ asset_id: number; symbol: string; precision_scale: number }>;
};
export type RechargePolicy = { agent_id: number; direct_recharge_enabled: boolean; version: number };
export type RechargeInput = { asset_id: number; amount: string; reason: string };
export type RechargeReview = { decision: 'approve' | 'reject'; reason: string };
export type RechargeQuery = { status?: RechargeStatus; user_id?: number; limit: number; offset: number };
export type RechargePage = { recharges: RechargeRecord[]; total: number };

function object(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}
function integer(value: unknown, min = 1): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= min;
}
function assert(condition: unknown, path: string): asserts condition {
  if (!condition) throw new ContractError('充值接口响应字段无效，请刷新核对，勿重复发起新充值', { path });
}
export function parseRecharge(value: unknown, path: string): RechargeRecord {
  assert(object(value), path);
  assert(['id', 'agent_id', 'agent_admin_id', 'user_id', 'asset_id'].every((key) => integer(value[key])), path);
  assert(typeof value.asset_symbol === 'string' && value.asset_symbol.trim(), path);
  assert(decimalFitsStorage(value.amount) && isPositiveDecimalText(value.amount), path);
  assert(typeof value.reason === 'string' && value.reason.trim(), path);
  assert(value.mode === 'review' || value.mode === 'direct', path);
  assert(['pending', 'credited', 'rejected'].includes(String(value.status)), path);
  assert(value.reviewed_by === null || integer(value.reviewed_by), path);
  assert(value.review_reason === null || typeof value.review_reason === 'string', path);
  assert(isUnixMillis(value.created_at) &&
    (value.reviewed_at === null || isUnixMillis(value.reviewed_at)) &&
    (value.credited_at === null || isUnixMillis(value.credited_at)), path);
  assert((value.status === 'credited') === (value.credited_at !== null), path);
  const reviewed = value.mode === 'review' && value.status !== 'pending';
  assert(reviewed
    ? value.reviewed_by !== null && value.reviewed_at !== null && typeof value.review_reason === 'string' && value.review_reason.trim()
    : value.reviewed_by === null && value.reviewed_at === null && value.review_reason === null, path);
  assert(value.mode !== 'direct' || value.status === 'credited', path);
  return value as RechargeRecord;
}
export function rechargeReason(value: string): string {
  const reason = value.trim();
  if (!reason || [...reason].length > 500) throw new Error('原因需为 1 至 500 个字符');
  return reason;
}
export async function getRechargeOptions(signal?: AbortSignal): Promise<RechargeOptions> {
  const path = '/agent/api/v1/recharges/options';
  const value = await apiRequest(path, { authScope: 'agent', signal });
  assert(object(value) && typeof value.direct_recharge_enabled === 'boolean' && Array.isArray(value.assets), path);
  assert(value.assets.every((asset) => object(asset) && integer(asset.asset_id) &&
    typeof asset.symbol === 'string' && asset.symbol.trim() &&
    integer(asset.precision_scale, 0) && asset.precision_scale <= 18), path);
  assert(new Set(value.assets.map((asset) => asset.asset_id)).size === value.assets.length, path);
  return value as RechargeOptions;
}
export async function submitAgentRecharge(userId: number, input: RechargeInput, key: string): Promise<RechargeRecord> {
  requiredSafeInteger(userId, '用户ID', 1);
  requiredSafeInteger(input.asset_id, '资产ID', 1);
  if (!decimalFitsStorage(input.amount) || !isPositiveDecimalText(input.amount)) throw new Error('充值金额超出容量或不是正数');
  if (!key.trim()) throw new Error('缺少充值请求标识');
  const path = `/agent/api/v1/users/${userId}/recharge`;
  const record = parseRecharge(await apiRequest(path, {
    authScope: 'agent', method: 'POST',
    body: JSON.stringify({ ...input, reason: rechargeReason(input.reason), idempotency_key: key })
  }), path);
  assert(record.user_id === userId && record.asset_id === input.asset_id &&
    compareDecimalText(record.amount, input.amount) === 0 && record.reason === input.reason.trim(), path);
  return record;
}
export async function listRecharges(
  scope: 'agent' | 'admin', query: RechargeQuery & { agent_id?: number }, signal?: AbortSignal
): Promise<RechargePage> {
  const params = new URLSearchParams();
  if (query.status) {
    if (!['pending', 'credited', 'rejected'].includes(query.status)) throw new Error('无效充值状态');
    params.set('status', query.status);
  }
  if (query.user_id !== undefined) params.set('user_id', String(requiredSafeInteger(query.user_id, '用户ID', 1)));
  if (scope === 'admin' && query.agent_id !== undefined) params.set('agent_id', String(requiredSafeInteger(query.agent_id, '代理ID', 1)));
  params.set('limit', String(requiredSafeInteger(query.limit, '分页大小', 1, 100)));
  params.set('offset', String(requiredSafeInteger(query.offset, '分页偏移', 0, 100000)));
  const path = `${scope === 'agent' ? '/agent/api/v1/recharges' : '/admin/api/v1/agent-recharges'}?${params}`;
  const value = await apiRequest(path, { authScope: scope, signal });
  assert(object(value) && Array.isArray(value.recharges) && integer(value.total, 0), path);
  return { recharges: value.recharges.map((row) => parseRecharge(row, path)), total: value.total };
}
export async function reviewRecharge(id: number, input: RechargeReview): Promise<RechargeRecord> {
  requiredSafeInteger(id, '申请ID', 1);
  if (!['approve', 'reject'].includes(input.decision)) throw new Error('无效审核决定');
  const path = `/admin/api/v1/agent-recharges/${id}/review`;
  const record = parseRecharge(await apiRequest(path, {
    authScope: 'admin', method: 'POST', body: JSON.stringify({ decision: input.decision, reason: rechargeReason(input.reason) })
  }), path);
  assert(record.id === id && record.status === (input.decision === 'approve' ? 'credited' : 'rejected') &&
    record.review_reason === input.reason.trim(), path);
  return record;
}

export async function getAdminRecharge(id: number): Promise<RechargeRecord> {
  requiredSafeInteger(id, '申请ID', 1);
  const path = `/admin/api/v1/agent-recharges/${id}`;
  const record = parseRecharge(await apiRequest(path, { authScope: 'admin' }), path);
  assert(record.id === id, path);
  return record;
}
function policy(value: unknown, path: string, id: number): RechargePolicy {
  assert(object(value) && value.agent_id === id && typeof value.direct_recharge_enabled === 'boolean' && integer(value.version, 0), path);
  return value as RechargePolicy;
}
export async function getRechargePolicy(id: number, signal?: AbortSignal): Promise<RechargePolicy> {
  requiredSafeInteger(id, '代理ID', 1);
  const path = `/admin/api/v1/agents/${id}/recharge-policy`;
  return policy(await apiRequest(path, { authScope: 'admin', signal }), path, id);
}
export async function updateRechargePolicy(id: number, input: { direct_recharge_enabled: boolean; expected_version: number; reason: string }): Promise<RechargePolicy> {
  requiredSafeInteger(id, '代理ID', 1);
  requiredSafeInteger(input.expected_version, '授权版本', 0);
  if (typeof input.direct_recharge_enabled !== 'boolean') throw new Error('无效授权状态');
  const path = `/admin/api/v1/agents/${id}/recharge-policy`;
  return policy(await apiRequest(path, {
    authScope: 'admin', method: 'PATCH', body: JSON.stringify({ ...input, reason: rechargeReason(input.reason) })
  }), path, id);
}
