import { requiredSafeInteger } from './numeric.ts'

export interface HistoryPage<T> {
  items: T[]
  nextOffset: number
  hasMore: boolean
}

export async function fetchAllHistoryPages<T>(
  fetchPage: (request: { limit: number; offset: number }) => Promise<HistoryPage<T>>,
  options: {
    pageSize: number
    identity: (item: T) => string
    maxPages?: number
  },
): Promise<T[]> {
  validateHistoryPageRequest(options.pageSize, 0)
  const maxPages = options.maxPages ?? 200
  requiredSafeInteger(maxPages, 'maxPages', 1)
  const items: T[] = []
  let offset = 0
  for (let pageNumber = 0; pageNumber < maxPages; pageNumber += 1) {
    const page = await fetchPage({ limit: options.pageSize, offset })
    const merged = mergeHistoryPage(items, page, options.identity)
    items.splice(0, items.length, ...merged.items)
    if (!page.hasMore || page.items.length === 0) return items
    if (page.nextOffset <= offset) throw new TypeError('history page offset did not advance')
    offset = page.nextOffset
  }
  throw new TypeError('history page count exceeded safety bound')
}

export interface HistoryPaginationState<T> {
  items: T[]
  nextOffset: number
  hasMore: boolean
  loading: boolean
  loadingMore: boolean
  initialError: unknown | null
  appendError: unknown | null
}

export type HistoryPaginationOperation = 'loaded' | 'error' | 'stale' | 'ignored'

export function validateHistoryPageRequest(limit: number, offset: number): void {
  requiredSafeInteger(limit, 'limit', 1)
  if (limit > 100) throw new TypeError('invalid history limit')
  requiredSafeInteger(offset, 'offset', 0)
}

export function mapHistoryPage<T>(
  payload: unknown,
  request: { limit: number; offset: number },
  mapItem: (item: Record<string, unknown>) => T,
): HistoryPage<T> {
  validateHistoryPageRequest(request.limit, request.offset)
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    throw new TypeError('invalid history page envelope')
  }
  const raw = payload as Record<string, unknown>
  if (!Array.isArray(raw.items)) throw new TypeError('invalid history page items')
  if (raw.items.length > request.limit) throw new TypeError('history page exceeds limit')
  if (raw.has_more !== undefined && typeof raw.has_more !== 'boolean') {
    throw new TypeError('invalid history has_more')
  }
  const items = raw.items.map((item) => {
    if (!item || typeof item !== 'object' || Array.isArray(item)) {
      throw new TypeError('invalid history item')
    }
    return mapItem(item as Record<string, unknown>)
  })
  return {
    items,
    nextOffset: request.offset + items.length,
    hasMore: raw.has_more === undefined ? items.length === request.limit : raw.has_more,
  }
}

export function mergeHistoryPage<T>(
  current: readonly T[],
  page: HistoryPage<T>,
  identity: (item: T) => string,
): { items: T[]; addedCount: number } {
  const merged = new Map<string, T>()
  for (const item of current) merged.set(identity(item), item)
  const before = merged.size
  for (const item of page.items) merged.set(identity(item), item)
  return { items: [...merged.values()], addedCount: merged.size - before }
}

export function createHistoryPaginationController<T>(input: {
  sessionKey: () => string
  fetchPage: (request: { limit: number; offset: number }) => Promise<HistoryPage<T>>
  identity: (item: T) => string
  pageSize?: number
  onChange?: (state: HistoryPaginationState<T>) => void
}): {
  snapshot: () => HistoryPaginationState<T>
  loadInitial: () => Promise<HistoryPaginationOperation>
  loadMore: () => Promise<HistoryPaginationOperation>
  retryLoadMore: () => Promise<HistoryPaginationOperation>
  reset: () => void
  stop: () => void
} {
  const pageSize = input.pageSize ?? 30
  validateHistoryPageRequest(pageSize, 0)
  let generation = 0
  let active = true
  let inFlight = false
  let state: HistoryPaginationState<T> = {
    items: [],
    nextOffset: 0,
    hasMore: true,
    loading: false,
    loadingMore: false,
    initialError: null,
    appendError: null,
  }

  const snapshot = (): HistoryPaginationState<T> => ({ ...state, items: [...state.items] })
  const publish = (next: HistoryPaginationState<T>): void => {
    state = next
    input.onChange?.(snapshot())
  }

  async function request(offset: number): Promise<HistoryPage<T> | null> {
    const requestGeneration = generation
    const session = input.sessionKey()
    if (!session || !active) return null
    inFlight = true
    try {
      const page = await input.fetchPage({ limit: pageSize, offset })
      if (!active || requestGeneration !== generation || session !== input.sessionKey()) return null
      return page
    } finally {
      if (requestGeneration === generation) inFlight = false
    }
  }

  async function loadInitial(): Promise<HistoryPaginationOperation> {
    if (!active) return 'stale'
    generation += 1
    publish({ ...state, items: [], nextOffset: 0, hasMore: true, loading: true, loadingMore: false, initialError: null, appendError: null })
    try {
      const page = await request(0)
      if (!page) return 'stale'
      const merged = mergeHistoryPage([], page, input.identity)
      publish({ ...state, ...merged, loading: false, loadingMore: false, nextOffset: page.nextOffset, hasMore: page.hasMore && merged.addedCount > 0, initialError: null, appendError: null })
      return 'loaded'
    } catch (error) {
      if (!active) return 'stale'
      publish({ ...state, loading: false, initialError: error })
      return 'error'
    }
  }

  async function append(retry: boolean): Promise<HistoryPaginationOperation> {
    if (!active || inFlight || state.loading || state.loadingMore || !state.hasMore) return 'ignored'
    if (!retry && state.appendError !== null) return 'ignored'
    if (!input.sessionKey()) return 'stale'
    const previous = state
    publish({ ...state, loadingMore: true })
    try {
      const page = await request(previous.nextOffset)
      if (!page) return 'stale'
      const merged = mergeHistoryPage(previous.items, page, input.identity)
      publish({
        ...previous,
        items: merged.items,
        loadingMore: false,
        nextOffset: page.nextOffset,
        hasMore: page.hasMore && (page.items.length === 0 || merged.addedCount > 0),
        initialError: null,
        appendError: null,
      })
      return 'loaded'
    } catch (error) {
      if (!active) return 'stale'
      publish({ ...previous, loadingMore: false, appendError: error })
      return 'error'
    }
  }

  return {
    snapshot,
    loadInitial,
    loadMore: () => append(false),
    retryLoadMore: () => append(true),
    reset(): void {
      generation += 1
      publish({ items: [], nextOffset: 0, hasMore: true, loading: false, loadingMore: false, initialError: null, appendError: null })
    },
    stop(): void {
      active = false
      generation += 1
      publish({ items: [], nextOffset: 0, hasMore: false, loading: false, loadingMore: false, initialError: null, appendError: null })
    },
  }
}
