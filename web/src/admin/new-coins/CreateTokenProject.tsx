import { Button, Modal, SideSheet, TextArea } from '@douyinfe/semi-ui';
import { useRef, useState, useSyncExternalStore } from 'react';
import { Link } from 'react-router-dom';
import { authStore, type AuthSession } from '../../auth/authStore';
import { apiRequest, ApiError } from '../../api/client';
import { IssuancePolicyFields } from '../../coin-applications/IssuancePolicyFields';
import { IssuancePolicySummary } from '../../coin-applications/IssuancePolicySummary';
import { buildPolicy, emptyPolicyDraft, policyIdentity, validatePolicy, type IssuancePolicy } from '../../coin-applications/issuancePolicy';
import { AdminSelect, AdminTextInput } from '../../shared/SemiFormControls';
import { adminErrorMessage } from '../../shared/adminErrorMessage';
import { canonicalDecimalText, decimalFitsPrecision, decimalFitsStorage, isPositiveDecimalText } from '../../shared/decimal';
import { requiredSafeInteger } from '../../shared/integer';
import { useCanAdminRequest } from '../access';
import { useAssetOptions, type CreateActionProps } from '../resources/actions/shared';
import { UnsavedChangesGuard } from '../settings/UnsavedChangesGuard';
import '../../coin-applications/coinApplications.css';

type Intent = { asset_id: number; quote_asset_id: number; total_supply: string; issue_price: string; issuance_policy: IssuancePolicy; reason: string; idempotency_key: string };
function validate(value: unknown): Intent {
  const v = value as Intent;
  requiredSafeInteger(v?.asset_id, '发行资产', 1); requiredSafeInteger(v?.quote_asset_id, '计价资产', 1);
  if (v.asset_id === v.quote_asset_id || ![v.total_supply, v.issue_price].every(n => decimalFitsStorage(n) && isPositiveDecimalText(n))
    || typeof v.reason !== 'string' || !v.reason.trim() || v.reason.length > 500 || typeof v.idempotency_key !== 'string' || !v.idempotency_key) throw new Error('发行资料不完整');
  validatePolicy(v.issuance_policy); return v;
}
function readIntent(key: string): { intent: Intent | null; error: string } {
  try { const raw = sessionStorage.getItem(key); return { intent: raw && raw !== 'null' ? validate(JSON.parse(raw)) : null, error: '' }; }
  catch { return { intent: null, error: '未确认的创建记录无法恢复，请先核对项目管理并恢复会话存储。' }; }
}
function persist(key: string, value: Intent | null) {
  try {
    const raw = JSON.stringify(value); sessionStorage.setItem(key, raw);
    if (sessionStorage.getItem(key) !== raw) throw new Error('storage mismatch');
  } catch {
    throw new Error(value ? '无法保存创建请求标识，本次未发送。' : '创建结果已返回，但无法更新本地恢复记录，请核对项目或按原请求重试。');
  }
}

