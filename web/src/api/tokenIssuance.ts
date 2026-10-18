import { apiRequest, ContractError } from './client';
import { validatePolicy, type IssuancePolicy } from '../coin-applications/issuancePolicy';
import { decimalFitsStorage } from '../shared/decimal';
import { requiredSafeInteger } from '../shared/integer';
export type TokenProject = { id: number; symbol: string; lifecycle_status: string; status: string; issuer_agent_id: number; issuance_policy: IssuancePolicy;
  actual_purchase_at: number | null; actual_listed_at: number | null; next_transition_at: number | null; transition_error: string | null };
export const stageLabels: Record<string, string> = { preheat: '预热中', subscription: '申购中', distribution: '派发中', purchase: '认购中', listed: '已上市' };
export const nextStages: Record<string, string> = { preheat: 'subscription', subscription: 'distribution', distribution: 'purchase', purchase: 'listed' };
export type AllocationInput = { subscription_ids: number[]; percent: string; reason: string; idempotency_key: string };
export type AllocationReceipt = { idempotency_key: string; results: Array<{ subscription_id: number; quantity: string; settled_quote_amount: string; refunded_quote_amount: string; holding_id: number | null; status: string }> };
function record(v: unknown): v is Record<string, unknown> { return !!v && typeof v === 'object' && !Array.isArray(v); }
function assert(v: unknown): asserts v { if (!v) throw new ContractError('发行操作响应不完整，请核对记录', { path: 'token-issuance' }); }
function parseProject(v: unknown): TokenProject {
  assert(record(v) && typeof v.symbol === 'string' && typeof v.lifecycle_status === 'string' && v.lifecycle_status in stageLabels);
  requiredSafeInteger(v.id, '项目编号', 1); requiredSafeInteger(v.issuer_agent_id, '发行代理', 1); validatePolicy(v.issuance_policy);
  return v as TokenProject;
}
export async function tokenProjects(signal?: AbortSignal, before?: number): Promise<{ projects: TokenProject[]; next: number | null }> {
  const response = await apiRequest(`/agent/api/v1/token-projects${before ? `?before_id=${before}` : ''}`, { authScope: 'agent', signal });
  assert(record(response) && Array.isArray(response.projects));
  const next = response.next_before_id == null ? null : requiredSafeInteger(response.next_before_id, '下一页', 1);
  return { projects: response.projects.map(parseProject), next };
}
export async function advanceTokenProject(project: TokenProject, reason: string): Promise<TokenProject> {
  const target = nextStages[project.lifecycle_status]; if (!target) throw new Error('项目已上市');
  const row = parseProject(await apiRequest(`/agent/api/v1/token-projects/${project.id}/stage`, { authScope: 'agent', method: 'POST', body: JSON.stringify({ expected_stage: project.lifecycle_status, target_stage: target, reason }) }));
  assert(row.id === project.id && row.lifecycle_status === target); return row;
}
export async function allocateTokens(input: AllocationInput): Promise<AllocationReceipt> {
  const v = await apiRequest('/agent/api/v1/new-coin-allocations', { authScope: 'agent', method: 'POST', body: JSON.stringify(input) });
  assert(record(v) && v.idempotency_key === input.idempotency_key && Array.isArray(v.results) && v.results.length === input.subscription_ids.length);
  const seen = new Set<number>();
  for (const row of v.results) {
    assert(record(row) && typeof row.subscription_id === 'number' && input.subscription_ids.includes(row.subscription_id) && !seen.has(row.subscription_id));
    seen.add(row.subscription_id); assert(['quantity', 'settled_quote_amount', 'refunded_quote_amount'].every(k => decimalFitsStorage(row[k])));
    assert(['held', 'refunded'].includes(String(row.status)));
  }
  return v as AllocationReceipt;
}
