import { Banner, Button, Card, Space, Tag, Typography } from '@douyinfe/semi-ui';
import { useQuery } from '@tanstack/react-query';
import { useState, useSyncExternalStore } from 'react';
import { apiRequest } from '../../api/client';
import { authStore } from '../../auth/authStore';
import { PageHeader } from '../../layouts/PageHeader';
import { adminErrorMessage } from '../../shared/adminErrorMessage';
import { ConfirmAction } from '../../shared/ConfirmAction';
import { AdminSelect } from '../../shared/SemiFormControls';
import { TimestampText } from '../../shared/TimestampText';
import { AdminRequestActionBoundary, useCanAdminRequest } from '../access';
import { AddressImportForm, AssetForm, DepositEditPanel, networkLabel, ScannerForm } from './DepositSettingsEditors';
import { base, networkOptions, parseAddresses, parseSettings, type Asset, type Scanner, type Settings, type SharedAddress } from './contracts';
import './RechargeWorkspace.css';

type Editing = { type: 'scanner'; scanner: Scanner } | { type: 'asset'; network: string; existing?: Asset; snapshot: Settings } | { type: 'addresses'; network: string };
export function DepositSettingsPage() {
  const session = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('admin'));
  const canConfigure = useCanAdminRequest(`${base}/deposit-settings`, 'GET');
  const query = useQuery({ queryKey: ['timed-deposit-settings', session?.generation, canConfigure], queryFn: async (): Promise<Settings> => canConfigure ? parseSettings(await apiRequest<unknown>(`${base}/deposit-settings`)) : { scanners: [], assets: [], platform_assets: [], addresses: parseAddresses(await apiRequest<unknown>(`${base}/deposit-settings/addresses`)) }, enabled: Boolean(session), refetchOnWindowFocus: false });
  return <main className="exchange-page admin-action-page"><PageHeader title="链上充值" description="按网络管理监听币种和收款地址。同一网络可同时接收多个币种，用户先输入币种数量，再获得 30 分钟有效的充值组合。" />
    {query.error ? <Banner type="danger" description={adminErrorMessage(query.error)} /> : null}
    {query.data ? <SettingsEditor key={`${session?.generation}:${canConfigure}`} data={query.data} canConfigure={canConfigure} unavailable={query.isFetching || query.isError} reload={async () => { await query.refetch(); }} /> : query.isPending ? <p>正在读取配置…</p> : <Button onClick={() => { void query.refetch(); }}>重新读取配置</Button>}
  </main>;
}

