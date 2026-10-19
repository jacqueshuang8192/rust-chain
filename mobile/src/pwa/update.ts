export const PWA_UPDATE_TIMEOUT_MS = 15_000

export type PwaUpdateFailureReason =
  | 'activation-timeout'
  | 'controller-unavailable'
  | 'post-message-failed'
  | 'registration-unavailable'
  | 'update-check-failed'
  | 'update-check-timeout'
  | 'worker-redundant'

export interface PwaUpdateTimer {
  clear(handle: unknown): void
  set(callback: () => void, delayMs: number): unknown
}

type UpdateRegistration = Pick<ServiceWorkerRegistration,
  'addEventListener' | 'installing' | 'removeEventListener' | 'update' | 'waiting'>

export interface RunPwaUpdateOptions {
  controllerTarget: Pick<ServiceWorkerContainer, 'controller' | 'addEventListener' | 'removeEventListener'>
  /** Controller that served this page, retained across retries and other-tab updates. */
  loadedController: ServiceWorker | null
  onBusyChange(busy: boolean): void
  onErrorChange(error: boolean): void
  onFailure?(reason: PwaUpdateFailureReason): void
  onNoUpdate?(): void
  registration: UpdateRegistration | null
  recoverRegistration?(): Promise<UpdateRegistration | null>
  reload(): void
  timeoutMs?: number
  timer?: PwaUpdateTimer
}

/** A single deadline covers registration, download and activation, including retries. */
export function runPwaUpdate(options: RunPwaUpdateOptions): Promise<boolean> {
  const timeoutMs = Number.isFinite(options.timeoutMs) && Number(options.timeoutMs) > 0
    ? Number(options.timeoutMs) : PWA_UPDATE_TIMEOUT_MS
  const timer = options.timer || {
    clear: (handle: unknown) => globalThis.clearTimeout(handle as ReturnType<typeof setTimeout>),
    set: (callback: () => void, delay: number) => globalThis.setTimeout(callback, delay),
  }
  options.onBusyChange(true)
  options.onErrorChange(false)

  return new Promise<boolean>((resolve) => {
    let finished = false
    let activating = false
    const cleanup: Array<() => void> = []
    const watched = new Set<ServiceWorker>()
    const requested = new Set<ServiceWorker>()
    const finish = (changed: boolean, failure?: PwaUpdateFailureReason) => {
      if (finished) return
      finished = true
      cleanup.forEach((dispose) => dispose())
      if (failure) {
        options.onErrorChange(true)
        options.onFailure?.(failure)
      } else if (!changed) {
        options.onNoUpdate?.()
      }
      options.onBusyChange(false)
      if (changed) options.reload()
      resolve(changed)
    }
    const controllerChanged = () => {
      const controller = options.controllerTarget.controller
      // A lost/unchanged controller is not evidence that the new version controls this page.
      if (controller && controller !== options.loadedController) finish(true)
    }
    const deadline = timer.set(() => finish(false,
      activating ? 'activation-timeout' : 'update-check-timeout'), timeoutMs)
    cleanup.push(() => timer.clear(deadline))
    options.controllerTarget.addEventListener('controllerchange', controllerChanged)
    cleanup.push(() => options.controllerTarget.removeEventListener('controllerchange', controllerChanged))
    controllerChanged()
    if (finished) return

    const watchWorker = (worker: ServiceWorker | null) => {
      if (finished || !worker || watched.has(worker)) return
      watched.add(worker)
      const onStateChange = () => {
        if (finished) return
        controllerChanged()
        if (finished) return
        if (worker.state === 'redundant') {
          finish(false, 'worker-redundant')
        } else if (worker.state === 'installed' && !requested.has(worker)) {
          activating = true
          requested.add(worker)
          try {
            worker.postMessage({ type: 'SKIP_WAITING' })
          } catch {
            finish(false, 'post-message-failed')
          }
        } else if (worker.state === 'activating' || worker.state === 'activated') {
          activating = true
        }
      }
      worker.addEventListener('statechange', onStateChange)
      cleanup.push(() => worker.removeEventListener('statechange', onStateChange))
      onStateChange()
    }

    void (async () => {
      try {
        const registration = options.registration || await options.recoverRegistration?.()
        if (finished) return
        controllerChanged()
        if (finished) return
        if (!registration) {
          finish(false, 'registration-unavailable')
          return
        }
        const onUpdateFound = () => {
          watchWorker(registration.waiting)
          watchWorker(registration.installing)
        }
        registration.addEventListener('updatefound', onUpdateFound)
        cleanup.push(() => registration.removeEventListener('updatefound', onUpdateFound))
        onUpdateFound()
        if (finished || watched.size) return
        await registration.update()
        if (finished) return
        controllerChanged()
        if (finished) return
        onUpdateFound()
        // A stale prompt with no new worker is already up to date, not a registration failure.
        if (!finished && !watched.size) finish(false,
          options.loadedController && !options.controllerTarget.controller ? 'controller-unavailable' : undefined)
      } catch {
        if (!finished) finish(false, 'update-check-failed')
      }
    })()
  })
}
