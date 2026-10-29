import { Button, Tag } from '@douyinfe/semi-ui';
import type { Asset, PlatformAsset, Scanner, SharedAddress } from './contracts';
import { methodBlockers, scanStatus } from './setupStatus';

type Props = {
  network: string; scanner?: Scanner; assets: Asset[]; addresses: SharedAddress[]; catalog: PlatformAsset[];
  locked: boolean; unavailable: boolean; editNode?: () => void; editAssets?: () => void; editAddresses?: () => void;
};

export function DepositSetupGuide({ scanner, assets, addresses, catalog, locked, unavailable, editNode, editAssets, editAddresses }: Props) {
  const scan = scanStatus(scanner);
  const activeAssets = assets.filter(asset => asset.enabled && catalog.some(row => row.symbol === asset.asset_symbol && row.status === 'active' && row.deposit_enabled));
  const activeAddresses = addresses.filter(row => row.enabled);
  const available = unavailable ? 0 : assets.filter(asset => methodBlockers(asset, scanner, addresses, catalog).length === 0).length;
  const steps = [
    { title: '节点接入', summary: scanner?.endpoint_configured ? '节点已保存' : '选择免费渠道或配置自有节点', done: scanner?.endpoint_configured, action: '配置节点', edit: editNode },
    { title: '监听币种', summary: activeAssets.length ? `${activeAssets.length} 个币种已开启` : '添加币种并开启平台充值', done: activeAssets.length > 0, action: '配置币种', edit: editAssets },
    { title: '收款地址', summary: activeAddresses.length ? `${activeAddresses.length} 个地址已启用` : '导入本网络收款地址', done: activeAddresses.length > 0, action: '配置地址', edit: editAddresses },
    { title: '扫描与接单', summary: unavailable ? '状态待重新读取' : `${scan.label} · ${scanner?.enabled ? '接单已开启' : '接单未开启'}`, done: !unavailable && available > 0 && available === activeAssets.length, action: '', edit: undefined }
  ];
  return <section className="deposit-setup-guide" aria-label="收款配置指引">
    <div className="deposit-setup-heading"><strong>收款配置指引</strong><Tag color={available ? 'green' : 'grey'}>{unavailable ? '状态待刷新' : `可收款支付方式 ${available} / ${assets.length}`}</Tag></div>
    <ol className="deposit-setup-steps">{steps.map((step, index) => <li key={step.title}>
      <div><span className={`deposit-step-number${!unavailable && step.done ? ' is-complete' : ''}`} aria-hidden="true">{!unavailable && step.done ? '✓' : index + 1}</span><strong>{step.title}</strong><span className="deposit-step-state">{unavailable ? '待确认' : step.done ? '已满足' : '待完成'}</span></div>
      <p>{step.summary}</p>{step.edit ? <Button size="small" theme="borderless" disabled={locked} onClick={step.edit}>{step.action}</Button> : null}
    </li>)}</ol>
  </section>;
}
