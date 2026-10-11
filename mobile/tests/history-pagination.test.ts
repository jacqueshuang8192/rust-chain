import assert from 'node:assert/strict'
import test from 'node:test'
import {
  createHistoryPaginationController,
  mapHistoryPage,
  mergeHistoryPage,
} from '../src/core/historyPagination.ts'

test('history pages validate bounds, preserve offset progress, and deduplicate records', () => {
  const page = mapHistoryPage(
    { items: [{ id: 2 }, { id: 1 }], has_more: true },
    { limit: 2, offset: 4 },
    (item) => item,
  )
  assert.equal(page.nextOffset, 6)
  assert.deepEqual(mergeHistoryPage([{ id: 2 }], page, (item) => String(item.id)), {
    items: [{ id: 2 }, { id: 1 }],
    addedCount: 1,
  })
  assert.throws(() => mapHistoryPage({ items: [{ id: 1 }] }, { limit: 1.5, offset: 0 }, (item) => item), /limit/)
})

test('history controller retries append without losing loaded records and ignores stale sessions', async () => {
  let session = 'one'
  let calls = 0
  let failAppend = true
  const controller = createHistoryPaginationController({
    sessionKey: () => session,
    pageSize: 2,
    fetchPage: async ({ offset }) => {
      calls += 1
      if (offset === 0) return { items: [{ id: 3 }, { id: 2 }], nextOffset: 2, hasMore: true }
      if (failAppend) {
        failAppend = false
        throw new Error('temporary')
      }
      return { items: [{ id: 2 }, { id: 1 }], nextOffset: 4, hasMore: false }
    },
    identity: (item) => String(item.id),
  })
  assert.equal(await controller.loadInitial(), 'loaded')
  assert.equal(await controller.loadMore(), 'error')
  assert.deepEqual(controller.snapshot().items.map((item) => item.id), [3, 2])
  assert.equal(await controller.retryLoadMore(), 'loaded')
  assert.deepEqual(controller.snapshot().items.map((item) => item.id), [3, 2, 1])
  session = 'two'
  controller.reset()
  assert.equal(calls, 3)
})

test('old initial and append failures cannot replace a new authenticated history page', async () => {
  for (const append of [false, true]) {
    let session = 'old'
    let reject!: (error: Error) => void
    const controller = createHistoryPaginationController({
      sessionKey: () => session,
      fetchPage: async ({ offset }) => {
        if (session === 'new') return { items: [{ id: 9 }], nextOffset: 1, hasMore: false }
        if (append && offset === 0) return { items: [{ id: 1 }], nextOffset: 1, hasMore: true }
        return new Promise<{ items: { id: number }[]; nextOffset: number; hasMore: boolean }>((_, no) => { reject = no })
      },
      identity: (row) => String(row.id),
    })
    if (append) await controller.loadInitial()
    const old = append ? controller.loadMore() : controller.loadInitial()
    session = 'new'
    controller.reset()
    await controller.loadInitial()
    reject(new Error('old response'))
    assert.equal(await old, 'stale')
    assert.deepEqual(controller.snapshot().items, [{ id: 9 }])
    assert.equal(controller.snapshot().initialError, null)
    assert.equal(controller.snapshot().appendError, null)
    controller.stop()
  }
})
