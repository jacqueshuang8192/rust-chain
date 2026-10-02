import { Banner, Button, SideSheet, Switch, TextArea } from '@douyinfe/semi-ui';
import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { ApiError } from '../../api/client';
import { getRechargePolicy, updateRechargePolicy, type RechargePolicy } from '../../api/recharges';
import { authStore, type AuthSession } from '../../auth/authStore';
import { assertRechargeSession } from '../../recharges/recovery';
import { adminErrorMessage } from '../../shared/adminErrorMessage';
import { parseSafeInteger } from '../../shared/integer';
import { useCanAdminRequest } from '../access';
import '../../recharges/recharges.css';

function PolicyEditor({ id, session }: { id: number; session: AuthSession }) {
  const [policy, setPolicy] = useState<RechargePolicy | null>(null);
  const [enabled, setEnabled] = useState(false);
  const [reason, setReason] = useState('');
  const [error, setError] = useState('');
  const [conflict, setConflict] = useState(false);
  const [revision, setRevision] = useState(0);
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);
  const flight = useRef(false);
  const canWrite = useCanAdminRequest(`/admin/api/v1/agents/${id}/recharge-policy`, 'PATCH');
  useEffect(() => {
    const controller = new AbortController();
    setPolicy(null); setError(''); setConflict(false);
    void getRechargePolicy(id, controller.signal).then((value) => {
      if (!controller.signal.aborted) { setPolicy(value); setEnabled(value.direct_recharge_enabled); setReason(''); }
    }).catch((cause: unknown) => { if (!controller.signal.aborted) setError(adminErrorMessage(cause, '授权加载失败')); });
    return () => controller.abort();
  }, [id, revision]);
  async function save() {
    if (!policy || !canWrite || conflict || flight.current) return;
    flight.current = true; setBusy(true); setError(''); setSaved(false);
    try {
      assertRechargeSession(session);
      const updated = await updateRechargePolicy(id, { direct_recharge_enabled: enabled, expected_version: policy.version, reason });
      setPolicy(updated); setEnabled(updated.direct_recharge_enabled); setReason(''); setSaved(true);
    } catch (cause) {
      setConflict(cause instanceof ApiError && cause.status === 409);
      setError(adminErrorMessage(cause, '授权保存结果未确认，请保留原版本重试或重新读取'));
    } finally { flight.current = false; setBusy(false); }
  }
  return <section className="recharge-form">
    <p>代理ID：{id}</p>
    <Banner type="warning" description="免审授权仅对当前代理有效，不向下级继承。撤销即时影响新申请；开启或撤销授权均不会自动执行既有待审核申请。直充不扣代理钱包。" />
    {policy ? <>
      <p>已保存状态：{policy.direct_recharge_enabled ? '免审直充' : '需管理员审核'}；版本：{policy.version}</p>
      {canWrite ? <>
        <label>允许免审直充<Switch aria-label="允许免审直充" checked={enabled} disabled={busy} onChange={(value) => { setEnabled(value); setSaved(false); }} /></label>
        <label>授权变更原因<TextArea aria-label="授权变更原因" value={reason} disabled={busy} onChange={setReason} /></label>
        <Button theme="solid" disabled={busy || conflict || !reason.trim() || enabled === policy.direct_recharge_enabled} loading={busy} onClick={() => void save()}>保存授权</Button>
      </> : <p>当前角色仅可查看，无免审授权修改权限。</p>}
    </> : null}
    {error ? <p role="alert">{conflict ? '授权已被他人修改，输入已保留。请放弃当前输入并重新读取后再操作。' : error}</p> : null}
    {saved ? <p role="status">授权已保存</p> : null}
    <Button disabled={busy} onClick={() => setRevision((value) => value + 1)}>{policy ? '放弃输入并重新读取' : '重新读取授权'}</Button>
  </section>;
}

export function RechargePolicyAction({ agentId }: { agentId: string }) {
  const [visible, setVisible] = useState(false);
  const session = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('admin'));
  const id = parseSafeInteger(agentId, 1);
  const canRead = useCanAdminRequest(`/admin/api/v1/agents/${agentId}/recharge-policy`, 'GET');
  if (!canRead || !session || id === null) return null;
  return <>
    <Button size="small" theme="borderless" onClick={() => setVisible(true)}>充值授权</Button>
    <SideSheet title="代理充值授权" visible={visible} width="min(640px, calc(100vw - 32px))" onCancel={() => setVisible(false)}>
      {visible ? <PolicyEditor key={`${id}:${session.subject}:${session.generation}`} id={id} session={session} /> : null}
    </SideSheet>
  </>;
}
