import { Banner, Button, Card, Space, Tabs, TabPane, Tag, Typography } from '@douyinfe/semi-ui';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useState, useSyncExternalStore } from 'react';
import { apiRequest } from '../../api/client';
import { authStore } from '../../auth/authStore';
import { PageHeader } from '../../layouts/PageHeader';
import { adminErrorMessage } from '../../shared/adminErrorMessage';
import { ConfirmAction } from '../../shared/ConfirmAction';
import { AdminRequestActionBoundary, useCanAdminRequest } from '../access';
import { AddressImportForm, AssetForm, DepositEditPanel, networkLabel, ScannerForm } from './DepositSettingsEditors';
import { base, parseAddresses, parseSettings, type Asset, type Scanner, type ScannerStream, type RpcPreset, type Settings, type SharedAddress } from './contracts';
import { DepositSetupGuide } from './DepositSetupGuide';
import { ScannerProgressStatus } from './ScannerProgressStatus';
import { ScannerRecoveryForm } from './ScannerRecoveryForm';
import { ScannerProviderStatus } from './ScannerProviderStatus';
import { methodBlockers, scanGuides } from './setupStatus';
import { NetworkNavigation, NetworkOverview } from './NetworkWorkbench';
import './RechargeWorkspace.css';

type Editing = { type: 'recovery'; scanner: Scanner; stream?: ScannerStream['stream'] } | { type: 'scanner'; scanner: Scanner; presets: RpcPreset[] } | { type: 'asset'; network: string; existing?: Asset; snapshot: Settings } | { type: 'addresses'; network: string };
export function DepositSettingsPage() {
  const session = useSyncExternalStore(authStore.subscribe, () => authStore.getSession('admin'));
  const canConfigure = useCanAdminRequest(`${base}/deposit-settings`, 'GET');
  const query = useQuery({ queryKey: ['timed-deposit-settings', session?.generation, canConfigure], queryFn: async (): Promise<Settings> => canConfigure ? parseSettings(await apiRequest<unknown>(`${base}/deposit-settings`)) : { rpc_presets: [], scanners: [], assets: [], platform_assets: [], addresses: parseAddresses(await apiRequest<unknown>(`${base}/deposit-settings/addresses`)) }, enabled: Boolean(session), refetchOnWindowFocus: false });
  return <main className="exchange-page admin-action-page deposit-workspace-page"><PageHeader title="链上充值" description="管理各网络的收款币种、地址与监听渠道。" />
    {query.error ? <Banner type="danger" description={adminErrorMessage(query.error)} /> : null}
    {query.data ? <SettingsEditor key={`${session?.generation}:${canConfigure}`} data={query.data} canConfigure={canConfigure} unavailable={query.isFetching || query.isError} refreshing={query.isFetching} reload={async () => { await query.refetch(); }} /> : query.isPending ? <div className="admin-table-state" role="status">正在读取网络与收款配置…</div> : <Button onClick={() => { void query.refetch(); }}>重新读取配置</Button>}
  </main>;
}

