// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest'
import { effectScope } from 'vue'
import { useRecords } from '../composables/useRecords'
import { scopes, deferred, history } from '../test/support'
import type { HistoryRecord } from '../types/desktop'

vi.mock('../services/desktop', () => ({
  desktop: Object.fromEntries(
    [
      'bootstrap',
      'translate',
      'testAiConnection',
      'saveSettings',
      'listHistory',
      'deleteHistory',
      'clearHistory',
      'listFavorites',
      'saveFavorite',
      'deleteFavorite',
      'exportFavorites',
      'copyText',
      'onClipboard',
      'onNavigate',
    ].map((key) => [key, vi.fn()]),
  ),
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}))

describe('paged search results', () => {
  it('loads pages with the expected offset and debounces keyword search', async () => {
    vi.useFakeTimers()
    const fetch = vi.fn().mockResolvedValue({ items: [history], total: 25 })
    const scope = effectScope()
    scopes.push(scope)
    const ui = scope.run(() => useRecords<HistoryRecord>(fetch))!
    await ui.load()
    ui.next()
    await vi.advanceTimersByTimeAsync(0)
    expect(fetch).toHaveBeenLastCalledWith('', 20, 20)
    ui.keyword.value = 'first'
    await vi.advanceTimersByTimeAsync(100)
    ui.keyword.value = 'second'
    await vi.advanceTimersByTimeAsync(250)
    expect(fetch).toHaveBeenLastCalledWith('second', 20, 0)
    expect(fetch).toHaveBeenCalledTimes(3)
  })
  it('ignores old responses after a keyword changes', async () => {
    vi.useFakeTimers()
    const old = deferred<{ items: HistoryRecord[]; total: number }>()
    const fetch = vi
      .fn()
      .mockReturnValueOnce(old.promise)
      .mockResolvedValue({ items: [{ ...history, id: 2 }], total: 1 })
    const scope = effectScope()
    scopes.push(scope)
    const ui = scope.run(() => useRecords<HistoryRecord>(fetch))!
    const original = ui.load()
    ui.keyword.value = 'new'
    await vi.advanceTimersByTimeAsync(250)
    old.resolve({ items: [history], total: 1 })
    await original
    expect(ui.items.value[0]?.id).toBe(2)
  })
  it('returns to the first page after the final record on a later page is removed', async () => {
    vi.useFakeTimers()
    const fetch = vi
      .fn()
      .mockResolvedValueOnce({ items: [history], total: 21 })
      .mockResolvedValue({ items: [], total: 0 })
    const scope = effectScope()
    scopes.push(scope)
    const ui = scope.run(() => useRecords<HistoryRecord>(fetch))!
    await ui.load()
    ui.next()
    await vi.advanceTimersByTimeAsync(0)
    expect(ui.pageNumber.value).toBe(1)
    expect(ui.pages.value).toBe(1)
    expect(fetch).toHaveBeenLastCalledWith('', 20, 0)
  })
})