export function CreateTokenProject(props: CreateActionProps) {
  const session = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('admin'));
  const writable = useCanAdminRequest('/admin/api/v1/new-coins/issuance', 'POST');
  return session && writable ? <CreateForm key={`${session.subject}:${session.generation}`} session={session} {...props} /> : null;
}
function CreateForm({ session, onCreated }: CreateActionProps & { session: AuthSession }) {
  const storageKey = `admin-token-issuance:${session.subject}:${session.generation}`;
  const [initial] = useState(() => readIntent(storageKey));
  const [intent, setIntent] = useState(initial.intent);
  const [open, setOpen] = useState(false), [busy, setBusy] = useState(false), [error, setError] = useState(initial.error);
  const [confirmation, setConfirmation] = useState<Intent | null>(null), [created, setCreated] = useState<number | null>(null);
  const [values, setValues] = useState({ asset: '', quote: '', supply: '', price: '', reason: '' });
  const [policy, setPolicy] = useState(emptyPolicyDraft);
  const flight = useRef(false);
  const canReadAssets = useCanAdminRequest('/admin/api/v1/assets', 'GET');
  const { assetOptions, assetLoading, assetError } = useAssetOptions(open && canReadAssets);
  const disabled = busy || Boolean(intent) || Boolean(initial.error);
  const base = assetOptions.find(v => v.id === values.asset), quote = assetOptions.find(v => v.id === values.quote);
  function prepare() {
    try {
      if (intent) { setConfirmation(intent); setError(''); return; }
      if (!base || !quote) throw new Error('请先选择已启用的发行资产和计价资产');
      const next = validate({ asset_id: requiredSafeInteger(base.id, '发行资产', 1), quote_asset_id: requiredSafeInteger(quote.id, '计价资产', 1),
        total_supply: values.supply.trim(), issue_price: values.price.trim(), issuance_policy: buildPolicy(policy, quote.precisionScale ?? 18),
        reason: values.reason.trim(), idempotency_key: crypto.randomUUID() });
      if (!decimalFitsPrecision(next.total_supply, base.precisionScale ?? 18) || !decimalFitsPrecision(next.issue_price, quote.precisionScale ?? 18)) throw new Error('发行总量或申购价格超出资产精度');
      setConfirmation(next); setError('');
    } catch (cause) { setError(adminErrorMessage(cause)); }
  }
  async function submit() {
    if (flight.current || !confirmation) return;
    flight.current = true; setBusy(true); setError('');
    try {
      if (authStore.getSession('admin')?.generation !== session.generation) throw new Error('登录会话已变化，请重新打开页面');
      const current = intent ?? confirmation; persist(storageKey, current); setIntent(current);
      const result = await apiRequest<Record<string, unknown>>('/admin/api/v1/new-coins/issuance', { method: 'POST', body: JSON.stringify(current) });
      if (authStore.getSession('admin')?.generation !== session.generation) throw new Error('登录会话已变化，请在原会话核对创建结果');
      const id = requiredSafeInteger(result.id, '项目编号', 1);
      if (result.asset_id !== current.asset_id || result.quote_asset_id !== current.quote_asset_id) throw new Error('返回的项目资产不符，请按原请求重试核对');
      validatePolicy(result.issuance_policy);
      if (policyIdentity(result.issuance_policy) !== policyIdentity(current.issuance_policy)
        || typeof result.total_supply !== 'string' || typeof result.issue_price !== 'string'
        || canonicalDecimalText(result.total_supply) !== canonicalDecimalText(current.total_supply)
        || canonicalDecimalText(result.issue_price) !== canonicalDecimalText(current.issue_price)) throw new Error('返回的发行合同不符，请按原请求重试核对');
      persist(storageKey, null); setIntent(null); setConfirmation(null); setCreated(id); setOpen(false);
      setValues({ asset: '', quote: '', supply: '', price: '', reason: '' }); setPolicy(emptyPolicyDraft()); onCreated?.();
    } catch (cause) {
      if (cause instanceof ApiError && ([400, 422].includes(cause.status) || (cause.status === 409 && cause.code === 'TOKEN_ISSUANCE_EXISTS'))) {
        try { persist(storageKey, null); setIntent(null); } catch { /* Keep the original intent if storage recovery fails. */ }
      }
      setError(adminErrorMessage(cause, '创建结果尚未确认，已保留原始配置，请按原请求重试。'));
    } finally { flight.current = false; setBusy(false); }
  }
  return <>
    <Button theme="solid" onClick={() => setOpen(true)}>添加新币项目</Button>
    {created ? <Link to={`/admin/new-coins/projects/${created}`}>查看已创建项目 {created}</Link> : null}
    <SideSheet title="添加新币项目" visible={open} width="min(920px, calc(100vw - 32px))" bodyStyle={{ overflowY: 'auto' }}
      maskClosable={false} closeOnEsc={!busy} onCancel={() => { if (!busy) setOpen(false); }}>
      <UnsavedChangesGuard enabled={busy || (open && Object.values(values).some(Boolean))} />
      <section className="coin-application-form">
        <p>从已启用资产创建新版发行项目。创建后进入预热，生成专属交易对与待启动行情，并补齐零余额钱包。</p>
        {intent ? <p role="alert">创建结果待核对，原始配置已锁定。重试会返回同一个项目，不会重复发行。</p> : <>
          <div className="coin-application-fields">
            <label id="admin-token-base-label">发行资产<AdminSelect ariaLabel="发行资产" ariaLabelledBy="admin-token-base-label" value={values.asset} disabled={disabled || assetLoading} optionList={assetOptions.map(v => ({ value: v.id, label: v.label }))} onChange={asset => setValues(v => ({ ...v, asset, quote: v.quote === asset ? '' : v.quote }))} /></label>
            <label id="admin-token-quote-label">计价资产<AdminSelect ariaLabel="计价资产" ariaLabelledBy="admin-token-quote-label" value={values.quote} disabled={disabled || assetLoading} optionList={assetOptions.filter(v => v.id !== values.asset).map(v => ({ value: v.id, label: v.label }))} onChange={quote => setValues(v => ({ ...v, quote }))} /></label>
            <label>发行总量<AdminTextInput ariaLabel="发行总量" value={values.supply} disabled={disabled} onChange={supply => setValues(v => ({ ...v, supply }))} /></label>
            <label>申购价格<AdminTextInput ariaLabel="申购价格" value={values.price} disabled={disabled} onChange={price => setValues(v => ({ ...v, price }))} /></label>
          </div>
          <IssuancePolicyFields value={policy} onChange={setPolicy} disabled={disabled} />
          <label>创建原因<TextArea aria-label="创建原因" value={values.reason} disabled={disabled} onChange={reason => setValues(v => ({ ...v, reason }))} /></label>
        </>}
        {!canReadAssets ? <p role="alert">缺少资产读取权限，请联系管理员分配资产读取权限。</p> : null}
        {assetError ? <p role="alert">{adminErrorMessage(assetError)}</p> : null}
        {error ? <p role="alert">{error}</p> : null}
        <Button theme="solid" disabled={busy || Boolean(initial.error) || (!intent && (!canReadAssets || assetLoading || Boolean(assetError)))} onClick={prepare}>{intent ? '核对原创建请求' : '提交添加新币项目'}</Button>
      </section>
    </SideSheet>
    <Modal title="确认创建新版发行项目" visible={Boolean(confirmation)} motion={false} width="min(560px, calc(100vw - 32px))" maskClosable={false}
      closeOnEsc={!busy} onCancel={() => { if (!busy) setConfirmation(null); }} onOk={() => void submit()} confirmLoading={busy}
      okText={intent ? '按原请求重试' : '确认创建'} okButtonProps={{ 'aria-label': intent ? '按原请求重试' : '确认创建' }} cancelButtonProps={{ disabled: busy, 'aria-label': '取消' }}>
      {confirmation ? <><p>发行资产 ID {confirmation.asset_id} · 计价资产 ID {confirmation.quote_asset_id}</p><p>发行总量 {confirmation.total_supply} · 固定申购价 {confirmation.issue_price}</p>
        <IssuancePolicySummary policy={confirmation.issuance_policy} /><p>创建原因：{confirmation.reason}</p><p>创建后合同固定；各阶段按独立开关推进，派发需人工确认。</p></> : null}
      {error ? <p role="alert">{error}</p> : null}
    </Modal>
  </>;
}
