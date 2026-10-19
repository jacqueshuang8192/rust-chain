import { Button, Image, Upload } from '@douyinfe/semi-ui';
import type { customRequestArgs } from '@douyinfe/semi-ui/lib/es/upload';
import { useEffect, useRef, useState } from 'react';
import { apiRequest } from '../api/client';
import { authStore } from '../auth/authStore';
import { uploadAdminImageFile, type AdminUploadedImage } from '../shared/AdminImageUpload';
import { adminErrorMessage } from '../shared/adminErrorMessage';

export function CoinLogoUpload({ scope, value, disabled, onChange, onBusy }: {
  scope: 'admin' | 'agent'; value: string; disabled: boolean;
  onChange: (url: string) => void; onBusy: (busy: boolean) => void;
}) {
  const [busy, setBusy] = useState(false), [error, setError] = useState('');
  const active = useRef(true), flight = useRef(false);
  useEffect(() => { active.current = true; return () => { active.current = false; }; }, []);
  async function upload(request: customRequestArgs) {
    if (disabled || flight.current) return;
    flight.current = true; setBusy(true); onBusy(true); setError('');
    const generation = authStore.getSession(scope)?.generation;
    try {
      const body = new FormData(); body.append('file', request.fileInstance);
      const result = scope === 'admin' ? await uploadAdminImageFile(request.fileInstance)
        : await apiRequest<AdminUploadedImage>('/agent/api/v1/coin-applications/logo', { authScope: 'agent', method: 'POST', body });
      if (!active.current || authStore.getSession(scope)?.generation !== generation) return;
      if (typeof result.download_url !== 'string' || !result.download_url.trim()) throw new Error('上传未返回图片地址，请重试');
      onChange(result.download_url); request.onSuccess(result);
    } catch (cause) {
      if (active.current) { setError(adminErrorMessage(cause, 'Logo 上传失败，请重试')); request.onError({ status: 500 }); }
    } finally {
      flight.current = false;
      if (active.current) { setBusy(false); onBusy(false); }
    }
  }
  return <div className="coin-application-wide">
    <p>币种 Logo</p>
    {value ? <Image src={value} alt="币种 Logo" width={64} height={64} imgStyle={{ objectFit: 'contain' }} /> : null}
    <Upload accept="image/png,image/jpeg,image/webp,image/gif" customRequest={upload} showUploadList={false} disabled={disabled || busy}>
      <Button disabled={disabled || busy} loading={busy}>{value ? '更换币种 Logo' : '上传币种 Logo'}</Button>
    </Upload>
    {value ? <Button disabled={disabled || busy} onClick={() => onChange('')}>清除币种 Logo</Button> : null}
    <p>上市后用于现货交易对。</p>
    {error ? <p role="alert">{error}</p> : null}
  </div>;
}
