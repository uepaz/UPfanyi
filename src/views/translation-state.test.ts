// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest'
import { desktop } from '../services/desktop'
import { state, deferred, scopes, translation } from '../test/support'
import type { Translation } from '../types/desktop'

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

describe('latest translation request', () => {
  it('keeps the new request pending when an old success completes', async () => {
    const old = deferred<Translation>()
    const current = deferred<Translation>()
    vi.mocked(desktop.translate)
      .mockReturnValueOnce(old.promise)
      .mockReturnValueOnce(current.promise)
    const ui = state()
    ui.text.value = 'one'
    const first = ui.run()
    ui.text.value = 'two'
    const second = ui.run()
    old.resolve(translation())
    await first
    expect(ui.pending.value).toBe(true)
    expect(ui.result.value).toBeNull()
    current.resolve(translation('two', '二'))
    await second
    expect(ui.result.value?.sourceText).toBe('two')
    expect(ui.pending.value).toBe(false)
  })
  it('ignores an old failure after the latest success', async () => {
    const old = deferred<Translation>()
    vi.mocked(desktop.translate)
      .mockReturnValueOnce(old.promise)
      .mockResolvedValueOnce(translation('two', '二'))
    const ui = state()
    ui.text.value = 'one'
    const first = ui.run()
    ui.text.value = 'two'
    await ui.run()
    old.reject(new Error('old failure'))
    await first
    expect(ui.result.value?.translatedText).toBe('二')
    expect(ui.error.value).toBe('')
    expect(ui.pending.value).toBe(false)
  })
  it('does not replace the latest failure with an old success', async () => {
    const old = deferred<Translation>()
    vi.mocked(desktop.translate)
      .mockReturnValueOnce(old.promise)
      .mockRejectedValueOnce(new Error('latest failure'))
    const ui = state()
    ui.text.value = 'one'
    const first = ui.run()
    ui.text.value = 'two'
    await ui.run()
    old.resolve(translation())
    await first
    expect(ui.error.value).toBe('latest failure')
    expect(ui.result.value).toBeNull()
  })
  it('debounces typing and cancels when automatic translation is disabled', async () => {
    vi.useFakeTimers()
    const ui = state(true)
    ui.text.value = 'one'
    await vi.advanceTimersByTimeAsync(200)
    ui.text.value = 'two'
    await vi.advanceTimersByTimeAsync(449)
    expect(desktop.translate).not.toHaveBeenCalled()
    ui.auto.value = false
    await vi.advanceTimersByTimeAsync(1000)
    expect(desktop.translate).not.toHaveBeenCalled()
    ui.auto.value = true
    await vi.advanceTimersByTimeAsync(450)
    expect(desktop.translate).toHaveBeenCalledExactlyOnceWith({
      text: 'two',
      sourceLang: 'auto',
      targetLang: 'en',
    })
  })
  it('cancels a queued translation when input is cleared or scope disposed', async () => {
    vi.useFakeTimers()
    const ui = state(true)
    ui.text.value = 'one'
    ui.text.value = ''
    await vi.advanceTimersByTimeAsync(600)
    expect(desktop.translate).not.toHaveBeenCalled()
    ui.text.value = 'two'
    scopes[0]!.stop()
    await vi.advanceTimersByTimeAsync(600)
    expect(desktop.translate).not.toHaveBeenCalled()
  })
  it('translates a clipboard event once even with automatic translation enabled', async () => {
    vi.useFakeTimers()
    const ui = state(true)
    await ui.setTextAndTranslate('clipboard')
    await vi.advanceTimersByTimeAsync(1000)
    expect(desktop.translate).toHaveBeenCalledTimes(1)
  })
})