function SettingsEditor({ data, reload, canConfigure, unavailable }: { data: Settings; reload: () => Promise<void>; canConfigure: boolean; unavailable: boolean }) {
  const [network, setNetwork] = useState('eth');
  const [editing, setEditing] = useState<Editing | null>(null);
  const [addressBusy, setAddressBusy] = useState(false);
  const scanner = data.scanners.find(row => row.network === network);
  const assets = data.assets.filter(row => row.network === network);
  const addresses = data.addresses.filter(row => row.network === network);
  const canEditScanner = useCanAdminRequest(`${base}/deposit-settings/scanners/${network}`, 'PUT');
  const canEditAsset = useCanAdminRequest(`${base}/deposit-settings/assets`, 'PUT');
  const canImport = useCanAdminRequest(`${base}/deposit-settings/addresses`, 'POST');
  const saved = async () => { setEditing(null); await reload(); };
  const locked = unavailable || addressBusy;
  return <div className="deposit-settings-grid">
    <div className="deposit-network-toolbar"><div><label id="deposit-network-label">配置网络</label><AdminSelect ariaLabel="配置网络" ariaLabelledBy="deposit-network-label" value={network} onChange={setNetwork} optionList={networkOptions} /></div>
      <Button disabled={addressBusy || Boolean(editing)} onClick={() => { void reload(); }}>刷新配置与监听状态</Button>
    </div>
    {scanner ? <Card title={<Typography.Title heading={6}>{networkLabel(network)} · 网络与监听</Typography.Title>} headerExtraContent={canEditScanner ? <Button disabled={locked} onClick={() => setEditing({ type: 'scanner', scanner: { ...scanner } })}>编辑监听配置</Button> : <Tag>只读权限</Tag>}>
      <div className="deposit-overview"><span>新订单：{scanner.enabled ? '允许接单' : '已暂停'}</span><span>节点：{scanner.endpoint_configured ? '已配置' : '待配置'}</span><span>确认深度：{scanner.confirmations}</span><span>启用币种：{assets.filter(row => row.enabled).length} 个</span><span>启用地址：{addresses.filter(row => row.enabled).length} 个</span></div>
      <p>最近追上区块：<TimestampText value={scanner.last_scan_at} /></p>
      {scanner.last_error ? <Banner type="warning" description={`监听异常：${scanner.last_error}`} /> : <p className="muted">只有平台资产允许充值、币种与地址已启用、节点最近成功追上区块时，用户才能创建充值订单。</p>}
    </Card> : canConfigure ? <Banner type="warning" description="当前网络缺少监听配置，请刷新确认服务端配置完整。" /> : <Banner type="info" description="当前账号可管理收款地址。查看和编辑网络、币种需要充值网络配置权限。" />}
    {canConfigure ? <Card title={<Typography.Title heading={6}>监听币种 · {assets.length} 个</Typography.Title>} headerExtraContent={canEditAsset ? <Button theme="solid" disabled={locked} onClick={() => setEditing({ type: 'asset', network, snapshot: data })}>新增监听币种</Button> : <Tag>只读权限</Tag>}>
      <p>每个币种单独配置链上身份，共用本网络的地址池。Ethereum 可添加 ETH、USDT、USDC 等；后续仍可继续添加。</p>
      {!assets.length ? <p className="muted">尚未添加监听币种。先添加原生币或已核对合约的代币。</p> : <ul className="deposit-config-list">{assets.map(asset => <li key={asset.id}>
        <div className="deposit-config-main"><Space wrap><Typography.Text strong>{asset.asset_symbol}</Typography.Text><Tag color={asset.enabled ? 'green' : 'grey'}>{asset.enabled ? '启用' : '停用'}</Tag><span>{asset.asset_kind === 'native' ? '原生币' : '代币'} · 精度 {asset.decimals}</span></Space>
          <div className="deposit-contract">{asset.asset_kind === 'native' ? '无需合约地址' : <Typography.Text copyable>{asset.token_address}</Typography.Text>}</div>
          {data.platform_assets.some(row => row.symbol === asset.asset_symbol && (row.status !== 'active' || !row.deposit_enabled)) ? <span className="muted">平台资产充值尚未开启</span> : null}
        </div>
        {canEditAsset ? <Button aria-label={`编辑 ${asset.asset_symbol} 监听`} disabled={locked} onClick={() => setEditing({ type: 'asset', network, existing: { ...asset }, snapshot: data })}>编辑 / 启停</Button> : null}
      </li>)}</ul>}
    </Card> : null}
    <Card title={<Typography.Title heading={6}>收款地址 · {addresses.filter(row => row.enabled).length} 个启用 / 共 {addresses.length} 个</Typography.Title>} headerExtraContent={canImport ? <Button disabled={locked} onClick={() => setEditing({ type: 'addresses', network })}>添加收款地址</Button> : <Tag>只读权限</Tag>}>
      <p>可先存放 10 个地址，后续随时增加。不同金额可共用地址；同一币种的相同金额使用其他地址。组合满额时提示稍后重试，不修改充值金额。</p>
      {!addresses.length ? <p className="muted">尚无收款地址，点击“添加收款地址”批量导入。</p> : <ul className="deposit-config-list">{addresses.map(row => <AddressRow key={row.pool_id} row={row} disabled={locked} setBusy={setAddressBusy} reload={reload} />)}</ul>}
    </Card>
    {editing ? <DepositEditPanel title={editing.type === 'scanner' ? `${networkLabel(editing.scanner.network)} · 编辑监听配置` : editing.type === 'asset' ? `${networkLabel(editing.network)} · ${editing.existing ? '编辑监听币种' : '新增监听币种'}` : `${networkLabel(editing.network)} · 添加收款地址`} close={() => setEditing(null)}>
      {controls => editing.type === 'scanner' ? <ScannerForm scanner={editing.scanner} {...controls} saved={saved} /> : editing.type === 'asset' ? <AssetForm network={editing.network} existing={editing.existing} assets={editing.snapshot.assets.filter(row => row.network === editing.network)} catalog={editing.snapshot.platform_assets} {...controls} saved={saved} /> : <AddressImportForm network={editing.network} {...controls} saved={saved} />}
    </DepositEditPanel> : null}
  </div>;
}

function AddressRow({ row, disabled, setBusy, reload }: { row: SharedAddress; disabled: boolean; setBusy: (busy: boolean) => void; reload: () => Promise<void> }) {
  const url = `${base}/deposit-settings/addresses/${row.pool_id}`;
  return <li><div className="deposit-config-main"><Typography.Text copyable>{row.address}</Typography.Text><div><Tag color={row.enabled ? 'green' : 'grey'}>{row.enabled ? '启用' : '停用'}</Tag></div></div>
    <AdminRequestActionBoundary endpoint={url} method="PUT"><ConfirmAction actionText={row.enabled ? '停用地址' : '启用地址'} title={`${row.enabled ? '停用' : '启用'} ${row.address}`} disabled={disabled} description="只影响新订单，已有订单及迟到款继续监听。" onConfirm={async reason => {
      setBusy(true); try { await apiRequest(url, { method: 'PUT', body: JSON.stringify({ enabled: !row.enabled, reason }) }); await reload(); } finally { setBusy(false); }
    }} /></AdminRequestActionBoundary>
  </li>;
}
