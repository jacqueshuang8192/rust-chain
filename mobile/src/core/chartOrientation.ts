interface OrientationLock {
  lock?(orientation: 'landscape'): Promise<void>
  unlock?(): void
}

export interface ChartOrientationEnvironment {
  fullscreenElement(): Element | null
  exitFullscreen(): Promise<void>
  orientation?: OrientationLock
  onFullscreenChange(listener: () => void): () => void
}

/** Own only the chart's fullscreen/lock, including promises resolving after Close. */
export function createChartOrientation(environment: ChartOrientationEnvironment, onExit: () => void) {
  let active: { element: HTMLElement; native: boolean } | null = null
  let locked = false
  let disposed = false
  let cleanup = Promise.resolve()
  const releaseLock = () => {
    if (!locked) return
    locked = false
    try { environment.orientation?.unlock?.() } catch { /* Unsupported WebView. */ }
  }
  const exitOwned = async (element: HTMLElement) => {
    if (environment.fullscreenElement() !== element) return
    try { await environment.exitFullscreen() } catch { /* CSS presentation still closes. */ }
  }
  const removeListener = environment.onFullscreenChange(() => {
    if (active?.native && environment.fullscreenElement() !== active.element) {
      active = null
      releaseLock()
      onExit()
    }
  })

  async function enter(element: HTMLElement): Promise<void> {
    if (disposed || active) return
    const session = { element, native: false }
    active = session
    await cleanup
    if (active !== session) return
    // Never take over another component's native fullscreen or orientation.
    if (environment.fullscreenElement() && environment.fullscreenElement() !== element) return
    try {
      if (!environment.fullscreenElement()) await element.requestFullscreen?.()
      if (active !== session) {
        if (!active) await exitOwned(element)
        return
      }
      session.native = environment.fullscreenElement() === element
      if (environment.orientation?.lock) {
        await environment.orientation.lock('landscape')
        locked = true
        if (!active) releaseLock()
      }
    } catch {
      // Orientation/fullscreen denial keeps the automatically rotated CSS view.
    }
  }

  function exit(): void {
    const session = active
    active = null
    releaseLock()
    if (session) cleanup = exitOwned(session.element)
  }

  function dispose(): void {
    disposed = true
    exit()
    removeListener()
  }

  return { enter, exit, dispose }
}
