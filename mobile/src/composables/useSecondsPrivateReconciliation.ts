import { onBeforeUnmount, onMounted } from 'vue'
import { usePrivateUserStreamLease } from './usePrivateUserStreamLease'

export interface SecondsPrivateReconciliationResult {
  ordersLoaded: boolean
  accountsLoaded: boolean
}

/** Private frames are refresh hints; only reconciled REST data changes balances. */
export function useSecondsPrivateReconciliation(options: {
  enabled: () => boolean
  sessionKey: () => string
  reconcile: () => Promise<SecondsPrivateReconciliationResult>
  onResult: (loaded: boolean) => void
}) {
  let active = true
  let pending = false
  let inFlight: Promise<void> | null = null
  let lastRefreshAt = Date.now()
  let timer: ReturnType<typeof setInterval> | undefined

  function refresh(): Promise<void> {
    if (!active || !options.enabled()) return Promise.resolve()
    pending = true
    lastRefreshAt = Date.now()
    if (inFlight) return inFlight
    inFlight = (async () => {
      while (pending && active && options.enabled()) {
        pending = false
        const key = options.sessionKey()
        let loaded = false
        try {
          const result = await options.reconcile()
          loaded = result.ordersLoaded && result.accountsLoaded
        } catch {
          // Preserve the last authoritative snapshot and retry on the next hint.
        }
        if (active && options.enabled() && key === options.sessionKey()) options.onResult(loaded)
      }
    })().finally(() => { inFlight = null })
    return inFlight
  }

  function onVisible(): void {
    if (document.visibilityState === 'visible') void refresh()
  }

  const stream = usePrivateUserStreamLease({
    topic: 'seconds',
    consumerId: 'seconds-trading',
    enabled: options.enabled,
    onOpen: () => { void refresh() },
    onEvent: () => { void refresh() },
  })
  onMounted(() => {
    document.addEventListener('visibilitychange', onVisible)
    window.addEventListener('focus', onVisible)
    timer = setInterval(() => {
      if (Date.now() - lastRefreshAt >= 60_000) onVisible()
    }, 10_000)
  })
  onBeforeUnmount(() => {
    active = false
    pending = false
    clearInterval(timer)
    document.removeEventListener('visibilitychange', onVisible)
    window.removeEventListener('focus', onVisible)
  })
  return { ...stream, refresh }
}
