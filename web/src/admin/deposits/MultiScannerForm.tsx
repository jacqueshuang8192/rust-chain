import { Banner, Button, Collapse, Switch, Tag } from '@douyinfe/semi-ui';
import { useState } from 'react';
import { apiRequest } from '../../api/client';
import { ConfirmAction } from '../../shared/ConfirmAction';
import { AdminPasswordInput, AdminSelect, AdminTextInput } from '../../shared/SemiFormControls';
import { parseSafeInteger } from '../../shared/integer';
import { base, type RpcPreset, type Scanner } from './contracts';
import { nodePresets, type NodePreset } from './presets';

type KeyDraft = { uid: string; id: number | null; name: string; enabled: boolean; priority: string; api_key: string; configured: boolean; clear_api_key: boolean; interval: string };
type ProviderDraft = { preset_id: string; template_id?: string; uid: string; id: number | null; name: string; enabled: boolean; priority: string; endpoint: string; interval: string; keys: KeyDraft[] };
const newKey = (priority = 0): KeyDraft => ({ uid: crypto.randomUUID(), id: null, name: `凭据 ${priority + 1}`, enabled: true, priority: String(priority), api_key: '', configured: false, clear_api_key: false, interval: '250' });
const newProvider = (priority: number): ProviderDraft => ({ preset_id: '', uid: crypto.randomUUID(), id: null, name: `渠道 ${priority + 1}`, enabled: true, priority: String(priority), endpoint: '', interval: '250', keys: [newKey()] });
function fromPreset(preset: RpcPreset, priority: number): ProviderDraft {
  return { ...newProvider(priority), preset_id: preset.id, name: preset.name, interval: String(preset.request_interval_ms), keys: [{ ...newKey(), name: '免密钥访问', interval: String(preset.request_interval_ms) }] };
}
function fromTemplate(template: NodePreset, priority: number): ProviderDraft {
  const interval = String(template.freeAccount?.interval ?? 500);
  return { ...newProvider(priority), template_id: template.id, name: template.freeAccount?.name ?? template.label, endpoint: template.endpoint, interval, keys: [{ ...newKey(), interval }] };
}
function payload(rows: ProviderDraft[], presets: RpcPreset[]) {
  if (!rows.length || rows.length > 10) throw new Error('请配置 1 至 10 个渠道');
  return rows.map(row => {
    const priority = parseSafeInteger(row.priority, 0, 1000); const interval = parseSafeInteger(row.interval, 100, 2000);
    if (!row.name.trim() || row.name.trim().length > 80 || priority === null || interval === null) throw new Error('请填写渠道名称、0 至 1000 的优先级及 100 至 2000 毫秒请求间隔');
    if (row.preset_id && !presets.some(preset => preset.id === row.preset_id)) throw new Error('所选免费渠道已不在目录中，请重新选择');
    if (!row.preset_id && !row.id && !row.endpoint.trim()) throw new Error(`${row.name} 需要节点地址`);
    if (row.endpoint.trim()) {
      let url: URL;
      try { url = new URL(row.endpoint.trim().replace('{API_KEY}', 'credential')); } catch { throw new Error(`${row.name} 的节点地址无效`); }
      if (!['http:', 'https:'].includes(url.protocol) || url.hash || row.endpoint.length > 2000) throw new Error(`${row.name} 需要有效 HTTP 或 HTTPS 地址`);
    }
    if (!row.keys.length || row.keys.length > 10 || (row.enabled && !row.keys.some(key => key.enabled))) throw new Error(`${row.name} 需要 1 至 10 个凭据槽，启用渠道至少启用一个槽`);
    return { id: row.id, preset_id: row.preset_id, name: row.name.trim(), enabled: row.enabled, priority, endpoint: row.preset_id ? null : row.endpoint.trim() || null, request_interval_ms: interval, keys: row.keys.map(key => {
      const priority = parseSafeInteger(key.priority, 0, 1000); const interval = parseSafeInteger(key.interval, 100, 2000);
      if (!key.name.trim() || key.name.trim().length > 80 || priority === null || interval === null || key.api_key.length > 1024 || /[\r\n]/.test(key.api_key)) throw new Error(`${row.name} 的凭据名称、优先级、间隔或密钥格式无效`);
      if (key.enabled && row.endpoint.includes('{API_KEY}') && (!key.api_key.trim() && (!key.configured || key.clear_api_key))) throw new Error(`${row.name} 的 URL 模板需要 API Key`);
      return { id: key.id, name: key.name.trim(), enabled: key.enabled, priority, api_key: key.api_key.trim() || null, clear_api_key: key.clear_api_key, request_interval_ms: interval };
    }) };
  });
}

