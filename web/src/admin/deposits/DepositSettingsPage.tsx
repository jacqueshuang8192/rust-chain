import { Button, Card, Space, Switch, Typography } from '@douyinfe/semi-ui';
import { useQuery } from '@tanstack/react-query';
import { useState, useSyncExternalStore } from 'react';
import { apiRequest } from '../../api/client';
import { authStore } from '../../auth/authStore';
import { PageHeader } from '../../layouts/PageHeader';
import { adminErrorMessage } from '../../shared/adminErrorMessage';
import { ConfirmAction } from '../../shared/ConfirmAction';
import { AdminPasswordInput, AdminSelect, AdminTextInput, AdminTextArea } from '../../shared/SemiFormControls';
import { parseSafeInteger } from '../../shared/integer';
import { TimestampText } from '../../shared/TimestampText';
import { AdminRequestActionBoundary, useCanAdminRequest } from '../access';
import { UnsavedChangesGuard } from '../settings';
import { base, networkOptions, parseAddresses, parseSettings, type Asset, type Scanner, type Settings } from './contracts';

export function DepositSettingsPage() {
  const session = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('admin'));
  const canConfigure = useCanAdminRequest(`${base}/deposit-settings`, 'GET');
  const [network, setNetwork] = useState('eth');
  const query = useQuery({ queryKey: ['timed-deposit-settings', session?.generation, canConfigure], queryFn: async () => canConfigure ? parseSettings(await apiRequest<unknown>(`${base}/deposit-settings`)) : { scanners: [], assets: [], addresses: parseAddresses(await apiRequest<unknown>(`${base}/deposit-settings/addresses`)) }, enabled: Boolean(session), refetchOnWindowFocus: false });
  return <main className="exchange-page admin-action-page"><PageHeader title="链上充值" description="每个网络维护可扩充的共享地址池。相同金额占用一个组合 30 分钟；不同金额可共用地址。只有配置完整且监听追上区块的资产才会向用户开放。" />
    {query.error ? <p role="alert">{adminErrorMessage(query.error)}</p> : null}
    {query.data ? <SettingsEditor key={`${session?.generation}:${query.dataUpdatedAt}`} data={query.data} network={network} setNetwork={setNetwork} canConfigure={canConfigure} reload={async () => { await query.refetch(); }} /> : query.isPending ? <p>正在读取配置…</p> : <Button onClick={() => { void query.refetch(); }}>重新读取配置</Button>}
  </main>;
}

