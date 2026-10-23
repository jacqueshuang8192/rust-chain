import { Button, Card, Space, Switch, Typography } from '@douyinfe/semi-ui';
import { useQuery } from '@tanstack/react-query';
import { useState, useSyncExternalStore } from 'react';
import { Link } from 'react-router-dom';
import { apiRequest } from '../../api/client';
import { authStore } from '../../auth/authStore';
import { PageHeader } from '../../layouts/PageHeader';
import { adminErrorMessage } from '../../shared/adminErrorMessage';
import { ConfirmAction } from '../../shared/ConfirmAction';
import { AdminPasswordInput, AdminSelect, AdminTextInput } from '../../shared/SemiFormControls';
import { parseSafeInteger } from '../../shared/integer';
import { TimestampText } from '../../shared/TimestampText';
import { AdminRequestActionBoundary, useCanAdminRequest } from '../access';
import { UnsavedChangesGuard } from '../settings';
import { base, networkOptions, parseInventory, parseSettings, type Asset, type Inventory, type Scanner, type Settings } from './contracts';

export function DepositSettingsPage() {
  const session = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('admin'));
  const query = useQuery({ queryKey: ['timed-deposit-settings', session?.generation], queryFn: async () => parseSettings(await apiRequest<unknown>(`${base}/deposit-settings`)), enabled: Boolean(session), refetchOnWindowFocus: false });
  return <><PageHeader title="限时链上充值配置" description="每个网络维护可扩充的共享地址池。相同金额占用一个组合 30 分钟；不同金额可共用地址。只有配置完整且监听追上区块的资产才会向用户开放。" />
    {query.error ? <p role="alert">{adminErrorMessage(query.error)}</p> : null}
    {query.data ? <SettingsEditor key={`${session?.generation}:${query.dataUpdatedAt}`} data={query.data} reload={async () => { await query.refetch(); }} /> : <p>正在读取配置…</p>}
  </>;
}

