import { Collapse, Tag } from '@douyinfe/semi-ui';
import { TimestampText } from '../../shared/TimestampText';
import type { RpcPreset, Scanner, ScannerProvider } from './contracts';

function availability(provider: ScannerProvider, scanner: Scanner, unavailable: boolean) {
  const keys = provider.keys.filter(key => key.enabled && (key.cooldown_until === null || key.cooldown_until <= scanner.checked_at)).length;
  const cooling = (provider.stream_failures ?? []).filter(failure => failure.cooldown_until !== null && failure.cooldown_until > scanner.checked_at);
  const label = unavailable ? '状态待刷新' : !provider.enabled ? '已停用' : provider.cooldown_until !== null && provider.cooldown_until > scanner.checked_at ? '渠道冷却' : !keys ? '无可用凭据' : scanner.network === 'base' && !provider.stream_failures ? '分流状态待更新' : cooling.length === 2 ? '分流冷却' : cooling.length ? '部分冷却' : provider.last_success_at === null ? '尚未验证' : '可尝试';
  return { keys, cooling, label, eligible: ['尚未验证', '可尝试', '部分冷却'].includes(label) };
}

export function ScannerProviderStatus({ scanner, presets = [], unavailable = false }: { scanner: Scanner; presets?: RpcPreset[]; unavailable?: boolean }) {
  const providers = [...scanner.providers].sort((a, b) => a.priority - b.priority || a.id - b.id);
  const eligible = providers.filter(provider => availability(provider, scanner, unavailable).eligible).length;
  const unknown = unavailable || providers.some(provider => provider.enabled && scanner.network === 'base' && provider.stream_failures === undefined);
  const enabled = providers.filter(provider => provider.enabled);
  return <section aria-label="RPC 渠道状态">
    <div className="deposit-channel-summary"><div><strong>{providers.length}</strong><span>已配置渠道</span></div><div><strong>{unknown ? '—' : eligible}</strong><span>可尝试渠道</span></div><div><strong>{providers.reduce((sum, provider) => sum + provider.keys.length, 0)}</strong><span>凭据槽</span></div></div>
    <p className="muted">按优先级自动切换，每轮先尝试各渠道的首个可用凭据，再轮转其他 Key。冷却中的渠道或凭据会跳过；可尝试不代表已成功追上区块。</p>
    {!providers.length ? <p>尚未配置 RPC 渠道。打开“编辑监听配置”，选择免费公共渠道或填写自己的 API Key。</p> : null}
    <ul className="deposit-provider-status">{providers.map(provider => {
      const state = availability(provider, scanner, unavailable);
      const preset = presets.find(preset => preset.id === provider.preset_id);
      const rank = enabled.indexOf(provider);
      return <li key={provider.id}>
        <div className="deposit-channel-heading"><div><span className="deposit-channel-rank">{rank < 0 ? '停用' : rank === 0 ? '首选' : `备用 ${rank}`}</span><strong>{provider.name}</strong></div><Tag color={state.cooling.length || ['渠道冷却', '无可用凭据'].includes(state.label) ? 'orange' : 'grey'}>{state.label}</Tag></div>
        <p className="muted">优先级 {provider.priority} · 请求间隔 {provider.request_interval_ms} ms · 可尝试凭据 {unavailable ? '—' : state.keys} / {provider.keys.length}</p>
        {provider.preset_id ? <p>内置免费渠道 · {preset?.name ?? provider.preset_id} · 免密钥</p> : null}
        {preset?.recommended === false ? <p>{preset.note}</p> : null}
        <p>最近渠道扫描成功 <TimestampText value={provider.last_success_at} /></p>
        {provider.cooldown_until !== null && provider.cooldown_until > scanner.checked_at ? <p>渠道冷却至 <TimestampText value={provider.cooldown_until} /></p> : null}
        {provider.last_error ? <p className="deposit-channel-error">最近渠道错误：{provider.last_error}</p> : null}
        {scanner.network === 'base' && provider.stream_failures ? <div className="deposit-stream-availability">{(['native', 'token'] as const).map(stream => {
          const failure = provider.stream_failures?.find(failure => failure.stream === stream);
          const cooling = state.cooling.some(failure => failure.stream === stream);
          return <div key={stream}><strong>{stream === 'native' ? 'ETH 原生币' : 'ERC20 / USDC'}</strong><span>{unavailable ? '状态待刷新' : !provider.enabled ? '已停用' : provider.cooldown_until !== null && provider.cooldown_until > scanner.checked_at ? '渠道冷却' : !state.keys ? '无可用凭据' : cooling ? '冷却中' : '可尝试'}</span>
            {cooling ? <p>恢复时间 <TimestampText value={failure?.cooldown_until ?? null} /></p> : null}
            {failure?.last_error ? <p className="deposit-channel-error">最近分流错误：{failure.last_error}</p> : null}
          </div>;
        })}</div> : null}
        <Collapse motion={false} keepDOM defaultActiveKey={provider.keys.some(key => key.last_error) ? 'keys' : undefined}>
          <Collapse.Panel itemKey="keys" header={`凭据与错误详情 · ${provider.keys.length} 个`}>
            {provider.keys.map(key => <div key={key.id} className="deposit-key-status"><strong>{key.name}</strong> · {key.api_key_configured ? '密钥已配置' : '免密钥访问'} · {unavailable ? '状态待刷新' : !key.enabled ? '已停用' : key.cooldown_until !== null && key.cooldown_until > scanner.checked_at ? '冷却中' : '可尝试'}
              <p className="muted">优先级 {key.priority} · 请求间隔 {key.request_interval_ms} ms</p>
              {key.cooldown_until !== null && key.cooldown_until > scanner.checked_at ? <p>恢复时间 <TimestampText value={key.cooldown_until} /></p> : null}
              {key.last_error ? <p className="deposit-channel-error">{key.last_error}</p> : null}
            </div>)}
          </Collapse.Panel>
        </Collapse>
      </li>;
    })}</ul>
  </section>;
}
