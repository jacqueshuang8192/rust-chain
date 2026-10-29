import { Banner, Button, Modal, SideSheet, Space, Switch, Typography } from '@douyinfe/semi-ui';
import { useState, type ReactNode } from 'react';
import { apiRequest } from '../../api/client';
import { ConfirmAction } from '../../shared/ConfirmAction';
import { AdminPasswordInput, AdminSelect, AdminTextArea, AdminTextInput } from '../../shared/SemiFormControls';
import { parseSafeInteger } from '../../shared/integer';
import { createModalProps } from '../resources/actions/shared';
import { UnsavedChangesGuard } from '../settings';
import { base, networkOptions, type Asset, type PlatformAsset, type RpcPreset, type Scanner } from './contracts';
import { MultiScannerForm } from './MultiScannerForm';
import { assetPresets, hasCredentialPlaceholder, nativeAssets, nodePresets } from './presets';

export const networkLabel = (network: string) => networkOptions.find(row => row.value === network)?.label ?? network;
type EditorControls = { dirty: () => void; busy: boolean; setBusy: (busy: boolean) => void };

/** 编辑器固定打开时的配置快照；取消、浏览器关闭和站内跳转均保护草稿。 */
export function DepositEditPanel({ title, close, children, className }: { className?: string; title: string; close: () => void; children: (controls: EditorControls) => ReactNode }) {
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(false);
  const [discard, setDiscard] = useState(false);
  const cancel = () => { if (!busy) { if (dirty) setDiscard(true); else close(); } };
  const modalProps = createModalProps('medium');
  return <>
    <UnsavedChangesGuard enabled={dirty || busy} />
    <SideSheet {...modalProps} className={[modalProps.className, className].filter(Boolean).join(' ')} title={title} visible motion={false} onCancel={cancel} closable={!busy} closeOnEsc={!busy}
      footer={<Button disabled={busy} onClick={cancel}>取消编辑</Button>}>
      {children({ dirty: () => setDirty(true), busy, setBusy })}
    </SideSheet>
    <Modal width="min(448px, calc(100vw - 32px))" visible={discard} title="放弃未保存的配置？" motion={false} maskClosable={false} okText="放弃草稿" cancelText="继续编辑" okButtonProps={{ 'aria-label': '放弃草稿' }} cancelButtonProps={{ 'aria-label': '继续编辑' }}
      onOk={close} onCancel={() => setDiscard(false)}>
      当前编辑内容尚未保存。放弃后可重新打开配置，或选择其他网络。
    </Modal>
  </>;
}

