import { Banner, Switch } from '@douyinfe/semi-ui';
import { useState } from 'react';
import { apiRequest } from '../../api/client';
import { ConfirmAction } from '../../shared/ConfirmAction';
import { AdminSelect } from '../../shared/SemiFormControls';
import { base, type Scanner, type ScannerStream } from './contracts';

type Props = { scanner: Scanner; stream?: ScannerStream['stream']; dirty: () => void; busy: boolean; setBusy: (busy: boolean) => void; saved: () => Promise<void> };
export function ScannerRecoveryForm({ scanner, stream, dirty, busy, setBusy, saved }: Props) {
  const [mode, setMode] = useState('backfill');
  const [acknowledged, setAcknowledged] = useState(false);
  const skipping = mode === 'skip';
  const history = scanner.backfill_cursor_height ?? scanner.cursor_height;
  return <div className="deposit-edit-form">
    {stream ? <p>恢复范围：{stream === 'native' ? 'ETH 原生币' : 'ERC20 代币（包括 USDC）'}。另一分流保留原进度。</p> : null}
    <p>当前实时进度：{scanner.cursor_height}。目标由后台读取该网络最新已确认{scanner.network === 'solana' ? ' slot' : '区块'}，保留确认深度 {scanner.confirmations}。</p>
    <label id="recovery-mode-label">历史区块处理方式</label>
    <AdminSelect ariaLabel="历史区块处理方式" ariaLabelledBy="recovery-mode-label" disabled={busy} value={mode} optionList={[{ value: 'backfill', label: '双进度：跳到最新并补扫历史' }, { value: 'skip', label: '手动跳过：不再扫描历史区间' }]} onChange={value => { dirty(); setMode(value); setAcknowledged(false); }} />
    {skipping ? <>
      <Banner type="warning" description={`将放弃从进度 ${history} 到目标区块之间的历史扫描，包括已有的补扫任务。该区间的到账不会再自动识别，需另行核对处理；不会删除订单或已入账记录。`} />
      <label className="deposit-switch-row">我已了解跳过历史可能遗漏到账<Switch aria-label="确认跳过历史风险" checked={acknowledged} disabled={busy} onChange={value => { dirty(); setAcknowledged(value); }} /></label>
    </> : <Banner type="info" description={`保留从进度 ${history} 开始的历史区间。最新区块优先监听，历史区块按限速继续补扫；重复恢复会扩展已有补扫范围，重复发现的交易不会重复入账。`} />}
    <p>提交后先等待节点校验。节点不可用、链身份不符或区块锚点异常时保留原进度。恢复完成且最新扫描成功后，符合其他收款条件的网络即可继续接单。</p>
    <ConfirmAction modalWidth="min(448px, calc(100vw - 32px))" key={mode} actionText="提交恢复请求" title={skipping ? '跳到最新并跳过历史' : '跳到最新并保留历史补扫'} dangerous={skipping} disabled={busy || (skipping && !acknowledged)} description={skipping ? '此操作会停止该历史区间（含已有补扫）的自动到账识别。请填写核对依据和操作原因。' : '先恢复最新扫描，同时保留历史补扫；操作与实际应用的进度均记录审计。'} onConfirm={async reason => {
      setBusy(true);
      try {
        await apiRequest(`${base}/deposit-settings/scanners/${scanner.network}/recovery`, { method: 'POST', body: JSON.stringify({ ...(stream ? { stream } : {}), mode, expected_revision: scanner.revision, acknowledge_skip: skipping && acknowledged, reason }) });
        await saved();
      } finally { setBusy(false); }
    }} />
  </div>;
}
