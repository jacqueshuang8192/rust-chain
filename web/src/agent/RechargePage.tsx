import { Banner, Button, Modal, Select, TextArea, Typography } from '@douyinfe/semi-ui';
import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { Link, useParams } from 'react-router-dom';
import { authStore, type AuthSession } from '../auth/authStore';
import { getRechargeOptions, rechargeReason, submitAgentRecharge, type RechargeInput, type RechargeOptions } from '../api/recharges';
import { PageHeader } from '../layouts/PageHeader';
import { RechargeHistory, rechargeStatuses } from '../recharges/RechargeHistory';
import { assertRechargeSession, clearRechargeDraft, definitiveRechargeFailure, readRechargeDraft, rechargeDraftKey, saveRechargeDraft } from '../recharges/recovery';
import { AdminTextInput } from '../shared/SemiFormControls';
import { canonicalDecimalText, decimalFitsPrecision, decimalFitsStorage, isPositiveDecimalText } from '../shared/decimal';
import { financialCommandIntents, financialCommandScopeFromSession, runRecoverableFinancialCommand } from '../shared/idempotency';
import { parseSafeInteger, requiredSafeInteger } from '../shared/integer';
import { adminErrorMessage } from '../shared/adminErrorMessage';

function validInput(value: unknown): value is RechargeInput {
  if (!value || typeof value !== 'object') return false;
  const input = value as RechargeInput;
  return typeof input.asset_id === 'number' && parseSafeInteger(input.asset_id, 1) !== null &&
    decimalFitsStorage(input.amount) && isPositiveDecimalText(input.amount) && typeof input.reason === 'string' && Boolean(input.reason.trim());
}