type CommonProps = EditorControls & { saved: () => Promise<void> };
export function ScannerForm(props: CommonProps & { scanner: Scanner; presets?: RpcPreset[] }) {
  return props.scanner.providers.length || props.presets?.some(preset => preset.network === props.scanner.network) ? <MultiScannerForm {...props} /> : <InitialScannerForm {...props} />;
}
function InitialScannerForm({ scanner, dirty, busy, setBusy, saved }: CommonProps & { scanner: Scanner }) {
  const [enabled, setEnabled] = useState(scanner.enabled);
  const [endpoint, setEndpoint] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [confirmations, setConfirmations] = useState(String(scanner.confirmations));
  const [presetId, setPresetId] = useState('');
  const [providerKey, setProviderKey] = useState('');
  const presets = nodePresets.filter(row => row.network === scanner.network && (!row.account || row.legacy));
  const preset = presets.find(row => row.id === presetId);
  const count = parseSafeInteger(confirmations, 1, 10000);
  const error = hasCredentialPlaceholder(endpoint) ? '请将节点地址中的 API_KEY 占位符替换为自己的真实密钥' : count === null ? '确认数必须为 1 至 10000 的整数' : enabled && !scanner.endpoint_configured && !endpoint.trim() ? '开启新订单前，请填写监听节点地址' : '';
  return <div className="deposit-edit-form">
    {presets.length ? <div className="deposit-preset">
      <label id="node-preset-label">主网节点预设</label>
      <AdminSelect motion={false} ariaLabel="主网节点预设" ariaLabelledBy="node-preset-label" disabled={busy} value={presetId} optionList={[{ value: '', label: '手动配置 / 保留已有节点' }, ...presets.map(row => ({ value: row.id, label: row.label }))]} onChange={value => {
        setPresetId(value); setProviderKey('');
        const next = presets.find(row => row.id === value);
        if (next) { dirty(); setEndpoint(next.endpoint); setConfirmations(String(next.confirmations)); }
      }} />
      <p className="muted">选择后只填入节点地址和确认深度，可继续编辑；不会自动保存或切换接单状态。已有节点在确认保存后才被替换，密钥框留空仍保留原值。</p>
      {preset?.endpoint.includes('{API_KEY}') ? <label>服务商 API Key<AdminPasswordInput ariaLabel="服务商 API Key" disabled={busy} value={providerKey} placeholder="填写后自动生成节点地址" onChange={value => {
        dirty(); setProviderKey(value); setEndpoint(preset.endpoint.replace('{API_KEY}', value.trim() ? encodeURIComponent(value.trim()) : '{API_KEY}'));
      }} /><span className="muted">只用于当前预设的 URL 鉴权，保存时随节点地址加密；切换预设会清空此输入。</span></label> : null}
      {preset ? <><p>{preset.note}</p><a href={preset.source} target="_blank" rel="noopener noreferrer">查看节点官方说明</a></> : null}
    </div> : null}
    <fieldset disabled={busy}>
      <label className="deposit-switch-row">接受新充值订单<Switch aria-label="接受新充值订单" disabled={busy} checked={enabled} onChange={value => { dirty(); setEnabled(value); }} /></label>
      <label>监听节点地址<AdminPasswordInput ariaLabel="监听节点地址" value={endpoint} onChange={value => { dirty(); setPresetId(''); setProviderKey(''); setEndpoint(value); }} /></label>
      <p className="muted">{scanner.endpoint_configured ? '节点地址已配置，留空保留原值。' : '尚未配置节点地址。'} 密钥内容不会回显。</p>
      <label>节点 API 密钥<AdminPasswordInput ariaLabel="节点 API 密钥" value={apiKey} onChange={value => { dirty(); setApiKey(value); }} /></label>
      <p className="muted">{scanner.api_key_configured ? 'API 密钥已配置。' : 'API 密钥未配置。'} 留空保留原值。</p>
      <label>确认数 / Solana finalized slot 深度<AdminTextInput ariaLabel="充值确认数" value={confirmations} onChange={value => { dirty(); setConfirmations(value); }} /></label>
    </fieldset>
    <p>停止接单后仍监听已有订单。Ethereum 原生 ETH 需要节点支持 callTracer；Base 原生 ETH 通过区块与回执识别直接转账，不支持合约内部转账；Bitcoin 使用 Core RPC；Tron 监听 TRC20；Solana 监听 finalized 区块与经典 SPL Token。</p>
    {error ? <Banner type="warning" description={error} /> : null}
    <ConfirmAction actionText="保存监听配置" title={`更新 ${networkLabel(scanner.network)} 监听配置`} disabled={busy || Boolean(error)}
      description="保存后等待监听重新追上区块，再恢复接单。原监听进度保留。" onConfirm={async reason => {
        if (error || count === null) throw new Error(error || '确认数无效');
        setBusy(true);
        try {
          await apiRequest(`${base}/deposit-settings/scanners/${scanner.network}`, { method: 'PUT', body: JSON.stringify({ enabled, endpoint: endpoint.trim() || null, api_key: apiKey || null, confirmations: count, expected_revision: scanner.revision, reason }) });
          await saved();
        } finally { setBusy(false); }
      }} />
  </div>;
}

