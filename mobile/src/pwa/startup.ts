import type { PwaUpdateTimer } from './update'

export const PWA_STARTUP_TIMEOUT_MS = 4_000

interface PwaStartupOptions {
  controllerTarget: Pick<ServiceWorkerContainer,
    'controller' | 'addEventListener' | 'removeEventListener'>
  register(): Promise<ServiceWorkerRegistration | null>
  reload(): void
  timeoutMs?: number
  timer?: PwaUpdateTimer
}

/** Automatic activation is allowed only before the application becomes interactive. */
export function preparePwaStartup(options: PwaStartupOptions): Promise<boolean> {
  const initialController = options.controllerTarget.controller
  if (!initialController) return Promise.resolve(false)

  const timer = options.timer || {
    set: (callback: () => void, delay: number) => globalThis.setTimeout(callback, delay),
    clear: (handle: unknown) => globalThis.clearTimeout(handle as ReturnType<typeof setTimeout>),
  }
  const timeoutMs = options.timeoutMs && options.timeoutMs > 0
    ? options.timeoutMs : PWA_STARTUP_TIMEOUT_MS

  return new Promise<boolean>((resolve) => {
    let finished = false
    let activationRequested = false
    const cleanup: Array<() => void> = []
    const watched = new Set<ServiceWorker>()
    const finish = (changed: boolean) => {
      if (finished) return
      finished = true
      cleanup.forEach((dispose) => dispose())
      if (changed) {
        try {
          options.reload()
        } catch {
          resolve(false)
          return
        }
      }
      resolve(changed)
    }
    const deadline = timer.set(() => finish(false), timeoutMs)
    cleanup.push(() => timer.clear(deadline))
    const onControllerChange = () => {
      const controller = options.controllerTarget.controller
      if (controller && controller !== initialController) finish(true)
    }
    options.controllerTarget.addEventListener('controllerchange', onControllerChange)
    cleanup.push(() => options.controllerTarget.removeEventListener('controllerchange', onControllerChange))

    const watchWorker = (worker: ServiceWorker | null) => {
      if (finished || !worker || watched.has(worker)) return
      watched.add(worker)
      const onStateChange = () => {
        if (finished) return
        if (worker.state === 'redundant') {
          finish(false)
        } else if (worker.state === 'installed' && !activationRequested) {
          activationRequested = true
          try {
            worker.postMessage({ type: 'SKIP_WAITING' })
          } catch {
            finish(false)
          }
        }
      }
      worker.addEventListener('statechange', onStateChange)
      cleanup.push(() => worker.removeEventListener('statechange', onStateChange))
      onStateChange()
    }

    void (async () => {
      try {
        const registration = await options.register()
        if (finished) return
        if (!registration) {
          finish(false)
          return
        }
        onControllerChange()
        if (finished) return
        const onUpdateFound = () => {
          watchWorker(registration.waiting)
          watchWorker(registration.installing)
        }
        registration.addEventListener('updatefound', onUpdateFound)
        cleanup.push(() => registration.removeEventListener('updatefound', onUpdateFound))
        onUpdateFound()
        if (finished || activationRequested) return
        await registration.update()
        if (finished) return
        onUpdateFound()
        if (!activationRequested && !registration.installing && !registration.waiting) finish(false)
      } catch {
        finish(false)
      }
    })()
  })
}
