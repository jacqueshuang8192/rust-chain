import { authStore, type AuthSession } from '../auth/authStore';
import { ApiError } from '../api/client';
import { canonicalRequestIntent } from '../shared/idempotency';

export function rechargeDraftKey(session: AuthSession, command: string, id: number): string {
  return `recharge-draft:${canonicalRequestIntent({
    scope: session.scope, subject: session.subject, generation: session.generation, command, id
  })}`;
}

export function readRechargeDraft<T>(key: string, validate: (value: unknown) => value is T): T | null {
  try {
    const raw = window.sessionStorage.getItem(key);
    if (!raw) return null;
    const value: unknown = JSON.parse(raw);
    if (value === null) return null;
    if (validate(value)) return value;
    throw new Error('invalid draft');
  } catch {
    throw new Error('未决充值记录无法读取，请恢复浏览器会话存储后重试，勿另建充值');
  }
}

export function saveRechargeDraft(key: string, value: unknown): void {
  try {
    const raw = JSON.stringify(value);
    window.sessionStorage.setItem(key, raw);
    if (window.sessionStorage.getItem(key) !== raw) throw new Error('storage unavailable');
  } catch {
    throw new Error('无法保存充值请求身份，本次未发送；请启用浏览器会话存储');
  }
}

export function clearRechargeDraft(key: string): void {
  // Persist a success tombstone before releasing the financial lease. Failed
  // removal must never resurrect a form with a fresh key after a known credit.
  saveRechargeDraft(key, null);
}

export function assertRechargeSession(session: AuthSession): void {
  const current = authStore.getSession(session.scope);
  if (current?.subject !== session.subject || current.generation !== session.generation) {
    throw new Error('登录会话已变更，请重新进入页面');
  }
}

export function definitiveRechargeFailure(error: unknown): boolean {
  // A conflict can describe an already-executed command; retain its original identity.
  // Authentication/scope can change after an earlier successful request lost
  // its response. Those failures must not release the original credit key.
  return error instanceof ApiError && [400, 422].includes(error.status);
}
