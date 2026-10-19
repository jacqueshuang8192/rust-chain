import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { fetchNewCoinProject, type NewCoinProject } from '@/api/newCoin'
import { createSessionRequestLifecycle } from '@/core/sessionRequest'

/** Planned dates never advance the project: only the public project read does. */
export function useNewCoinProject(symbol: () => string, paused: () => boolean) {
  const project = ref<NewCoinProject | null>(null)
  const refreshing = ref(false)
  const error = ref<unknown>(null)
  let timer: ReturnType<typeof setInterval> | undefined
  let active = true
  const requests = createSessionRequestLifecycle({
    sessionKey: symbol,
    request: () => fetchNewCoinProject(symbol()),
  })

  async function refresh(background = false): Promise<void> {
    if (!active || (background && (refreshing.value || paused() || document.visibilityState !== 'visible'))) return
    refreshing.value = true
    const result = await requests.load()
    if (result.state === 'stale') return
    refreshing.value = false
    // A legacy confirmation opened while the read was in flight keeps its snapshot.
    if (background && paused()) return
    if (result.state === 'loaded') {
      project.value = result.value
      error.value = null
    } else if (result.state === 'error') error.value = result.error
  }

  function onVisible(): void { void refresh(true) }
  watch(symbol, () => {
    requests.invalidate()
    project.value = null
    error.value = null
    refreshing.value = false
    void refresh()
  }, { immediate: true })
  onMounted(() => {
    timer = setInterval(onVisible, 10_000)
    document.addEventListener('visibilitychange', onVisible)
    window.addEventListener('focus', onVisible)
  })
  onBeforeUnmount(() => {
    active = false
    requests.stop()
    clearInterval(timer)
    document.removeEventListener('visibilitychange', onVisible)
    window.removeEventListener('focus', onVisible)
  })
  return { project, refreshing, error, refresh }
}
