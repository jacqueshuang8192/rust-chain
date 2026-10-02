import { Banner, Button, Select, SideSheet, TextArea } from '@douyinfe/semi-ui';
import { useRef, useState, useSyncExternalStore } from 'react';
import { authStore, type AuthSession } from '../../auth/authStore';
import { getAdminRecharge, parseRecharge, rechargeReason, reviewRecharge, type RechargeRecord, type RechargeReview } from '../../api/recharges';
import { PageHeader } from '../../layouts/PageHeader';
import { RechargeHistory, rechargeStatuses } from '../../recharges/RechargeHistory';
import { assertRechargeSession, clearRechargeDraft, definitiveRechargeFailure, readRechargeDraft, rechargeDraftKey, saveRechargeDraft } from '../../recharges/recovery';
import { adminErrorMessage } from '../../shared/adminErrorMessage';
import { financialCommandIntents, financialCommandScopeFromSession, runRecoverableFinancialCommand } from '../../shared/idempotency';
import { useCanAdminRequest } from '../access';

type ReviewDraft = { record: RechargeRecord; input: RechargeReview };
function validDraft(value: unknown): value is ReviewDraft {
  if (!value || typeof value !== 'object') return false;
  const draft = value as ReviewDraft;
  try { parseRecharge(draft.record, 'review-draft'); }
  catch { return false; }
  return Boolean(draft.input && ['approve', 'reject'].includes(draft.input.decision) && typeof draft.input.reason === 'string' && draft.input.reason.trim());
}