function SettingsEditor({ data, reload }: { data: Settings; reload: () => Promise<void> }) {
  const [network, setNetwork] = useState('eth');
  const [editing, setEditing] = useState<string | null>(null);
  const scanner = data.scanners.find((row) => row.network === network);
  return <div className="page-stack"><UnsavedChangesGuard enabled={editing !== null} />
    <Space wrap><AdminSelect ariaLabel="配置网络" value={network} onChange={setNetwork} optionList={networkOptions} disabled={editing !== null} />
      <Button onClick={() => { setEditing(null); void reload(); }}>{editing ? '放弃草稿并重新载入' : '刷新配置与监听状态'}</Button>
      <Link to="/admin/wallet/deposit-address-pool">维护或批量添加地址</Link><Link to="/admin/wallet/deposit-receipts">到账核对</Link></Space>
    {scanner ? <ScannerEditor key={network} scanner={scanner} disabled={editing !== null && editing !== 'scanner'} dirty={() => setEditing('scanner')} saved={reload} /> : null}
    <AssetEditor key={`asset:${network}`} network={network} assets={data.assets.filter((row) => row.network === network)} disabled={editing !== null && editing !== 'asset'} dirty={() => setEditing('asset')} saved={reload} />
    <AddressEditor network={network} data={data} disabled={editing !== null} saved={reload} />
  </div>;
}
function ScannerEditor({ scanner, disabled, dirty, saved }: { scanner: Scanner; disabled: boolean; dirty: () => void; saved: () => Promise<void> }) {
  const [enabled, setEnabled] = useState(scanner.enabled);
  const [endpoint, setEndpoint] = useState(''); const [key, setKey] = useState(''); const [confirmations, setConfirmations] = useState(String(scanner.confirmations));
  const [busy, setBusy] = useState(false);
  const count = parseSafeInteger(confirmations, 1, 10000);
  const url = `${base}/deposit-settings/scanners/${scanner.network}`;
  return <Card title="只读监听节点"><fieldset disabled={disabled || busy} className="admin-action-form" style={{ border: 0, padding: 0 }}>
    <label>启用监听<Switch checked={enabled} onChange={(value) => { dirty(); setEnabled(value); }} /></label>
    <label>节点地址（留空保留；已配置：{scanner.endpoint_configured ? '是' : '否'}）<AdminPasswordInput ariaLabel="监听节点地址" value={endpoint} onChange={(value) => { dirty(); setEndpoint(value); }} /></label>
    <label>节点 API 密钥（留空保留）<AdminPasswordInput ariaLabel="节点 API 密钥" value={key} onChange={(value) => { dirty(); setKey(value); }} /></label>
    <label>确认数 / Solana finalized slot 深度<AdminTextInput ariaLabel="充值确认数" value={confirmations} onChange={(value) => { dirty(); setConfirmations(value); }} /></label>
    <p>最近追上区块：<TimestampText value={scanner.last_scan_at} />。{scanner.last_error ? `监听异常：${scanner.last_error}` : '请结合最近成功时间判断可用状态。'}</p>
    <p>Ethereum / Base 原生 ETH 需要 callTracer；Bitcoin 使用 Core RPC；Tron 使用 FullNode TRC20 日志；Solana 使用完整 finalized 区块与经典 SPL Token。</p>
    <AdminRequestActionBoundary endpoint={url} method="PUT"><ConfirmAction actionText="保存监听配置" title={`更新 ${scanner.network} 监听配置`} disabled={disabled || busy || count === null || (enabled && !scanner.endpoint_configured && !endpoint.trim())} description="保存后暂停新订单，直到监听重新追上区块。原游标与历史订单保留。" onConfirm={async (reason) => {
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
  function edit(asset: Asset) { dirty(); setSymbol(asset.asset_symbol); setKind(asset.asset_kind); setToken(asset.token_address); setDecimals(String(asset.decimals)); setEnabled(asset.enabled); }
  return <Card title="链上资产映射"><Space wrap>{assets.map((asset) => <Button key={asset.id} disabled={disabled || busy} onClick={() => edit(asset)}>{asset.asset_symbol} · {asset.enabled ? '启用' : '停用'}</Button>)}</Space>
    <fieldset disabled={disabled || busy} className="admin-action-form" style={{ border: 0, padding: '16px 0 0' }}>
      <label>平台币种代码<AdminTextInput ariaLabel="平台币种代码" value={symbol} onChange={(value) => { dirty(); setSymbol(value); }} /></label>
      <label>链上类型<AdminSelect ariaLabel="链上资产类型" value={kind} disabled={Boolean(existing)} onChange={(value) => { dirty(); setKind(value); setToken(''); }} optionList={[{ value: 'native', label: '原生币（Tron 暂不支持）' }, { value: 'token', label: '代币合约 / SPL mint' }]} /></label>
      <label>合约地址 / mint（原生币留空）<AdminTextInput ariaLabel="合约地址或 mint" value={token} disabled={Boolean(existing) || kind === 'native'} onChange={(value) => { dirty(); setToken(value); }} /></label>
      <label>链上 decimals<AdminTextInput ariaLabel="链上精度" value={decimals} disabled={Boolean(existing)} onChange={(value) => { dirty(); setDecimals(value); }} /></label>
      <label>开放该资产<Switch checked={enabled} onChange={(value) => { dirty(); setEnabled(value); }} /></label>
      <p>合约和精度创建后不可改写。平台资产、网络白名单和地址范围也必须允许该币种。节点会核对代币精度。</p>
      <AdminRequestActionBoundary endpoint={url} method="PUT"><ConfirmAction actionText="保存资产映射" title={`保存 ${symbol || '币种'} · ${network} 链上身份`} disabled={disabled || busy || !symbol.trim() || scale === null || (kind === 'token' && !token.trim())} description={`类型：${kind}；合约：${token || '原生币'}；精度：${decimals}。请核对真实主网身份。`} onConfirm={async (reason) => {
        if (scale === null) throw new Error('链上精度必须为 0 至 18 的整数');
        setBusy(true); try { await apiRequest(url, { method: 'PUT', body: JSON.stringify({ network, asset_symbol: symbol.trim().toUpperCase(), asset_kind: kind, token_address: token.trim(), decimals: scale, enabled, expected_revision: existing?.revision || 0, reason }) }); await saved(); } finally { setBusy(false); }
      }} /></AdminRequestActionBoundary>
    </fieldset>
  </Card>;
}
function AddressEditor({ network, data, disabled, saved }: { network: string; data: Settings; disabled: boolean; saved: () => Promise<void> }) {
  const [id, setId] = useState(''); const [inventory, setInventory] = useState<Inventory | null>(null); const [error, setError] = useState(''); const [busy, setBusy] = useState(false);
  const canRead = useCanAdminRequest('/admin/api/v1/deposit-address-pool', 'GET');
  const selectedId = parseSafeInteger(id, 1);
  const selected = inventory && inventory.id === selectedId && inventory.network === network ? inventory : null;
  return <Card title={`共享地址池 · ${data.addresses.filter((row) => row.network === network && row.enabled).length} 个启用地址`}>
    <p>先在充值地址池新增或批量导入真实地址，再按库存编号预览并纳入共享模式。历史永久分配的地址不得改为共享地址；数量没有 10 个的固定上限。</p>
    <Space wrap><AdminTextInput ariaLabel="地址库存编号" value={id} onChange={(value) => { setId(value); setInventory(null); }} disabled={disabled || busy || !canRead} />
      <Button disabled={disabled || busy || selectedId === null || !canRead} onClick={async () => { setBusy(true); setError(''); try { setInventory(parseInventory(await apiRequest(`/admin/api/v1/deposit-address-pool/${selectedId}`))); } catch (cause) { setError(adminErrorMessage(cause)); } finally { setBusy(false); } }}>预览库存地址</Button></Space>
    {error ? <p role="alert">{error}</p> : null}
    {selected ? <div style={{ overflowWrap: 'anywhere' }}><p>{selected.network} · {selected.address} · 库存状态：{selected.status}</p><AdminRequestActionBoundary endpoint={`${base}/deposit-settings/addresses/${selected.id}`} method="PUT">
      <ConfirmAction actionText="纳入共享地址池" title="确认共享收款地址" description={`${selected.network} · ${selected.address}。纳入后将由限时订单共享，不再永久绑定单个用户。`} disabled={disabled || selected.status !== 'available' || selected.assigned_user_id !== null || Boolean(selected.memo)} onConfirm={async (reason) => { await apiRequest(`${base}/deposit-settings/addresses/${selected.id}`, { method: 'PUT', body: JSON.stringify({ enabled: true, reason }) }); await saved(); }} />
    </AdminRequestActionBoundary></div> : inventory ? <p role="alert">库存网络与当前网络不一致，请切换网络后核对。</p> : null}
    {data.addresses.filter((row) => row.network === network).map((row) => <div key={row.pool_id} style={{ padding: '12px 0', overflowWrap: 'anywhere' }}><Typography.Text>{row.pool_id} · {row.address} · {row.enabled ? '启用' : '停用'}</Typography.Text> <AdminRequestActionBoundary endpoint={`${base}/deposit-settings/addresses/${row.pool_id}`} method="PUT"><ConfirmAction actionText={row.enabled ? '停用地址' : '启用地址'} title={`${row.enabled ? '停用' : '启用'} ${row.address}`} disabled={disabled} description="只影响新订单，已有订单及迟到款继续监听。" onConfirm={async (reason) => { await apiRequest(`${base}/deposit-settings/addresses/${row.pool_id}`, { method: 'PUT', body: JSON.stringify({ enabled: !row.enabled, reason }) }); await saved(); }} /></AdminRequestActionBoundary></div>)}
  </Card>;
}