export function AssetForm({ network, assets, catalog, existing, dirty, busy, setBusy, saved }: CommonProps & { network: string; assets: Asset[]; catalog: PlatformAsset[]; existing?: Asset }) {
  const native = nativeAssets[network];
  const defaultKind = native && (network === 'btc' || !assets.some(row => row.asset_kind === 'native')) ? 'native' : 'token';
  const [kind, setKind] = useState(existing?.asset_kind ?? defaultKind);
  const [symbol, setSymbol] = useState(existing?.asset_symbol ?? (defaultKind === 'native' ? native?.symbol ?? '' : ''));
  const [token, setToken] = useState(existing?.token_address ?? '');
  const [decimals, setDecimals] = useState(String(existing?.decimals ?? (defaultKind === 'native' ? native?.decimals ?? '' : '')));
  const [enabled, setEnabled] = useState(existing?.enabled ?? true);
  const [presetId, setPresetId] = useState('');
  const presets = assetPresets.filter(row => row.network === network && !assets.some(asset => asset.asset_symbol === row.symbol));
  const preset = presets.find(row => row.id === presetId);
  const scale = parseSafeInteger(decimals, 0, 18);
  const platform = catalog.find(row => row.symbol === symbol);
  const error = !platform ? '请先选择已创建的平台币种' : !existing && assets.some(row => row.asset_symbol === symbol) ? '此币种已配置，请在列表中编辑' : scale === null ? '请填写 0 至 18 的链上精度（decimals）' : kind === 'token' && !token.trim() ? '请填写当前网络的真实合约地址或 mint' : '';
  function changeKind(value: string) {
    dirty(); setPresetId(''); setKind(value); setToken('');
    setSymbol(value === 'native' ? native?.symbol ?? '' : '');
    setDecimals(value === 'native' ? String(native?.decimals ?? '') : '');
  }
  return <div className="deposit-edit-form">
    {existing ? <Banner type="info" description="币种、链上类型、合约和精度在创建后固定，不能原地修改，以保护历史订单归属。这里可以启用或停用该币种。" /> : <p>一个网络可以添加多个监听币种。例如 Ethereum 可分别配置 ETH、USDT、USDC，并继续添加其他 ERC20 资产。</p>}
    {!existing && presets.length ? <div className="deposit-preset">
      <label id="asset-preset-label">主网币种预设</label>
      <AdminSelect motion={false} ariaLabel="主网币种预设" ariaLabelledBy="asset-preset-label" disabled={busy} value={presetId} optionList={[{ value: '', label: '手动配置币种' }, ...presets.map(row => {
        const available = catalog.some(asset => asset.symbol === row.symbol);
        return { value: row.id, label: `${row.label}${available ? '' : '（请先创建平台资产）'}`, disabled: !available };
      })]} onChange={value => {
        setPresetId(value);
        const next = presets.find(row => row.id === value);
        if (next && catalog.some(asset => asset.symbol === next.symbol)) { dirty(); setKind(next.kind); setSymbol(next.symbol); setToken(next.token); setDecimals(String(next.decimals)); }
      }} />
      <p className="muted">只填入当前主网的币种、类型、合约和精度，不自动保存。已配置币种不再列出；新币种仍可手动添加。</p>
      {preset ? <p>已填入 {preset.label} · 精度 {preset.decimals}。<a href={preset.source} target="_blank" rel="noopener noreferrer">核对官方资产身份</a></p> : null}
    </div> : null}
    <fieldset disabled={busy}>
      <label id="chain-kind-label">链上资产类型</label>
      {existing ? <p>{kind === 'native' ? '原生币' : '代币合约 / SPL mint'}</p> : <AdminSelect ariaLabel="链上资产类型" ariaLabelledBy="chain-kind-label" value={kind} onChange={changeKind} optionList={[
        ...(native ? [{ value: 'native', label: `原生币 ${native.symbol}` }] : []),
        ...(network !== 'btc' ? [{ value: 'token', label: network === 'solana' ? '经典 SPL Token' : network === 'tron' ? 'TRC20 代币' : 'ERC20 代币' }] : [])
      ]} />}
      <label id="chain-symbol-label">平台币种</label>
      {existing || kind === 'native' ? <p>{symbol} {kind === 'native' ? '· 原生币身份固定' : ''}</p> : <AdminSelect ariaLabel="平台币种" ariaLabelledBy="chain-symbol-label" filter value={symbol} onChange={value => { dirty(); setPresetId(''); setSymbol(value); setToken(''); setDecimals(''); }} placeholder="选择要监听的币种" optionList={catalog.filter(row => !assets.some(asset => asset.asset_symbol === row.symbol)).map(row => ({ value: row.symbol, label: `${row.symbol}${row.status !== 'active' || !row.deposit_enabled ? '（平台充值未开启）' : ''}` }))} />}
      {!existing ? <p className="muted">列表来自平台资产。若没有所需币种，请先在资产管理中添加，再刷新本页。</p> : null}
      {kind === 'native' ? <p>原生币无需填写合约地址，链上精度为 {decimals}。</p> : <>
        <label>合约地址 / mint{existing ? <Typography.Text copyable>{token}</Typography.Text> : <AdminTextInput ariaLabel="合约地址或 mint" value={token} onChange={value => { dirty(); setPresetId(''); setToken(value); }} />}</label>
        <label>链上精度（decimals）{existing ? <span>{decimals}</span> : <AdminTextInput ariaLabel="链上精度" value={decimals} onChange={value => { dirty(); setPresetId(''); setDecimals(value); }} placeholder="以链上合约为准" />}</label>
      </>}
      <label className="deposit-switch-row">开放该币种充值<Switch aria-label="开放该币种充值" disabled={busy} checked={enabled} onChange={value => { dirty(); setEnabled(value); }} /></label>
    </fieldset>
    {platform && (platform.status !== 'active' || !platform.deposit_enabled) ? <Banner type="warning" description="平台资产尚未启用或未开启充值。可以保存映射，但用户入口需等待平台资产和网络均准备完成。" /> : null}
    <p>该网络的所有启用地址共享这些币种。代币精度由节点再次核对；钱包精度为 {platform?.precision_scale ?? '—'}，用户可输入精度取钱包与链上精度中较小者。</p>
    {error ? <Banner type="warning" description={error} /> : null}
    <ConfirmAction actionText={existing ? '保存币种状态' : '添加监听币种'} title={`${existing ? '更新' : '添加'} ${symbol || '币种'} · ${networkLabel(network)}`} disabled={busy || Boolean(error)}
      description={`类型：${kind === 'native' ? '原生币' : '代币'}；合约：${token || '无'}；精度：${decimals || '未填写'}。请核对真实主网身份；保存后等待监听追上区块。`} onConfirm={async reason => {
        if (error || scale === null) throw new Error(error || '链上精度无效');
        setBusy(true);
        try {
          await apiRequest(`${base}/deposit-settings/assets`, { method: 'PUT', body: JSON.stringify({ network, asset_symbol: symbol, asset_kind: kind, token_address: token.trim(), decimals: scale, enabled, expected_revision: existing?.revision ?? 0, reason }) });
          await saved();
        } finally { setBusy(false); }
      }} />
  </div>;
}