function RechargeForm({ session, userId, onSuccess }: { session: AuthSession; userId: number; onSuccess: () => void }) {
  const key = rechargeDraftKey(session, 'submit', userId);
  const [initial] = useState(() => {
    try { return { pending: readRechargeDraft(key, validInput), error: '' }; }
    catch (cause) { return { pending: null, error: adminErrorMessage(cause) }; }
  });
  const [frozen, setFrozen] = useState<RechargeInput | null>(initial.pending);
  const [asset, setAsset] = useState(String(initial.pending?.asset_id ?? ''));
  const [amount, setAmount] = useState(initial.pending?.amount ?? '');
  const [reason, setReason] = useState(initial.pending?.reason ?? '');
  const [options, setOptions] = useState<RechargeOptions | null>(null);
  const [optionsError, setOptionsError] = useState('');
  const [revision, setRevision] = useState(0);
  const [error, setError] = useState(initial.error);
  const [receipt, setReceipt] = useState('');
  const [confirmation, setConfirmation] = useState<RechargeInput | null>(null);
  const [busy, setBusy] = useState(false);
  const flight = useRef(false);
  useEffect(() => {
    const controller = new AbortController();
    setOptions(null); setOptionsError('');
    void getRechargeOptions(controller.signal)
      .then((value) => { if (!controller.signal.aborted) setOptions(value); })
      .catch((cause: unknown) => { if (!controller.signal.aborted) setOptionsError(adminErrorMessage(cause, '充值选项加载失败')); });
    return () => controller.abort();
  }, [revision]);
  function prepare() {
    try {
      if (initial.error) throw new Error(initial.error);
      assertRechargeSession(session);
      const input = frozen ?? { asset_id: requiredSafeInteger(asset, '充值资产', 1), amount, reason: rechargeReason(reason) };
      if (!decimalFitsStorage(input.amount) || !isPositiveDecimalText(input.amount)) throw new Error('充值金额必须为正数，最多 20 位整数、18 位小数，不会自动舍入');
      const normalized = { ...input, amount: canonicalDecimalText(input.amount)! };
      const scope = financialCommandScopeFromSession(session, 'agent-user-recharge', userId, normalized.asset_id);
      const values = { ...normalized, user_id: userId };
      const hasPending = financialCommandIntents.hasPending(scope, values);
      if (frozen && !hasPending) throw new Error('原充值请求标识缺失，请联系管理员核对申请记录，不能重新生成充值请求');
      if (!hasPending) {
        const precision = options?.assets.find((item) => item.asset_id === normalized.asset_id)?.precision_scale;
        if (precision === undefined) throw new Error('请重新加载充值选项并确认资产精度');
        if (!decimalFitsPrecision(normalized.amount, precision)) throw new Error(`该资产最多支持 ${precision} 位小数，不会自动舍入`);
      }
      setError(''); setReceipt(''); setConfirmation(normalized);
    } catch (cause) { setError(adminErrorMessage(cause)); }
  }
  async function submit() {
    if (flight.current || !confirmation) return;
    flight.current = true; setBusy(true); setError('');
    const input = confirmation;
    try {
      assertRechargeSession(session);
      const scope = financialCommandScopeFromSession(session, 'agent-user-recharge', userId, input.asset_id);
      const values = { ...input, user_id: userId };
      if (frozen && !financialCommandIntents.hasPending(scope, values)) {
        throw new Error('原充值请求标识缺失，请联系管理员核对申请记录，不能重新生成充值请求');
      }
      saveRechargeDraft(key, input);
      setFrozen(input);
      const record = await runRecoverableFinancialCommand({
        scope, values, store: financialCommandIntents,
        isDefinitiveFailure: definitiveRechargeFailure,
        request: async (idempotencyKey) => {
          if (!financialCommandIntents.hasPending(scope, values)) throw new Error('充值请求标识未能保存，本次未发送');
          const result = await submitAgentRecharge(userId, input, idempotencyKey);
          clearRechargeDraft(key);
          return result;
        }
      });
      setFrozen(null); setConfirmation(null); setAmount(''); setReason('');
      setReceipt(`申请 ${record.id}：${rechargeStatuses[record.status]}，${record.amount} ${record.asset_symbol}`);
      onSuccess();
    } catch (cause) {
      if (definitiveRechargeFailure(cause)) {
        try { clearRechargeDraft(key); setFrozen(null); setConfirmation(null); }
        catch (storageError) { setError(adminErrorMessage(storageError)); return; }
      }
      setError(adminErrorMessage(cause, '结果尚未确认，请按原请求重试核对'));
    } finally { flight.current = false; setBusy(false); }
  }
  const precision = options?.assets.find((item) => String(item.asset_id) === asset)?.precision_scale;
  const assetOptions = options?.assets.map((item) => ({ label: `${item.symbol}（${item.precision_scale} 位小数）`, value: String(item.asset_id) })) ?? [];
  if (frozen && !assetOptions.some((item) => item.value === String(frozen.asset_id))) {
    assetOptions.push({ label: `原请求资产 ID ${frozen.asset_id}`, value: String(frozen.asset_id) });
  }
  return <section className="recharge-form">
    <Banner type="warning" description="后台人工充值进入用户现货钱包可用余额，不扣代理钱包。默认审核后入账；免审授权仅当前代理有效，不继承。既有待审核申请不会自动入账。" />
    <p>{options ? `当前新申请：${options.direct_recharge_enabled ? '免审直充' : '需管理员审核'}` : '正在确认当前充值路径'}</p>
    {frozen ? <Banner type="warning" description="有结果未确认的原申请，已保留用户、资产、金额及原因。请核对原请求，勿另建相同充值。" /> : null}
    <label>用户ID<AdminTextInput ariaLabel="充值用户ID" value={String(userId)} disabled onChange={() => undefined} /></label>
    <label id="agent-recharge-asset">充值资产
      <Select aria-labelledby="agent-recharge-asset" value={asset} disabled={Boolean(frozen) || busy} optionList={assetOptions}
        onChange={(value) => setAsset(String(value))} onSelect={(value) => setAsset(String(value))} />
    </label>
    <label>充值金额<AdminTextInput ariaLabel="充值金额" value={amount} disabled={Boolean(frozen) || busy} onChange={setAmount} /></label>
    {precision !== undefined ? <p>资产精度：{precision} 位小数，超出精度不会自动舍入。</p> : null}
    <label>申请原因<TextArea aria-label="申请原因" value={reason} disabled={Boolean(frozen) || busy} onChange={setReason} /></label>
    {optionsError ? <p role="alert">{optionsError}</p> : null}
    <Button onClick={() => setRevision((value) => value + 1)} disabled={busy}>重新加载充值选项</Button>
    {error ? <p role="alert">{error}</p> : null}
    {receipt ? <p role="status">{receipt}</p> : null}
    <Button theme="solid" onClick={prepare} disabled={busy || Boolean(initial.error) || (!frozen && (!asset || !amount.trim() || !reason.trim()))}>
      {frozen ? '核对原请求' : '提交充值申请'}
    </Button>
    <Modal title="确认充值申请" visible={confirmation !== null} motion={false} maskClosable={false}
      closeOnEsc={!busy} onCancel={() => { if (!flight.current) setConfirmation(null); }} onOk={() => void submit()}
      confirmLoading={busy} okText={frozen ? '按原请求重试' : '确认提交'}
      okButtonProps={{ 'aria-label': frozen ? '按原请求重试' : '确认提交' }}
      cancelButtonProps={{ 'aria-label': '取消', disabled: busy }}
      width="min(560px, calc(100vw - 32px))">
      {confirmation ? <div className="recharge-form">
        <p>用户ID：{userId}；资产ID：{confirmation.asset_id}</p>
        <p>金额：{confirmation.amount}</p><p>原因：{confirmation.reason}</p>
        <p>申请不可修改；实际审核路径以服务端提交结果为准。</p>
        {error ? <p role="alert">{error}</p> : null}
      </div> : null}
    </Modal>
  </section>;
}

function AgentRechargeContent({ session, userId }: { session: AuthSession; userId?: number }) {
  const [revision, setRevision] = useState(0);
  return <main className="exchange-page">
    <PageHeader title={userId === undefined ? '我的充值申请' : `用户 ${userId} 充值`} actions={<Link to={userId === undefined ? '/agent/users' : '/agent/recharges'}>{userId === undefined ? '选择团队用户充值' : '我的全部申请'}</Link>} />
    {userId !== undefined ? <RechargeForm session={session} userId={userId} onSuccess={() => setRevision((value) => value + 1)} /> : null}
    <Typography.Title heading={5}>本人发起的申请</Typography.Title>
    <p>仅当前登录代理账号发起、且用户仍在当前团队范围内的申请，不含下级代理账号的历史。</p>
    <RechargeHistory scope="agent" userId={userId} revision={revision}
      action={userId === undefined ? (row) => <Link to={`/agent/users/${row.user_id}/recharge`}>用户充值</Link> : undefined} />
  </main>;
}

export function AgentRechargePage() {
  const session = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('agent'));
  const { userId: rawUserId } = useParams();
  const userId = rawUserId === undefined ? undefined : parseSafeInteger(rawUserId, 1);
  if (!session || userId === null) return <p role="alert">登录会话或用户ID无效</p>;
  return <AgentRechargeContent key={`${session.subject}:${session.generation}:${userId ?? 'all'}`} session={session} userId={userId} />;
}
