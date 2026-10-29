import { Tag, Typography } from '@douyinfe/semi-ui';
import type { ReactNode } from 'react';
import { AdminSelect } from '../../shared/SemiFormControls';
import { TimestampText } from '../../shared/TimestampText';
import { networkOptions, type Asset, type PlatformAsset, type Scanner, type Settings, type SharedAddress } from './contracts';
import { networkLabel } from './DepositSettingsEditors';
import { methodBlockers, scanStatus } from './setupStatus';

function receivingState(scanner: Scanner | undefined, assets: Asset[], addresses: SharedAddress[], catalog: PlatformAsset[], unavailable: boolean) {
  const available = unavailable ? 0 : assets.filter(asset => methodBlockers(asset, scanner, addresses, catalog).length === 0).length;
  const label = unavailable ? '状态待刷新' : available > 0 ? available === assets.length ? '可收款' : '部分可收款' : !scanner?.endpoint_configured ? '待配置' : !scanner.enabled ? '已暂停接单' : '暂不可收款';
  return { available, label };
}

export function NetworkNavigation({ data, network, onChange, canConfigure, unavailable, disabled }: {
  data: Settings; network: string; onChange: (network: string) => void; canConfigure: boolean; unavailable: boolean; disabled: boolean;
}) {
  return <aside className="deposit-network-nav">
    <div className="deposit-network-mobile"><label id="deposit-network-label">配置网络</label><AdminSelect ariaLabel="配置网络" ariaLabelledBy="deposit-network-label" value={network} onChange={onChange} optionList={networkOptions} disabled={disabled} /></div>
    <nav className="deposit-network-list" aria-label="收款网络">
      <div className="deposit-network-nav-title"><strong>收款网络</strong><span>{networkOptions.length} 个主网</span></div>
      {networkOptions.map(option => {
        const scanner = data.scanners.find(row => row.network === option.value);
        const assets = data.assets.filter(row => row.network === option.value);
        const addresses = data.addresses.filter(row => row.network === option.value);
        const state = receivingState(scanner, assets, addresses, data.platform_assets, unavailable);
        return <button key={option.value} type="button" className="deposit-network-option" aria-label={`切换至 ${option.label}`} aria-pressed={network === option.value} disabled={disabled} onClick={() => onChange(option.value)}>
          <span className="deposit-network-option-name">{option.label}<span className={`deposit-network-dot${canConfigure && state.available ? ' is-ready' : ''}`} aria-hidden="true" /></span>
          <span className="deposit-network-option-state">{canConfigure ? state.label : '收款地址管理'}</span>
          <span className="deposit-network-option-count">{canConfigure ? `${assets.length} 个币种 · ` : ''}{addresses.filter(row => row.enabled).length} 个启用地址</span>
        </button>;
      })}
    </nav>
  </aside>;
}

export function NetworkOverview({ network, scanner, assets, addresses, catalog, canConfigure, unavailable, actions }: {
  network: string; scanner?: Scanner; assets: Asset[]; addresses: SharedAddress[]; catalog: PlatformAsset[]; canConfigure: boolean; unavailable: boolean; actions: ReactNode;
}) {
  const state = receivingState(scanner, assets, addresses, catalog, unavailable);
  const scan = scanStatus(scanner);
  const pending = unavailable ? [] : [...new Set(assets.flatMap(asset => methodBlockers(asset, scanner, addresses, catalog)))];
  if (!unavailable && !assets.length) pending.push('尚未添加监听币种');
  return <section className="deposit-network-overview" aria-label={`${networkLabel(network)} 收款概况`}>
    <div className="deposit-workbench-heading">
      <div><Typography.Title heading={4}>{networkLabel(network)}</Typography.Title><Tag color={canConfigure && state.available ? 'green' : 'grey'}>{canConfigure ? state.label : '收款地址管理'}</Tag></div>
      <div className="deposit-workbench-actions">{actions}</div>
    </div>
    {canConfigure ? <>
      <dl className="deposit-workbench-metrics">
        <div><dt>可收款币种</dt><dd>{unavailable ? '—' : state.available}<small> / {assets.length}</small></dd></div>
        <div><dt>启用收款地址</dt><dd>{addresses.filter(row => row.enabled).length}<small> / {addresses.length}</small></dd></div>
        <div><dt>启用 RPC 渠道</dt><dd>{scanner?.providers.filter(row => row.enabled).length ?? 0}<small> / {scanner?.providers.length ?? 0}</small></dd></div>
      </dl>
      <div className="deposit-workbench-snapshot"><span>{unavailable ? '正在重新确认状态，请勿以旧数据判断可用性。' : scan.label}</span><span>服务端检查：<TimestampText value={scanner?.checked_at ?? null} /> · 空闲时每 15 秒刷新</span></div>
      {pending.length ? <p className="deposit-workbench-pending">待处理：{pending.join('；')}</p> : null}
    </> : <p className="muted">选择网络，查看、添加或停用共享收款地址。</p>}
  </section>;
}