function ReviewContent({ session }: { session: AuthSession }) {
  const key = rechargeDraftKey(session, 'review', 0);
  const [initial] = useState(() => {
    try { return { pending: readRechargeDraft(key, validDraft), error: '' }; }
    catch (cause) { return { pending: null, error: adminErrorMessage(cause) }; }
  });
  const [pending, setPending] = useState(initial.pending);
  const [record, setRecord] = useState<RechargeRecord | null>(initial.pending?.record ?? null);
  const [visible, setVisible] = useState(Boolean(initial.pending));
  const [decision, setDecision] = useState<RechargeReview['decision']>(initial.pending?.input.decision ?? 'approve');
  const [reason, setReason] = useState(initial.pending?.input.reason ?? '');
  const [error, setError] = useState(initial.error);
  const [receipt, setReceipt] = useState('');
  const [revision, setRevision] = useState(0);
  const [busy, setBusy] = useState(false);
  const flight = useRef(false);
  const canReview = useCanAdminRequest('/admin/api/v1/agent-recharges/1/review', 'POST');
  async function reconcile() {
    if (flight.current || !pending) return;
    flight.current = true; setBusy(true); setError('');
    try {
      assertRechargeSession(session);
      const result = await getAdminRecharge(pending.record.id);
      if (result.status === 'pending') {
        setError('服务器仍为待审核，请按原决定重试。');
        return;
      }
      clearRechargeDraft(key);
      setPending(null); setRecord(null); setVisible(false);
      setReceipt(`申请 ${result.id}：${rechargeStatuses[result.status]}；审核人ID：${result.reviewed_by ?? '-'}，原因：${result.review_reason ?? '-'}`);
      setRevision((value) => value + 1);
    } catch (cause) {
      setError(adminErrorMessage(cause, '查询未成功，原审核请求已保留'));
    } finally { flight.current = false; setBusy(false); }
  }
  async function submit() {
    if (flight.current || !record || !canReview || initial.error) return;
    flight.current = true; setBusy(true); setError('');
    try {
      assertRechargeSession(session);
      const draft = pending ?? { record, input: { decision, reason: rechargeReason(reason) } };
      saveRechargeDraft(key, draft); setPending(draft);
      const response = await runRecoverableFinancialCommand({
        scope: financialCommandScopeFromSession(session, `agent-recharge-review:${draft.record.id}`, draft.record.user_id, draft.record.asset_id),
        values: { ...draft.input, id: draft.record.id }, store: financialCommandIntents,
        isDefinitiveFailure: definitiveRechargeFailure,
        request: async () => {
          const result = await reviewRecharge(draft.record.id, draft.input);
          clearRechargeDraft(key);
          return result;
        }
      });
      setPending(null); setRecord(null); setVisible(false); setReason('');
      setReceipt(`申请 ${response.id}：${rechargeStatuses[response.status]}`);
      setRevision((value) => value + 1);
    } catch (cause) {
      if (definitiveRechargeFailure(cause)) {
        try { clearRechargeDraft(key); setPending(null); }
        catch (storageError) { setError(adminErrorMessage(storageError)); return; }
      }
      setError(adminErrorMessage(cause, '审核结果尚未确认，请按原决定和原因重试'));
    } finally { flight.current = false; setBusy(false); }
  }
  return <main className="exchange-page">
    <PageHeader title="代理充值审核" />
    {!canReview ? <Banner description="当前角色仅可查看充值申请，无审核权限。" type="info" /> : null}
    {initial.error ? <p role="alert">{initial.error}</p> : null}
    {receipt ? <p role="status">{receipt}</p> : null}
    {pending ? <Banner type="warning" description={<>
      申请 {pending.record.id} 审核结果未确认，原决定及原因已保留。
      <Button onClick={() => setVisible(true)}>继续核对审核</Button>
      <Button disabled={busy} onClick={() => void reconcile()}>查询最终审核结果</Button>
    </>} /> : null}
    <RechargeHistory scope="admin" revision={revision} action={canReview ? (row) =>
      row.status === 'pending' ? <Button size="small" disabled={Boolean(pending) || Boolean(initial.error) || busy} onClick={() => {
        setRecord(row); setDecision('approve'); setReason(''); setError(''); setVisible(true);
      }}>审核</Button> : null : undefined} />
    <SideSheet visible={visible} title="审核充值申请" width="min(640px, calc(100vw - 32px))" onCancel={() => { if (!flight.current) setVisible(false); }}>
      {record ? <section className="recharge-form">
        <p>申请ID：{record.id}；代理ID：{record.agent_id}；发起账号ID：{record.agent_admin_id}</p>
        <p>用户ID：{record.user_id}</p><p>金额：{record.amount} {record.asset_symbol}</p>
        <p>申请原因：{record.reason}</p>
        <Banner type="warning" description="批准后进入用户现货钱包可用余额，不扣代理钱包。拒绝后不入账，终态不可修改。" />
        <label id="recharge-decision">审核决定
          <Select aria-labelledby="recharge-decision" value={decision} disabled={Boolean(pending) || busy || !canReview}
            onChange={(value) => setDecision(value as RechargeReview['decision'])}
            onSelect={(value) => setDecision(value as RechargeReview['decision'])}
            optionList={[{ label: '批准入账', value: 'approve' }, { label: '拒绝申请', value: 'reject' }]} />
        </label>
        <label>审核原因<TextArea aria-label="审核原因" value={reason} disabled={Boolean(pending) || busy || !canReview} onChange={setReason} /></label>
        {error ? <p role="alert">{error}</p> : null}
        {pending ? <Button disabled={busy} onClick={() => void reconcile()}>查询最终审核结果</Button> : null}
        {canReview ? <Button theme="solid" type={decision === 'reject' ? 'danger' : 'primary'} loading={busy}
          disabled={!reason.trim() || Boolean(initial.error)} onClick={() => void submit()}>{pending ? '按原决定重试' : '确认审核'}</Button> : null}
      </section> : null}
    </SideSheet>
  </main>;
}

export function RechargeReviewPage() {
  const session = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('admin'));
  return session ? <ReviewContent key={`${session.subject}:${session.generation}`} session={session} /> : <p role="alert">管理员会话已失效</p>;
}
