import { Button, Card, Tag, Typography } from '@douyinfe/semi-ui';
import { TimestampText } from '../../shared/TimestampText';
import type { Asset, PlatformAsset, Scanner, SharedAddress } from './contracts';
import { methodBlockers, scanGuides, scanStatus } from './setupStatus';

type Props = {
  network: string; scanner?: Scanner; assets: Asset[]; addresses: SharedAddress[]; catalog: PlatformAsset[];
  locked: boolean; unavailable: boolean; editNode?: () => void; editAssets?: () => void; editAddresses?: () => void;
};

export function DepositSetupGuide({ network, scanner, assets, addresses, catalog, locked, unavailable, editNode, editAssets, editAddresses }: Props) {
  const scan = scanStatus(scanner);
  const activeAssets = assets.filter(asset => asset.enabled && catalog.some(row => row.symbol === asset.asset_symbol && row.status === 'active' && row.deposit_enabled));
  const activeAddresses = addresses.filter(row => row.enabled);
  const available = unavailable ? 0 : assets.filter(asset => methodBlockers(asset, scanner, addresses, catalog).length === 0).length;
  const steps = [
    { title: '节点接入', summary: scanner?.endpoint_configured ? '节点已保存' : '选择免费默认渠道或配置自有节点', done: scanner?.endpoint_configured, action: '配置节点', edit: editNode },
    { title: '监听币种', summary: activeAssets.length ? `${activeAssets.length} 个币种已开启平台充值与监听` : '添加币种身份并开启平台充值', done: activeAssets.length > 0, action: '配置币种', edit: editAssets },
    { title: '收款地址', summary: activeAddresses.length ? `${activeAddresses.length} 个启用地址，共享给本网络币种` : '导入本网络的收款地址', done: activeAddresses.length > 0, action: '配置地址', edit: editAddresses },
    { title: '扫描与接单', summary: unavailable ? '状态待重新读取' : `${scan.label} · ${scanner?.enabled ? '接单开关已开启' : '接单开关未开启'}`, done: !unavailable && available > 0, action: '', edit: undefined }
  ];
  return <Card className="deposit-setup-guide" title={<Typography.Title heading={6}>收款配置指引</Typography.Title>}>
    <div className="deposit-setup-summary"><div><Typography.Text strong>{scanGuides[network]?.title}</Typography.Text><p>{scanGuides[network]?.detail}</p></div><Tag color={available ? 'green' : 'grey'}>{unavailable ? '状态待刷新' : `可收款支付方式 ${available} / ${assets.length}`}</Tag></div>
    <ol className="deposit-setup-steps">{steps.map((step, index) => <li key={step.title}>
      <div><span className="deposit-step-number">{index + 1}</span><Typography.Text strong>{step.title}</Typography.Text><Tag color={step.done ? 'green' : 'grey'}>{step.done ? '已满足' : '待完成'}</Tag></div>
      <p>{step.summary}</p>{step.edit ? <Button disabled={locked} onClick={step.edit}>{step.action}</Button> : null}
    </li>)}</ol>
    <p className="muted">一个支付方式 = 网络 + 币种，例如 Ethereum / USDT。配置保存后仍需后台扫描成功，才会向用户开放。暂停接单不停止已有订单的到账扫描。</p>
    {scanner ? <p className="muted">服务端检查时间：<TimestampText value={scanner.checked_at} />；未编辑时每 15 秒刷新。{unavailable ? '当前读取失败或正在刷新，请勿将旧快照作为可用性依据。' : ''}</p> : null}
  </Card>;
}