type Props = { scanner: Scanner; presets?: RpcPreset[]; dirty: () => void; busy: boolean; setBusy: (busy: boolean) => void; saved: () => Promise<void> };
export function MultiScannerForm({ scanner, presets: catalog = [], dirty, busy, setBusy, saved }: Props) {
  const presets = catalog.filter(preset => preset.network === scanner.network);
  const accountTemplates = nodePresets.filter(preset => preset.network === scanner.network && preset.freeAccount);
  const [accountTemplateId, setAccountTemplateId] = useState(accountTemplates[0]?.id ?? '');
  const accountTemplate = accountTemplates.find(preset => preset.id === accountTemplateId);
  const [enabled, setEnabled] = useState(scanner.enabled);
  const [confirmations, setConfirmations] = useState(String(scanner.confirmations));
  const [rows, setRows] = useState<ProviderDraft[]>(() => scanner.providers.length ? scanner.providers.map(row => ({ preset_id: row.preset_id ?? '', uid: `provider-${row.id}`, id: row.id, name: row.name, enabled: row.enabled, priority: String(row.priority), endpoint: '', interval: String(row.request_interval_ms), keys: row.keys.map(key => ({ uid: `key-${key.id}`, id: key.id, name: key.name, enabled: key.enabled, priority: String(key.priority), api_key: '', configured: key.api_key_configured, clear_api_key: false, interval: String(key.request_interval_ms) })) })) : presets.some(preset => preset.recommended) ? presets.filter(preset => preset.recommended).map(fromPreset) : accountTemplates[0] ? [fromTemplate(accountTemplates[0], 0)] : [newProvider(0)]);
  const [expanded, setExpanded] = useState<string[]>(() => rows.slice(0, 1).map(row => row.uid));
  const append = (additions: ProviderDraft[]) => { dirty(); setRows(current => [...current, ...additions]); setExpanded(current => [...current, ...additions.map(row => row.uid)]); };
  const ordered = rows.filter(row => row.enabled && row.keys.some(key => key.enabled)).map((row, index) => ({ row, index })).sort((a, b) => (parseSafeInteger(a.row.priority, 0, 1000) ?? 0) - (parseSafeInteger(b.row.priority, 0, 1000) ?? 0) || (a.row.id ?? Number.MAX_SAFE_INTEGER) - (b.row.id ?? Number.MAX_SAFE_INTEGER) || a.index - b.index);
  const count = parseSafeInteger(confirmations, 1, 10000);
  let error = count === null ? '确认数必须为 1 至 10000 的整数' : enabled && !rows.some(row => row.enabled && row.keys.some(key => key.enabled)) ? '接单前至少启用一个渠道和凭据槽' : '';
  try { payload(rows, presets); } catch (cause) { error = cause instanceof Error ? cause.message : '渠道配置无效'; }
  const update = (uid: string, patch: Partial<ProviderDraft>) => { dirty(); setRows(current => current.map(row => row.uid === uid ? { ...row, ...patch } : row)); };
  const updateKey = (row: ProviderDraft, uid: string, patch: Partial<KeyDraft>) => update(row.uid, { keys: row.keys.map(key => key.uid === uid ? { ...key, ...patch } : key) });
  const missing = presets.filter(preset => preset.recommended && !rows.some(row => row.preset_id === preset.id));
  const nextPriority = Math.min(1000, Math.max(-1, ...rows.map(row => parseSafeInteger(row.priority, 0, 1000) ?? 0)) + 1);
  const selectPreset = (row: ProviderDraft, id: string) => {
    if (id === row.preset_id) return;
    const preset = presets.find(preset => preset.id === id);
    if (preset) {
      const key = row.keys[0] ?? newKey();
      update(row.uid, { preset_id: id, template_id: '', name: preset.name, endpoint: '', interval: String(preset.request_interval_ms), keys: [{ ...key, name: '免密钥访问', enabled: true, priority: '0', api_key: '', clear_api_key: key.configured, interval: String(preset.request_interval_ms) }] });
    } else {
      update(row.uid, { preset_id: '', endpoint: presets.find(preset => preset.id === row.preset_id)?.endpoint ?? row.endpoint });
    }
  };
  return <div className="deposit-edit-form">
    <label className="deposit-switch-row">接受新充值订单<Switch aria-label="接受新充值订单" disabled={busy} checked={enabled} onChange={value => { dirty(); setEnabled(value); }} /></label>
    <label>确认数 / finalized slot 深度<AdminTextInput ariaLabel="充值确认数" disabled={busy} value={confirmations} onChange={value => { dirty(); setConfirmations(value); }} /></label>
    <Banner type="info" description="按优先级切换独立渠道，再轮转同渠道的其他 Key。限流后保留冷却；同账户多个 Key 共用额度。" />
    {accountTemplate ? <section className="deposit-preset" aria-label="免费账号渠道">
      <strong>免费账号渠道 · 需申请 API Key</strong>
      <p>Ethereum 内部转账监听需要区块 trace 权限。选择下面的账号模板后填写自己的 Key，已有渠道继续保留。免费额度不代表无限使用，需等完整扫描成功。</p>
      <label id="account-rpc-template">账号渠道模板</label><AdminSelect motion={false} ariaLabel="账号渠道模板" ariaLabelledBy="account-rpc-template" disabled={busy} value={accountTemplateId} optionList={accountTemplates.map(template => ({ value: template.id, label: template.label }))} onChange={setAccountTemplateId} />
      <p>{accountTemplate.note}</p>
      <p><a href={accountTemplate.freeAccount?.signup} target="_blank" rel="noopener noreferrer">申请免费账号 / 获取 Key</a> · <a href={accountTemplate.source} target="_blank" rel="noopener noreferrer">查看账号渠道说明</a></p>
      <Button disabled={busy || rows.length >= 10 || rows.some(row => row.template_id === accountTemplate.id)} onClick={() => append([fromTemplate(accountTemplate, nextPriority)])}>添加所选账号渠道</Button>
    </section> : null}
    {presets.length ? <div className="deposit-preset">
      <strong>免密钥公共渠道</strong>
      <p>{presets.some(preset => preset.recommended) ? scanner.providers.length ? '补充默认渠道会追加到现有列表，已配置的渠道继续保留。' : '已填入本网络的默认渠道，核对后确认保存即可使用。' : '本网络暂不推荐免密钥默认渠道；现有配置继续保留，可在渠道来源中手动选择。'} 公共服务可能限流，请以完整扫描成功和接单状态为准。</p>
      <Button disabled={busy || !missing.length || rows.length >= 10} onClick={() => append(missing.slice(0, 10 - rows.length).map((preset, index) => fromPreset(preset, Math.min(1000, nextPriority + index))))}>补充免费默认渠道</Button>
    </div> : null}
    <section className="deposit-priority-preview" aria-label="主备尝试顺序">
      <strong>主备尝试顺序 · 保存后生效</strong>
      <ol>{ordered.map(({ row }, index) => <li key={row.uid}><span>{index === 0 ? '首选' : `备用 ${index}`}</span><strong>{row.name || '未命名渠道'}</strong><span>优先级 {row.priority}</span></li>)}</ol>
      {!ordered.length ? <p>尚无启用的渠道与凭据。</p> : null}
      <p className="muted">小数值优先，同优先级按保存顺序；冷却中的候选运行时会跳过。此处展示配置顺序，实际状态见渠道列表。</p>
    </section>
    <div className="deposit-provider-actions"><Button disabled={busy} onClick={() => setExpanded(rows.map(row => row.uid))}>展开全部渠道</Button><Button disabled={busy} onClick={() => setExpanded([])}>收起全部渠道</Button></div>
    <Collapse className="deposit-provider-editor" keepDOM motion={false} activeKey={expanded} onChange={value => setExpanded(Array.isArray(value) ? value : value ? [value] : [])}>
    {rows.map((row, index) => <Collapse.Panel key={row.uid} itemKey={row.uid} disabled={busy} header={<div className="deposit-channel-heading"><div><span className="deposit-channel-rank">渠道 {index + 1}</span><strong>{row.name || '未命名渠道'}</strong></div><Tag>{row.enabled ? `${row.keys.length} 个凭据 · 优先级 ${row.priority}` : '已停用'}</Tag></div>}>
      <fieldset disabled={busy} className="deposit-provider-draft" style={expanded.includes(row.uid) ? undefined : { display: 'none' }}>
      <legend>渠道 {index + 1}</legend>
      <label>渠道名称<AdminTextInput ariaLabel={`渠道 ${index + 1} 名称`} value={row.name} onChange={name => update(row.uid, { name })} /></label>
      <label className="deposit-switch-row">启用渠道<Switch aria-label={`启用渠道 ${index + 1}`} disabled={busy} checked={row.enabled} onChange={value => update(row.uid, { enabled: value, ...(row.preset_id && value ? { keys: row.keys.map(key => ({ ...key, enabled: true })) } : {}) })} /></label>
      {presets.length ? <><label id={`preset-${row.uid}`}>渠道 {index + 1} 来源</label><AdminSelect motion={false} ariaLabel={`渠道 ${index + 1} 来源`} ariaLabelledBy={`preset-${row.uid}`} disabled={busy} value={row.preset_id} optionList={[{ value: '', label: '自定义 RPC / 自有 API Key' }, ...presets.map(preset => ({ value: preset.id, label: `${preset.name}${preset.recommended ? '（默认）' : ''}` }))]} onChange={value => selectPreset(row, value)} /></> : null}
      {row.preset_id ? <div className="deposit-preset">
        <strong>内置免费渠道 · 免密钥</strong>
        <p className="deposit-public-endpoint">{presets.find(preset => preset.id === row.preset_id)?.endpoint}</p>
        <p>{presets.find(preset => preset.id === row.preset_id)?.note}</p>
        <a href={presets.find(preset => preset.id === row.preset_id)?.source} target="_blank" rel="noopener noreferrer">查看渠道官方说明</a>
        {row.keys.some(key => key.configured) ? <p>保存为免费渠道时会清除本渠道原有密钥，仅使用免密钥访问。</p> : null}
      </div> : <>
        <label id={`preset-${row.uid}-custom`}>渠道 {index + 1} 服务商模板</label><AdminSelect motion={false} ariaLabel={`渠道 ${index + 1} 服务商模板`} ariaLabelledBy={`preset-${row.uid}-custom`} disabled={busy} value={row.template_id ?? ''} optionList={[{ value: '', label: '选择模板填入地址' }, ...nodePresets.filter(preset => preset.network === scanner.network).map(preset => ({ value: preset.id, label: preset.label }))]} onChange={value => {
          const template = nodePresets.find(preset => preset.id === value && preset.network === scanner.network);
          if (template && value !== row.template_id) { const next = fromTemplate(template, parseSafeInteger(row.priority, 0, 1000) ?? 0); update(row.uid, { template_id: next.template_id, endpoint: next.endpoint, name: next.name, interval: next.interval, keys: next.keys }); }
          else if (!value) update(row.uid, { template_id: '' });
        }} />
        {row.template_id ? <><p>{nodePresets.find(preset => preset.id === row.template_id)?.note}</p><p>已为所选模板创建空白凭据。请填写该服务商的 Key；确认保存后替换本渠道原有凭据。</p></> : null}
        <label>节点基础地址<AdminPasswordInput ariaLabel={`渠道 ${index + 1} 节点地址`} value={row.endpoint} placeholder={row.id ? '已配置；留空保留原值' : 'https://'} onChange={endpoint => update(row.uid, { endpoint, template_id: '' })} /></label>
        <p className="muted">URL 鉴权可在路径或查询中使用 {'{API_KEY}'}，系统将填入当前凭据并编码。TronGrid 使用基础地址和下方 Key 请求头，无需占位符。</p>
      </>}
      <div className="deposit-provider-fields"><label>优先级（小数值优先）<AdminTextInput ariaLabel={`渠道 ${index + 1} 优先级`} value={row.priority} onChange={priority => update(row.uid, { priority })} /></label><label>渠道请求间隔（毫秒）<AdminTextInput ariaLabel={`渠道 ${index + 1} 请求间隔`} value={row.interval} onChange={interval => update(row.uid, { interval, ...(row.preset_id ? { keys: row.keys.map(key => ({ ...key, interval })) } : {}) })} /></label></div>
      {!row.preset_id ? row.keys.map((key, k) => <div className="deposit-key-draft" key={key.uid}>
        <label>凭据名称<AdminTextInput ariaLabel={`渠道 ${index + 1} 凭据 ${k + 1} 名称`} value={key.name} onChange={name => updateKey(row, key.uid, { name })} /></label>
        <label>API Key<AdminPasswordInput ariaLabel={`渠道 ${index + 1} 凭据 ${k + 1} API Key`} disabled={busy || key.clear_api_key} value={key.api_key} placeholder={key.configured ? '已配置；留空保留原值' : '无鉴权节点可留空'} onChange={api_key => updateKey(row, key.uid, { api_key })} /></label>
        <label className="deposit-switch-row">启用凭据<Switch aria-label={`启用渠道 ${index + 1} 凭据 ${k + 1}`} disabled={busy} checked={key.enabled} onChange={enabled => updateKey(row, key.uid, { enabled })} /></label>
        {key.configured ? <label className="deposit-switch-row">清除已有密钥<Switch aria-label={`清除渠道 ${index + 1} 凭据 ${k + 1} 密钥`} disabled={busy} checked={key.clear_api_key} onChange={clear_api_key => updateKey(row, key.uid, { clear_api_key, api_key: '' })} /></label> : null}
        <div className="deposit-provider-fields"><label>凭据优先级<AdminTextInput ariaLabel={`渠道 ${index + 1} 凭据 ${k + 1} 优先级`} value={key.priority} onChange={priority => updateKey(row, key.uid, { priority })} /></label><label>凭据请求间隔（毫秒）<AdminTextInput ariaLabel={`渠道 ${index + 1} 凭据 ${k + 1} 请求间隔`} value={key.interval} onChange={interval => updateKey(row, key.uid, { interval })} /></label></div>
        <Button disabled={busy || row.keys.length === 1} onClick={() => update(row.uid, { keys: row.keys.filter(item => item.uid !== key.uid) })}>移除凭据 {k + 1}</Button>
      </div>) : null}
      <div className="deposit-provider-actions">{!row.preset_id ? <Button disabled={busy || row.keys.length >= 10} onClick={() => update(row.uid, { keys: [...row.keys, newKey(row.keys.length)] })}>添加 API Key · 渠道 {index + 1}</Button> : null}<Button type="danger" disabled={busy || rows.length === 1} onClick={() => { dirty(); setRows(current => current.filter(item => item.uid !== row.uid)); }}>移除渠道 {index + 1}</Button></div>
    </fieldset></Collapse.Panel>)}
    </Collapse>
    <Button disabled={busy || rows.length >= 10} onClick={() => append([newProvider(nextPriority)])}>添加备用渠道</Button>
    <p>免密钥公共渠道无需 Key；免费账号模板需自有 Key。原有渠道输入框留空保留已有密钥，切换服务商模板需重新填写。移除和启停在确认保存后生效。原扫描进度不变，所有渠道不可用时停止新订单准入，恢复后继续补扫。</p>
    {error ? <Banner type="warning" description={error} /> : null}
    <ConfirmAction modalWidth="min(448px, calc(100vw - 32px))" actionText="保存监听配置" title="保存主备渠道与凭据" description={`将保存 ${rows.length} 个渠道、${rows.reduce((sum, row) => sum + row.keys.length, 0)} 个凭据槽及网络接单设置。被移除的配置不再参与监听，原扫描进度保留。`} disabled={busy || Boolean(error)} onConfirm={async reason => {
      if (error || count === null) throw new Error(error || '确认数无效');
      const providers = payload(rows, presets);
      setBusy(true);
      try { await apiRequest(`${base}/deposit-settings/scanners/${scanner.network}`, { method: 'PUT', body: JSON.stringify({ enabled, endpoint: null, api_key: null, confirmations: count, expected_revision: scanner.revision, providers, reason }) }); await saved(); } finally { setBusy(false); }
    }} />
  </div>;
}