function SettingsEditor({ data, reload, network, setNetwork, canConfigure }: { data: Settings; reload: () => Promise<void>; network: string; setNetwork: (network: string) => void; canConfigure: boolean }) {
  const [editing, setEditing] = useState<string | null>(null);
  const scanner = data.scanners.find((row) => row.network === network);
  return <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr)', gap: 16, minWidth: 0 }}><UnsavedChangesGuard enabled={editing !== null} />
    <div className="deposit-network-toolbar"><div><label id="deposit-network-label">配置网络</label><AdminSelect ariaLabel="配置网络" ariaLabelledBy="deposit-network-label" value={network} onChange={setNetwork} optionList={networkOptions} disabled={editing !== null} /></div>
      <Button onClick={() => { setEditing(null); void reload(); }}>{editing ? '放弃草稿并重新载入' : '刷新配置与监听状态'}</Button>
      </div>
    {scanner ? <ScannerEditor key={network} scanner={scanner} disabled={editing !== null && editing !== 'scanner'} dirty={() => setEditing('scanner')} saved={reload} /> : null}
    {canConfigure ? <AssetEditor key={`asset:${network}`} network={network} assets={data.assets.filter((row) => row.network === network)} disabled={editing !== null && editing !== 'asset'} dirty={() => setEditing('asset')} saved={reload} /> : null}
    <AddressEditor key={`addresses:${network}`} network={network} data={data} disabled={editing !== null && editing !== 'address'} dirty={() => setEditing('address')} saved={reload} />
  </div>;
}
function ScannerEditor({ scanner, disabled, dirty, saved }: { scanner: Scanner; disabled: boolean; dirty: () => void; saved: () => Promise<void> }) {
  const [enabled, setEnabled] = useState(scanner.enabled);
  const [endpoint, setEndpoint] = useState(''); const [key, setKey] = useState(''); const [confirmations, setConfirmations] = useState(String(scanner.confirmations));
  const [busy, setBusy] = useState(false);
  const count = parseSafeInteger(confirmations, 1, 10000);
  const url = `${base}/deposit-settings/scanners/${scanner.network}`;
  const canWrite = useCanAdminRequest(url, 'PUT');
  return <Card title="网络与监听节点"><fieldset disabled={disabled || busy || !canWrite} className="admin-action-form" style={{ border: 0, padding: 0, minWidth: 0 }}>
    <label>接受新充值订单<Switch checked={enabled} onChange={(value) => { dirty(); setEnabled(value); }} /></label>
    <label>节点地址（留空保留；已配置：{scanner.endpoint_configured ? '是' : '否'}）<AdminPasswordInput ariaLabel="监听节点地址" value={endpoint} onChange={(value) => { dirty(); setEndpoint(value); }} /></label>
    <label>节点 API 密钥（留空保留）<AdminPasswordInput ariaLabel="节点 API 密钥" value={key} onChange={(value) => { dirty(); setKey(value); }} /></label>
    <label>确认数 / Solana finalized slot 深度<AdminTextInput ariaLabel="充值确认数" value={confirmations} onChange={(value) => { dirty(); setConfirmations(value); }} /></label>
    <p>最近追上区块：<TimestampText value={scanner.last_scan_at} />。{scanner.last_error ? `监听异常：${scanner.last_error}` : '请结合最近成功时间判断可用状态。'}</p>
    <p>Ethereum / Base 原生 ETH 需要 callTracer；Bitcoin 使用 Core RPC；Tron 使用 FullNode TRC20 日志；Solana 使用完整 finalized 区块与经典 SPL Token。</p>
    <AdminRequestActionBoundary endpoint={url} method="PUT"><ConfirmAction actionText="保存监听配置" title={`更新 ${scanner.network} 监听配置`} disabled={disabled || busy || count === null || (enabled && !scanner.endpoint_configured && !endpoint.trim())} description="保存后暂停新订单，直到监听重新追上区块。停止收款后仍监听已创建订单；原游标保留。" onConfirm={async (reason) => {
      if (count === null) throw new Error('确认数必须为 1 至 10000 的整数');
      setBusy(true); try { await apiRequest(url, { method: 'PUT', body: JSON.stringify({ enabled, endpoint: endpoint.trim() || null, api_key: key || null, confirmations: count, expected_revision: scanner.revision, reason }) }); await saved(); } finally { setBusy(false); }
    }} /></AdminRequestActionBoundary>
  </fieldset></Card>;
}
function AssetEditor({ network, assets, disabled, dirty, saved }: { network: string; assets: Asset[]; disabled: boolean; dirty: () => void; saved: () => Promise<void> }) {
  const [symbol, setSymbol] = useState(''); const [kind, setKind] = useState('token'); const [token, setToken] = useState(''); const [decimals, setDecimals] = useState('6'); const [enabled, setEnabled] = useState(true); const [busy, setBusy] = useState(false);
  const existing = assets.find((row) => row.asset_symbol === symbol.trim().toUpperCase());
  const scale = parseSafeInteger(decimals, 0, 18);
  const url = `${base}/deposit-settings/assets`;
  const canWrite = useCanAdminRequest(url, 'PUT');
  function edit(asset: Asset) { dirty(); setSymbol(asset.asset_symbol); setKind(asset.asset_kind); setToken(asset.token_address); setDecimals(String(asset.decimals)); setEnabled(asset.enabled); }
  return <Card title="链上资产映射"><Space wrap>{assets.map((asset) => <Button key={asset.id} disabled={disabled || busy} onClick={() => edit(asset)}>{asset.asset_symbol} · {asset.enabled ? '启用' : '停用'}</Button>)}</Space>
    <fieldset disabled={disabled || busy || !canWrite} className="admin-action-form" style={{ border: 0, padding: '16px 0 0', minWidth: 0 }}>
      <label>平台币种代码<AdminTextInput ariaLabel="平台币种代码" value={symbol} onChange={(value) => { dirty(); setSymbol(value); }} /></label>
      <label>链上类型<AdminSelect ariaLabel="链上资产类型" value={kind} disabled={Boolean(existing)} onChange={(value) => { dirty(); setKind(value); setToken(''); }} optionList={[{ value: 'native', label: '原生币（Tron 暂不支持）' }, { value: 'token', label: '代币合约 / SPL mint' }]} /></label>
      <label>合约地址 / mint（原生币留空）<AdminTextInput ariaLabel="合约地址或 mint" value={token} disabled={Boolean(existing) || kind === 'native'} onChange={(value) => { dirty(); setToken(value); }} /></label>
      <label>链上 decimals<AdminTextInput ariaLabel="链上精度" value={decimals} disabled={Boolean(existing)} onChange={(value) => { dirty(); setDecimals(value); }} /></label>
      <label>开放该资产<Switch checked={enabled} onChange={(value) => { dirty(); setEnabled(value); }} /></label>
      <p>合约和精度创建后不可改写。平台资产须开启充值。该网络启用的全部收款地址均可接收这里开放的币种。节点会核对代币精度。</p>
      <AdminRequestActionBoundary endpoint={url} method="PUT"><ConfirmAction actionText="保存资产映射" title={`保存 ${symbol || '币种'} · ${network} 链上身份`} disabled={disabled || busy || !symbol.trim() || scale === null || (kind === 'token' && !token.trim())} description={`类型：${kind}；合约：${token || '原生币'}；精度：${decimals}。请核对真实主网身份。`} onConfirm={async (reason) => {
        if (scale === null) throw new Error('链上精度必须为 0 至 18 的整数');
        setBusy(true); try { await apiRequest(url, { method: 'PUT', body: JSON.stringify({ network, asset_symbol: symbol.trim().toUpperCase(), asset_kind: kind, token_address: token.trim(), decimals: scale, enabled, expected_revision: existing?.revision || 0, reason }) }); await saved(); } finally { setBusy(false); }
      }} /></AdminRequestActionBoundary>
    </fieldset>
  </Card>;
}
function AddressEditor({ network, data, disabled, dirty, saved }: { network: string; data: Settings; disabled: boolean; dirty: () => void; saved: () => Promise<void> }) {
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const addresses = text.split(/\r?\n/).map((value) => value.trim()).filter(Boolean);
  const rows = data.addresses.filter((row) => row.network === network);
  const canWrite = useCanAdminRequest(`${base}/deposit-settings/addresses`, 'POST');
  return <Card title={`收款地址 · ${rows.filter((row) => row.enabled).length} 个启用 / 共 ${rows.length} 个`}>
    <p>每行一个真实主网地址，可先添加 10 个，后续随时扩容。地址不会永久分配给用户；网络、币种、地址和精确金额的组合占用 30 分钟。</p>
    {canWrite ? <fieldset disabled={disabled || busy || !canWrite} className="admin-action-form" style={{ border: 0, padding: 0, minWidth: 0 }}>
      <label>添加收款地址（每行一个，单次最多 500 个）<AdminTextArea ariaLabel="收款地址批量输入" value={text} onChange={(value) => { dirty(); setText(value); }} placeholder="粘贴当前网络的一个或多个收款地址" /></label>
      <p>已输入 {addresses.length} 个。重复地址自动跳过，已停用地址不会被重新启用。任一地址无效则整批不保存。</p>
      <ConfirmAction actionText="添加收款地址" title={`向 ${networkOptions.find((row) => row.value === network)?.label} 添加地址`} disabled={disabled || busy || addresses.length === 0 || addresses.length > 500} description={`确认向当前网络添加以下地址：${addresses.join('；')}`}  onConfirm={async (reason) => {
        if (!addresses.length || addresses.length > 500) throw new Error('每次请提供 1 至 500 个地址');
        setBusy(true);
        try { await apiRequest(`${base}/deposit-settings/addresses`, { method: 'POST', body: JSON.stringify({ network, addresses, reason }) }); await saved(); }
        finally { setBusy(false); }
      }} />
    </fieldset> : null}
    {!rows.length ? <p>当前网络尚无收款地址，添加地址并完成监听后即可开放充值。</p> : null}
    {rows.map((row) => <div key={row.pool_id} style={{ padding: '12px 0', overflowWrap: 'anywhere' }}>
      <Typography.Text copyable>{row.address}</Typography.Text> · {row.enabled ? '启用' : '停用'} {' '}
      <AdminRequestActionBoundary endpoint={`${base}/deposit-settings/addresses/${row.pool_id}`} method="PUT">
        <ConfirmAction actionText={row.enabled ? '停用地址' : '启用地址'} title={`${row.enabled ? '停用' : '启用'} ${row.address}`} disabled={disabled || busy || Boolean(text)} description="只影响新订单，已有订单及迟到款继续监听。" onConfirm={async (reason) => {
          setBusy(true); try { await apiRequest(`${base}/deposit-settings/addresses/${row.pool_id}`, { method: 'PUT', body: JSON.stringify({ enabled: !row.enabled, reason }) }); await saved(); } finally { setBusy(false); }
        }} />
      </AdminRequestActionBoundary>
    </div>)}
  </Card>;
}
