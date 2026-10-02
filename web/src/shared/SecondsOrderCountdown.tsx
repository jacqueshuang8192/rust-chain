import { useEffect, useState } from 'react';

/** Display only: settlement remains authoritative on the server. */
export function SecondsOrderCountdown({ expiresAt, orderId }: { expiresAt: number; orderId: number }) {
  const [remaining, setRemaining] = useState(() => Math.max(0, Math.ceil((expiresAt - Date.now()) / 1000)));

  useEffect(() => {
    let interval: number | undefined;
    const sync = () => {
      const next = Math.max(0, Math.ceil((expiresAt - Date.now()) / 1000));
      setRemaining(next);
      if (next === 0 && interval !== undefined) {
        window.clearInterval(interval);
        interval = undefined;
      }
    };
    const onVisible = () => {
      if (document.visibilityState === 'visible') sync();
    };
    sync();
    if (expiresAt > Date.now()) interval = window.setInterval(sync, 1000);
    window.addEventListener('focus', sync);
    document.addEventListener('visibilitychange', onVisible);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener('focus', sync);
      document.removeEventListener('visibilitychange', onVisible);
    };
  }, [expiresAt]);

  const hours = Math.floor(remaining / 3600);
  const minutes = String(Math.floor((remaining % 3600) / 60)).padStart(2, '0');
  const seconds = String(remaining % 60).padStart(2, '0');
  const countdown = `${hours > 0 ? `${String(hours).padStart(2, '0')}:` : ''}${minutes}:${seconds}`;
  return (
    <span aria-label={`订单 ${orderId} 倒计时`} className="agent-order-countdown" role="timer" style={{ fontVariantNumeric: 'tabular-nums', whiteSpace: 'nowrap' }}>
      {remaining > 0 ? countdown : '待结算'}
    </span>
  );
}
