/** Inverse of the frame's clockwise 90-degree CSS transform. */
export function rotatedChartPoint(
  point: { clientX: number; clientY: number },
  box: { left: number; top: number; right: number; width: number; height: number },
  width: number,
  height: number,
) {
  return {
    clientX: box.left + (point.clientY - box.top) * width / (box.height || 1),
    clientY: box.top + (box.right - point.clientX) * height / (box.width || 1),
  }
}

/** Adapt only chart-canvas input; leave buttons, links and the engine's gestures native. */
export function bindRotatedChartInput(host: HTMLElement, rotated: () => boolean): () => void {
  const document = host.ownerDocument
  const window = document.defaultView!
  let dragging: HTMLElement | null = null
  const types = ['mousedown', 'mousemove', 'mouseup', 'mouseenter', 'mouseleave',
    'click', 'dblclick', 'contextmenu', 'touchstart', 'touchmove', 'touchend', 'touchcancel', 'wheel']
  const processed = new WeakSet<Event>()

  function adapt(event: Event): void {
    if (!rotated()) { dragging = null; return }
    if (processed.has(event)) return
    const target = event.target instanceof window.HTMLElement ? event.target : null
    const canvas = target?.tagName === 'CANVAS' && host.contains(target) ? target : null
    const surface = dragging || canvas
    if (!surface) return
    // Wheel zoom is handled by the chart root; pane/axis gestures use their canvas.
    const receiver = event.type === 'wheel'
      ? surface.closest<HTMLElement>('.tv-lightweight-charts') || surface : surface
    const box = receiver.getBoundingClientRect()
    const coordinates = (point: { clientX: number; clientY: number }) => {
      const mapped = rotatedChartPoint(point, box, receiver.clientWidth, receiver.clientHeight)
      return { ...mapped, pageX: mapped.clientX + window.scrollX, pageY: mapped.clientY + window.scrollY }
    }
    const descriptors = (values: Record<string, unknown>) => Object.fromEntries(
      Object.entries(values).map(([key, value]) => [key, { configurable: true, value }]),
    )
    if ('touches' in event) {
      const touchEvent = event as TouchEvent
      const mapped = new Map<Touch, Touch>()
      const list = (touches: TouchList) => {
        const values = Array.from(touches, (touch) => {
          if (!mapped.has(touch)) {
            mapped.set(touch, {
              identifier: touch.identifier, target: touch.target,
              screenX: touch.screenX, screenY: touch.screenY,
              radiusX: touch.radiusX, radiusY: touch.radiusY,
              rotationAngle: touch.rotationAngle, force: touch.force, ...coordinates(touch),
            })
          }
          return mapped.get(touch)!
        })
        return Object.assign(values, { item: (index: number) => values[index] ?? null })
      }
      Object.defineProperties(event, descriptors({
        touches: list(touchEvent.touches), targetTouches: list(touchEvent.targetTouches),
        changedTouches: list(touchEvent.changedTouches),
      }))
      dragging = touchEvent.touches.length ? surface : null
    } else if ('clientX' in event) {
      Object.defineProperties(event, descriptors(coordinates(event as MouseEvent)))
      if (event.type === 'mousedown') dragging = surface
      if (event.type === 'mouseup') dragging = null
    }
    processed.add(event)
  }

  const reset = () => { dragging = null }
  for (const type of types) document.addEventListener(type, adapt, { capture: true, passive: true })
  window.addEventListener('blur', reset)
  return () => {
    for (const type of types) document.removeEventListener(type, adapt, { capture: true })
    window.removeEventListener('blur', reset)
    reset()
  }
}
