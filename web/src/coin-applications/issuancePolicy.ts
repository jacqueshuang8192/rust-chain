import { canonicalDecimalText, compareDecimalText, decimalFitsStorage, decimalFitsPrecision, isPositiveDecimalText } from '../shared/decimal';
import { isUnixMillis, requiredSafeInteger } from '../shared/integer';
import { requiredNewCoinLocalDateTimeMillis } from '../admin/newCoinDateTime';

export type PhaseName = 'subscription' | 'distribution' | 'purchase' | 'listing';
export const phaseNames: PhaseName[] = ['subscription', 'distribution', 'purchase', 'listing'];
export const phaseLabels: Record<PhaseName, string> = { subscription: '申购开始', distribution: '进入派发', purchase: '认购开始', listing: '认购结束并上市' };
export type LockRule = { kind: 'none' | 'on_purchase' | 'on_listing' } | { kind: 'weeks' | 'months'; count: number } | { kind: 'fixed'; at: number };
export type ExitFee = { kind: 'none' } | { kind: 'profit' | 'gross'; rate: string } | { kind: 'fixed'; amount: string };
export type IssuancePolicy = { version: 2; timezone: string; purchase_initial_price: string; listing_reference_price: string;
  subscription: PhaseNode; distribution: PhaseNode; purchase: PhaseNode; listing: PhaseNode; lock: LockRule; fee: ExitFee };
type PhaseNode = { automatic: boolean; at: number };
export type PolicyDraft = { purchasePrice: string; listingPrice: string; timezone: string;
  nodes: Record<PhaseName, { automatic: boolean; local: string }>;
  lock: LockRule['kind']; period: string; unlockAt: string; fee: ExitFee['kind']; feeValue: string };
export function emptyPolicyDraft(): PolicyDraft {
  return { purchasePrice: '', listingPrice: '', timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    nodes: { subscription: { automatic: false, local: '' }, distribution: { automatic: false, local: '' }, purchase: { automatic: false, local: '' }, listing: { automatic: false, local: '' } },
    lock: 'on_listing', period: '', unlockAt: '', fee: 'none', feeValue: '' };
}
function localInstant(text: string, label: string): number {
  const time = requiredNewCoinLocalDateTimeMillis(text, label);
  const d = new Date(time);
  for (let minutes = -180; minutes <= 180; minutes += 15) {
    if (!minutes) continue;
    const candidate = new Date(time + minutes * 60_000);
    if (candidate.getFullYear() === d.getFullYear() && candidate.getMonth() === d.getMonth() && candidate.getDate() === d.getDate() && candidate.getHours() === d.getHours() && candidate.getMinutes() === d.getMinutes()) {
      throw new Error(`${label}位于夏令时重复时段，请选择不重复的时间`);
    }
  }
  return time;
}
export function buildPolicy(draft: PolicyDraft, quotePrecision: number): IssuancePolicy {
  const nodes = Object.fromEntries(phaseNames.map(key => [key, { automatic: draft.nodes[key].automatic, at: localInstant(draft.nodes[key].local, phaseLabels[key]) }])) as Record<PhaseName, PhaseNode>;
  let lock: LockRule;
  if (draft.lock === 'weeks' || draft.lock === 'months') lock = { kind: draft.lock, count: requiredSafeInteger(draft.period, '锁仓周期', 1, draft.lock === 'weeks' ? 520 : 120) };
  else if (draft.lock === 'fixed') lock = { kind: 'fixed', at: localInstant(draft.unlockAt, '固定解锁时间') };
  else lock = { kind: draft.lock };
  const fee: ExitFee = draft.fee === 'none' ? { kind: 'none' } : draft.fee === 'fixed' ? { kind: 'fixed', amount: draft.feeValue } : { kind: draft.fee, rate: draft.feeValue };
  const policy: IssuancePolicy = { version: 2, timezone: draft.timezone, purchase_initial_price: draft.purchasePrice, listing_reference_price: draft.listingPrice, ...nodes, lock, fee };
  validatePolicy(policy, quotePrecision); return policy;
}
export function validatePolicy(raw: unknown, precision = 18): asserts raw is IssuancePolicy {
  const p = raw as IssuancePolicy | undefined;
  if (!p || p.version !== 2 || typeof p.timezone !== 'string') throw new Error('发行规则不完整');
  try { new Intl.DateTimeFormat('zh-CN', { timeZone: p.timezone }).format(); } catch { throw new Error('发行时区无效'); }
  if (![p.purchase_initial_price, p.listing_reference_price].every(v => decimalFitsStorage(v) && isPositiveDecimalText(v) && decimalFitsPrecision(v, precision))) throw new Error('认购初价和上市参考价必须为正且符合计价资产精度');
  for (const [i, key] of phaseNames.entries()) {
    if (!p[key] || typeof p[key].automatic !== 'boolean' || !isUnixMillis(p[key].at)) throw new Error(`${phaseLabels[key]}时间或开关无效`);
    if (i > 0 && p[key].at <= p[phaseNames[i - 1]].at) throw new Error('申购、派发、认购和上市时间必须依次递增');
  }
  if (!p.lock || !['none', 'on_purchase', 'on_listing', 'weeks', 'months', 'fixed'].includes(p.lock.kind)) throw new Error('锁仓规则无效');
  if (p.lock.kind === 'weeks' || p.lock.kind === 'months') requiredSafeInteger(p.lock.count, '锁仓周期', 1, p.lock.kind === 'weeks' ? 520 : 120);
  if (p.lock.kind === 'fixed' && !isUnixMillis(p.lock.at)) throw new Error('固定解锁时间无效');
  if (!p.fee || !['none', 'profit', 'gross', 'fixed'].includes(p.fee.kind)) throw new Error('退出费规则无效');
  if (p.fee.kind !== 'none') {
    const amount = p.fee.kind === 'fixed' ? p.fee.amount : p.fee.rate;
    if (!decimalFitsStorage(amount) || compareDecimalText(amount, '0') === -1) throw new Error('费用需为有效的非负数');
    if (p.fee.kind === 'fixed' ? !decimalFitsPrecision(amount, precision) : compareDecimalText(amount, '1') === 1) throw new Error('固定费用须符合资产精度；比例请填 0 至 1');
  }
}
export function policyIdentity(p: IssuancePolicy | null | undefined): string {
  if (!p) return 'legacy';
  return JSON.stringify([p.version, p.timezone, canonicalDecimalText(p.purchase_initial_price), canonicalDecimalText(p.listing_reference_price), phaseNames.map(k => [p[k].at, p[k].automatic]),
    p.lock.kind, 'count' in p.lock ? p.lock.count : 'at' in p.lock ? p.lock.at : null, p.fee.kind, p.fee.kind === 'none' ? null : canonicalDecimalText(p.fee.kind === 'fixed' ? p.fee.amount : p.fee.rate)]);
}
export function lockLabel(lock: LockRule): string {
  switch (lock.kind) {
    case 'none': return '不锁仓'; case 'on_purchase': return '实际进入认购时解禁'; case 'on_listing': return '实际上市时解禁';
    case 'weeks': return `原购买时间起 ${lock.count} 周`; case 'months': return `原购买时间起 ${lock.count} 个自然月`;
    case 'fixed': return `固定时间 ${new Date(lock.at).toLocaleString()}`;
  }
}