function SettingsEditor({ data, reload, canConfigure, unavailable, refreshing }: { data: Settings; reload: () => Promise<void>; canConfigure: boolean; unavailable: boolean; refreshing: boolean }) {
  const [network, setNetwork] = useState('eth');
  const [section, setSection] = useState('receiving');
  const [editing, setEditing] = useState<Editing | null>(null);
  const [addressBusy, setAddressBusy] = useState(false);
  const [addressDialog, setAddressDialog] = useState(false);
  const scanner = data.scanners.find(row => row.network === network);
  const assets = data.assets.filter(row => row.network === network);
  const addresses = data.addresses.filter(row => row.network === network);
  const canEditScanner = useCanAdminRequest(`${base}/deposit-settings/scanners/${network}`, 'PUT');
  const canEditAsset = useCanAdminRequest(`${base}/deposit-settings/assets`, 'PUT');
  const canImport = useCanAdminRequest(`${base}/deposit-settings/addresses`, 'POST');
  const saved = async () => { setEditing(null); await reload(); };
  const locked = unavailable || addressBusy;
  const navigatingLocked = addressBusy || addressDialog || Boolean(editing);
  const openScanner = canEditScanner && scanner ? () => setEditing({ type: 'scanner', scanner: { ...scanner }, presets: data.rpc_presets }) : undefined;
  useEffect(() => {
    if (editing || addressDialog || locked || !canConfigure) return;
    const timer = window.setInterval(() => { if (document.visibilityState === 'visible') void reload(); }, 15000);
    return () => window.clearInterval(timer);
  }, [editing, addressDialog, locked, canConfigure, reload]);
  return <div className="deposit-network-workbench">
    <NetworkNavigation data={data} network={network} onChange={setNetwork} canConfigure={canConfigure} unavailable={unavailable} disabled={navigatingLocked} />
    <div className="deposit-workbench-content">
      <NetworkOverview network={network} scanner={scanner} assets={assets} addresses={addresses} catalog={data.platform_assets} canConfigure={canConfigure} unavailable={unavailable}
        actions={<><Button disabled={refreshing || navigatingLocked} onClick={() => { void reload(); }}>刷新配置与监听状态</Button>{canConfigure && scanner ? openScanner ? <Button theme="solid" disabled={locked || navigatingLocked} onClick={openScanner}>编辑监听配置</Button> : <Tag>只读权限</Tag> : null}</>} />
      {!canConfigure ? <Banner type="info" description="当前账号可管理收款地址。查看和编辑网络、币种需要充值网络配置权限。" /> : null}
      <Tabs className="deposit-section-tabs" activeKey={section} keepDOM={false} tabPaneMotion={false} onChange={setSection}>
        <TabPane itemKey="receiving" tab="收款配置" disabled={navigatingLocked}>
          <div className="deposit-settings-grid">
    {canConfigure ? <DepositSetupGuide network={network} scanner={scanner} assets={assets} addresses={addresses} catalog={data.platform_assets} locked={locked || navigatingLocked} unavailable={unavailable}
      editNode={canEditScanner && scanner ? () => setEditing({ type: 'scanner', scanner: { ...scanner }, presets: data.rpc_presets }) : undefined}
      editAssets={canEditAsset ? () => setEditing({ type: 'asset', network, snapshot: data }) : undefined}
      editAddresses={canImport ? () => setEditing({ type: 'addresses', network }) : undefined} /> : null}
            <div className="deposit-receiving-columns">
    {canConfigure ? <Card title={<Typography.Title heading={6}>监听币种 · {assets.length} 个</Typography.Title>} headerExtraContent={canEditAsset ? <Button theme="solid" disabled={locked || navigatingLocked} onClick={() => setEditing({ type: 'asset', network, snapshot: data })}>新增监听币种</Button> : <Tag>只读权限</Tag>}>
      <p className="muted">币种使用独立链上身份，共享本网络的收款地址。</p>
      {!assets.length ? <p className="muted">尚未添加监听币种。先添加原生币或已核对合约的代币。</p> : <ul className="deposit-config-list">{assets.map(asset => <li key={asset.id}>
        <div className="deposit-config-main"><Space wrap><Typography.Text strong>{asset.asset_symbol}</Typography.Text><Tag color={asset.enabled ? 'green' : 'grey'}>{asset.enabled ? '启用' : '停用'}</Tag><span>{asset.asset_kind === 'native' ? '原生币' : '代币'} · 精度 {asset.decimals}</span></Space>
          <div className="deposit-contract">{asset.asset_kind === 'native' ? '无需合约地址' : <Typography.Text copyable>{asset.token_address}</Typography.Text>}</div>
          <div className="deposit-method-state"><Tag color={!unavailable && methodBlockers(asset, scanner, addresses, data.platform_assets).length === 0 ? 'green' : 'grey'}>{unavailable ? '状态待刷新' : methodBlockers(asset, scanner, addresses, data.platform_assets).length === 0 ? '可创建充值订单' : '暂不可收款'}</Tag><span>{networkLabel(network)} / {asset.asset_symbol}</span></div>
          {methodBlockers(asset, scanner, addresses, data.platform_assets).length ? <p className="muted">待处理：{methodBlockers(asset, scanner, addresses, data.platform_assets).join('；')}</p> : null}
        </div>
        {canEditAsset ? <Button aria-label={`编辑 ${asset.asset_symbol} 监听`} disabled={locked || navigatingLocked} onClick={() => setEditing({ type: 'asset', network, existing: { ...asset }, snapshot: data })}>编辑 / 启停</Button> : null}
      </li>)}</ul>}
    </Card> : null}
    <Card title={<Typography.Title heading={6}>收款地址 · {addresses.filter(row => row.enabled).length} 个启用 / 共 {addresses.length} 个</Typography.Title>} headerExtraContent={canImport ? <Button disabled={locked || navigatingLocked} onClick={() => setEditing({ type: 'addresses', network })}>添加收款地址</Button> : <Tag>只读权限</Tag>}>
      <p className="muted">本网络所有监听币种共享地址。停用仅影响新订单，已有订单继续监听。</p>
      {!addresses.length ? <p className="muted">尚无收款地址，点击“添加收款地址”批量导入。</p> : <ul className="deposit-config-list">{addresses.map(row => <AddressRow key={row.pool_id} row={row} disabled={locked || navigatingLocked} setBusy={setAddressBusy} onOpenChange={setAddressDialog} reload={reload} />)}</ul>}
    </Card>
            </div>
          </div>
        </TabPane>
        {canConfigure ? <TabPane itemKey="channels" tab="RPC 渠道" disabled={navigatingLocked}>
          <div className="deposit-settings-grid">
    {scanner ? <Card className="deposit-channels-card" title={<Typography.Title heading={6}>RPC 渠道 · {networkLabel(network)}</Typography.Title>}>
      <ScannerProviderStatus scanner={scanner} presets={data.rpc_presets} unavailable={unavailable} />
    </Card> : <Banner type="warning" description="当前网络缺少监听配置，请刷新确认服务端配置完整。" />}
          </div>
        </TabPane> : null}
        {canConfigure ? <TabPane itemKey="progress" tab="扫描与恢复" disabled={navigatingLocked}>
          <div className="deposit-settings-grid">
    {scanner ? <Card title={<Typography.Title heading={6}>{networkLabel(network)} · 扫描进度</Typography.Title>}>
      <div className="deposit-overview"><span>接单开关：{scanner.enabled ? '已开启' : '已暂停'}</span><span>节点：{scanner.endpoint_configured ? '已配置' : '待配置'}</span><span>确认深度：{scanner.confirmations}</span><span>启用币种：{assets.filter(row => row.enabled).length} 个</span><span>启用地址：{addresses.filter(row => row.enabled).length} 个</span></div>
      {(network === 'base' ? (scanner.streams ?? []).map(stream => ({ scanner: { ...scanner, ...stream, streams: [] }, stream: stream.stream })) : [{ scanner, stream: undefined }]).map(item => <ScannerProgressStatus key={item.stream ?? network} scanner={item.scanner} stream={item.stream} unavailable={unavailable} locked={locked || navigatingLocked} recover={() => setEditing({ type: 'recovery', scanner: item.scanner, stream: item.stream })} />)}
    </Card> : <Banner type="warning" description="当前网络缺少监听配置，请刷新确认服务端配置完整。" />}
            <details className="deposit-network-notes"><summary>监听范围与网络说明</summary><p>{scanGuides[network]?.detail}</p><p>暂停接单不停止已有订单的到账扫描。用户输入币种数量后，获得 30 分钟有效的充值组合。</p></details>
          </div>
        </TabPane> : null}
      </Tabs>
    </div>
    {editing ? <DepositEditPanel className={editing.type === 'recovery' ? 'deposit-recovery-panel' : editing.type === 'scanner' ? 'deposit-scanner-panel' : undefined} title={editing.type === 'recovery' ? `${networkLabel(editing.scanner.network)} · 恢复扫描进度` : editing.type === 'scanner' ? `${networkLabel(editing.scanner.network)} · 编辑监听配置` : editing.type === 'asset' ? `${networkLabel(editing.network)} · ${editing.existing ? '编辑监听币种' : '新增监听币种'}` : `${networkLabel(editing.network)} · 添加收款地址`} close={() => setEditing(null)}>
      {controls => editing.type === 'recovery' ? <ScannerRecoveryForm scanner={editing.scanner} stream={editing.stream} {...controls} saved={saved} /> : editing.type === 'scanner' ? <ScannerForm scanner={editing.scanner} presets={editing.presets} {...controls} saved={saved} /> : editing.type === 'asset' ? <AssetForm network={editing.network} existing={editing.existing} assets={editing.snapshot.assets.filter(row => row.network === editing.network)} catalog={editing.snapshot.platform_assets} {...controls} saved={saved} /> : <AddressImportForm network={editing.network} {...controls} saved={saved} />}
    </DepositEditPanel> : null}
  </div>;
}

function AddressRow({ row, disabled, setBusy, onOpenChange, reload }: { row: SharedAddress; disabled: boolean; setBusy: (busy: boolean) => void; onOpenChange: (open: boolean) => void; reload: () => Promise<void> }) {
  const url = `${base}/deposit-settings/addresses/${row.pool_id}`;
  return <li><div className="deposit-config-main"><Typography.Text copyable>{row.address}</Typography.Text><div><Tag color={row.enabled ? 'green' : 'grey'}>{row.enabled ? '启用' : '停用'}</Tag></div></div>
    <AdminRequestActionBoundary endpoint={url} method="PUT"><ConfirmAction modalClassName="deposit-confirmation" modalWidth="min(448px, calc(100vw - 32px))" onOpenChange={onOpenChange} actionText={row.enabled ? '停用地址' : '启用地址'} title={`${row.enabled ? '停用' : '启用'} ${row.address}`} disabled={disabled} description="只影响新订单，已有订单及迟到款继续监听。" onConfirm={async reason => {
      setBusy(true); try { await apiRequest(url, { method: 'PUT', body: JSON.stringify({ enabled: !row.enabled, reason }) }); await reload(); } finally { setBusy(false); }
    }} /></AdminRequestActionBoundary>
  </li>;
}
