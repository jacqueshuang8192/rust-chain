import { Banner, Button, Tag, Typography } from '@douyinfe/semi-ui';
import { TimestampText } from '../../shared/TimestampText';
import { AdminRequestActionBoundary } from '../access';
import { base, type Scanner, type ScannerStream } from './contracts';
import { scanStatus } from './setupStatus';

type Props = { scanner: Scanner; stream?: ScannerStream['stream']; unavailable: boolean; locked: boolean; recover: () => void };
export function ScannerProgressStatus({ scanner, stream, unavailable, locked, recover }: Props) {
  const scan = scanStatus(scanner);
  const unit = scanner.network === 'solana' ? ' slot' : '区块高度';
  const label = stream === 'native' ? 'ETH 原生币' : 'ERC20 代币（包括 USDC）';
  return <section aria-label={stream ? `${label}监听状态` : '监听状态'}>
    {stream ? <Typography.Title heading={6}>{label}</Typography.Title> : null}
    <div className="deposit-scan-status"><Tag color={!unavailable && scan.ready ? 'green' : 'grey'}>{unavailable ? '状态待刷新' : scan.label}</Tag><span>已扫描{unit}：{scanner.cursor_height ?? '尚未建立游标'}</span><span>最近追上区块：<TimestampText value={scanner.last_scan_at} /></span></div>
    <p className="muted">{scan.detail} 刷新只读取状态；恢复进度需单独选择处理历史区块的方式。</p>
    {scanner.recovery_mode ? <Banner type="info" description={`恢复请求已排队：${scanner.recovery_mode === 'backfill' ? '跳到最新并补扫历史' : '跳到最新并跳过历史'}。节点校验成功后生效；失败会保留原进度并重试。`} /> : null}
    {scanner.backfill_target_height !== null ? <div className="deposit-history-progress"><Typography.Text strong>历史补扫{scanner.backfill_blocked ? ' · 异常待核对' : ''}</Typography.Text><p>已扫描{unit}：{scanner.backfill_cursor_height} / {scanner.backfill_target_height}</p><p className="muted">实时监听优先，历史范围单独补扫，不需要等待补扫完成才接收新充值。</p></div> : <p className="muted">当前没有待补扫的历史区间。</p>}
    {scanner.backfill_last_error ? <Banner type="warning" description={`历史补扫异常：${scanner.backfill_last_error}${scanner.backfill_blocked ? stream ? '；已阻止对应币种的新订单准入。' : '；为避免错误入账，已阻止新订单准入。' : '；保留历史进度等待重试。'}`} /> : null}
    <AdminRequestActionBoundary endpoint={`${base}/deposit-settings/scanners/${scanner.network}/recovery`} method="POST"><Button disabled={locked || scanner.cursor_height === null || !scanner.endpoint_configured || Boolean(scanner.recovery_mode)} onClick={recover}>{stream ? `恢复${stream === 'native' ? ' ETH' : ' ERC20'} 到最新区块` : '恢复到最新区块'}</Button></AdminRequestActionBoundary>
    {scanner.last_error ? <Banner type="warning" description={`监听异常：${scanner.last_error}`} /> : <p className="muted">只有平台资产允许充值、币种与地址已启用、节点最近成功追上区块时，用户才能创建充值订单。</p>}
  </section>;
}
