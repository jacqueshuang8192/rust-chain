import { Tag } from '@douyinfe/semi-ui';
import { TimestampText } from '../../shared/TimestampText';
import type { Scanner } from './contracts';
export function ScannerProviderStatus({ scanner }: { scanner: Scanner }) {
  return <ul className="deposit-provider-status">{scanner.providers.map(provider => <li key={provider.id}>
    <div><strong>{provider.name}</strong> <Tag>{!provider.enabled ? '已停用' : provider.cooldown_until !== null && provider.cooldown_until > scanner.checked_at ? '渠道冷却' : !provider.keys.some(key => key.enabled && (key.cooldown_until === null || key.cooldown_until <= scanner.checked_at)) ? '无可用凭据' : provider.last_success_at === null ? '尚未验证' : '可尝试'}</Tag></div>
    <p>优先级 {provider.priority} · 请求间隔 {provider.request_interval_ms} ms · 最近成功 <TimestampText value={provider.last_success_at} /></p>
    {provider.cooldown_until !== null && provider.cooldown_until > scanner.checked_at ? <p>渠道冷却至 <TimestampText value={provider.cooldown_until} /></p> : null}
    {provider.last_error ? <p>{provider.last_error}</p> : null}
    {provider.keys.map(key => <div key={key.id} className="deposit-key-status"><strong>{key.name}</strong> · {key.api_key_configured ? '密钥已配置' : '无请求头密钥'} · {!key.enabled ? '已停用' : key.cooldown_until !== null && key.cooldown_until > scanner.checked_at ? '冷却中' : '可尝试'}
      {key.cooldown_until !== null && key.cooldown_until > scanner.checked_at ? <span>，恢复时间 <TimestampText value={key.cooldown_until} /></span> : null}
      {key.last_error ? <p>{key.last_error}</p> : null}
    </div>)}
  </li>)}</ul>;
}