export function AddressImportForm({ network, dirty, busy, setBusy, saved }: CommonProps & { network: string }) {
  const [text, setText] = useState('');
  const addresses = text.split(/\r?\n/).map(value => value.trim()).filter(Boolean);
  const error = addresses.length === 0 ? '请每行填写一个收款地址' : addresses.length > 500 ? '单次最多添加 500 个地址' : '';
  return <div className="deposit-edit-form">
    <p>向 {networkLabel(network)} 添加真实主网地址。可先添加 10 个，后续随时扩容。</p>
    <fieldset disabled={busy}><label>收款地址（每行一个）<AdminTextArea ariaLabel="收款地址批量输入" value={text} onChange={value => { dirty(); setText(value); }} placeholder="粘贴当前网络的收款地址" /></label></fieldset>
    <p>已输入 {addresses.length} 个。重复地址自动跳过，已停用地址不会重新启用。任一地址无效则整批不保存。</p>
    {error ? <Banner type="warning" description={error} /> : null}
    <Space><ConfirmAction actionText="确认添加地址" title={`向 ${networkLabel(network)} 添加收款地址`} disabled={busy || Boolean(error)} description={`确认地址：${addresses.join('；')}`} onConfirm={async reason => {
      if (error) throw new Error(error);
      setBusy(true);
      try { await apiRequest(`${base}/deposit-settings/addresses`, { method: 'POST', body: JSON.stringify({ network, addresses, reason }) }); await saved(); }
      finally { setBusy(false); }
    }} /></Space>
  </div>;
}
