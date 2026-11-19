// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import { nextTick } from 'vue'
import { popup, targetLanguage } from '../services/desktop'
import { mockTargetLanguage } from '../test/targetLanguage'
import type { PopupSelection, PopupSnapshot } from '../types/desktop'
import SelectionPopup from './SelectionPopup.vue'

vi.mock('../services/desktop', () => ({
  targetLanguage: { get: vi.fn(), set: vi.fn(), onChange: vi.fn() },
  popup: {
    snapshot: vi.fn(),
    startDragging: vi.fn(),
    ready: vi.fn(),
    retranslate: vi.fn(),
    copy: vi.fn(),
    pin: vi.fn(),
    dismiss: vi.fn(),
    onUpdate: vi.fn(),
  },
  errorMessage: String,
}))
const original = {
  selectionId: 1,
  requestId: 1,
  locale: 'zh_CN',
  sourceText: 'Original A',
  sourceLang: 'en',
  targetLang: 'zh',
}
function success(extra: Partial<typeof original> = {}): PopupSelection {
  return { ...original, ...extra, status: 'success', translation: '译文 A', historySaved: true }
}
function snapshot(
  selection: PopupSelection | null,
  version: number,
  pinned = false,
): PopupSnapshot {
  return {
    selection: selection?.status === 'preparing' ? null : selection,
    dismissRequestId: selection?.requestId ?? null,
    version,
    pinned,
    preparing: selection?.status === 'preparing' ? selection : null,
  }
}
function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: Error) => void
  const promise = new Promise<T>((done, fail) => {
    resolve = done
    reject = fail
  })
  return { resolve, reject, promise }
}
let wrapper: VueWrapper | undefined
let notify: (version: number) => void
const cleanup = vi.fn()
async function open(value: PopupSnapshot) {
  vi.mocked(popup.snapshot).mockResolvedValue(value)
  wrapper = mount(SelectionPopup, { attachTo: document.body })
  await flushPromises()
  return wrapper
}
async function publish(value: PopupSnapshot) {
  vi.mocked(popup.snapshot).mockResolvedValue(value)
  notify(value.version)
  await flushPromises()
}
async function choose(id: string, lang: string) {
  await wrapper!.get('#' + id).trigger('click')
  document
    .getElementById(id + '-listbox')!
    .querySelector<HTMLElement>('[data-value="' + lang + '"]')!
    .click()
  await nextTick()
  await flushPromises()
}
beforeEach(() => {
  vi.resetAllMocks()
  mockTargetLanguage('zh')
  vi.mocked(popup.onUpdate).mockImplementation(async (handler) => {
    notify = handler
    return cleanup
  })
})
afterEach(() => {
  wrapper?.unmount()
  wrapper = undefined
  document.body.replaceChildren()
  vi.useRealTimers()
})

describe('selection result popup', () => {
  it.each([false, true])(
    'starts native dragging only on a primary press in the empty header (pinned=%s)',
    async (pinned) => {
      const view = await open(snapshot(success(), 1, pinned))
      await view.get('header').trigger('mousedown', { button: 0 })
      await view.get('header').trigger('mousedown', { button: 2 })
      for (const target of [
        '.pin',
        '.pin svg',
        'header button:last-child',
        '.popup-body',
        '.popup-copy',
        '#popup-source',
      ]) {
        await view.get(target).trigger('mousedown', { button: 0 })
      }
      expect(popup.startDragging).toHaveBeenCalledTimes(1)
      expect(popup.pin).not.toHaveBeenCalled()
      expect(popup.dismiss).not.toHaveBeenCalled()
    },
  )
  it('reads live progress after reload, disables copying and discards interrupted and stale previews', async () => {
    const partial = snapshot({ ...original, status: 'streaming', translation: '首段' }, 3)
    const view = await open(partial)
    expect(view.get('.popup-translation').text()).toBe('首段')
    expect(view.get('[aria-live]').attributes('aria-busy')).toBe('true')
    expect(view.get('.popup-copy').attributes('disabled')).toBeDefined()
    await view.get('.popup-copy').trigger('click')
    expect(popup.copy).not.toHaveBeenCalled()
    const slow = deferred<PopupSnapshot>()
    vi.mocked(popup.snapshot).mockReturnValueOnce(slow.promise)
    notify(4)
    await publish(
      snapshot(
        { ...original, locale: 'en_US', requestId: 5, status: 'error', error: 'incomplete' },
        6,
      ),
    )
    slow.resolve({ ...partial, version: 4 })
    await flushPromises()
    expect(view.text()).not.toContain('首段')
    expect(view.get('.popup-error').text()).toContain('stream ended')
    await publish(snapshot(success({ requestId: 7 }), 8))
    expect(view.get('.popup-copy').attributes('disabled')).toBeUndefined()
    await view.get('.popup-copy').trigger('click')
    expect(popup.copy).toHaveBeenCalledExactlyOnceWith(7)
  })
  it('subscribes before reading, renders loading before ready, and restores without another request', async () => {
    vi.mocked(popup.ready).mockImplementation(async () => {
      expect(document.querySelector('.popup-loading')?.textContent).toBe('翻译中…')
    })
    const view = await open(snapshot({ ...original, status: 'preparing' }, 1))
    expect(vi.mocked(popup.onUpdate).mock.invocationCallOrder[0]!).toBeLessThan(
      vi.mocked(popup.snapshot).mock.invocationCallOrder[0]!,
    )
    expect(popup.ready).toHaveBeenCalledExactlyOnceWith(1)
    notify(1)
    await publish(snapshot({ ...original, status: 'loading' }, 2))
    expect(view.get('.popup-copy').attributes('disabled')).toBeDefined()
    await publish(snapshot(success(), 3))
    view.unmount()
    await open(snapshot(success(), 3))
    expect(cleanup).toHaveBeenCalledOnce()
    expect(popup.ready).toHaveBeenCalledOnce()
    expect(popup.retranslate).not.toHaveBeenCalled()
  })
  it.each([
    ['zh_CN', '固定窗口', '复制译文', '选区翻译'],
    ['en_US', 'Pin window', 'Copy translation', 'Selection translation'],
  ])(
    'shows accessible icon actions and a bottom language bar for %s',
    async (locale, pin, copy, title) => {
      const value = snapshot(success({ locale }), 3)
      const view = await open(value)
      expect(view.get('main').attributes('aria-label')).toBe(title)
      expect(view.get('header').text()).toBe('')
      expect(view.get('.pin').attributes('aria-label')).toBe(pin)
      vi.mocked(popup.snapshot).mockResolvedValue({ ...value, version: 4, pinned: true })
      await view.get('.pin').trigger('click')
      await flushPromises()
      expect(popup.pin).toHaveBeenCalledExactlyOnceWith(1, true)
      expect(view.get('.pin').attributes('aria-pressed')).toBe('true')
      const button = view.get('.popup-copy')
      expect(button.attributes('aria-label')).toBe(copy)
      expect(button.text()).toBe('')
      const again = view.get('.popup-retranslate')
      expect(again.attributes('aria-label')).toBe(
        locale === 'zh_CN' ? '重新翻译' : 'Translate again',
      )
      expect(again.text()).toBe('')
      expect(
        view
          .get('.popup-actions')
          .findAll('button')
          .map((item) => item.element),
      ).toEqual([view.get('.popup-disclosure').element, button.element, again.element])
      await view.get('.popup-disclosure').trigger('click')
      expect(view.get<HTMLTextAreaElement>('.popup-original').element.value).toBe('Original A')
      vi.useFakeTimers()
      const normal = button.get('path').attributes('d')
      await button.trigger('click')
      expect(popup.copy).toHaveBeenCalledExactlyOnceWith(1)
      expect(button.get('path').attributes('d')).not.toBe(normal)
      await vi.advanceTimersByTimeAsync(1500)
      expect(button.get('path').attributes('d')).toBe(normal)
      expect(Array.from(view.get('main').element.children).map((el) => el.className)).toEqual([
        'popup-header',
        'popup-body',
        'popup-actions',
        'popup-languages',
      ])
      expect(
        view
          .get('.popup-languages')
          .findAll('button')
          .map((el) => el.attributes('id') ?? 'swap'),
      ).toEqual(['popup-source', 'swap', 'popup-target'])
    },
  )
  it('edits only the expanded source and retranslates the text at click time', async () => {
    const view = await open(snapshot(success(), 2))
    const disclosure = view.get('.popup-disclosure')
    const again = view.get('.popup-retranslate')
    const source = () => view.get<HTMLTextAreaElement>('#selection-original')
    const edited = 'Edited line 1\n第二行'
    expect(view.find('textarea').exists()).toBe(false)
    await disclosure.trigger('click')
    expect(document.activeElement).toBe(source().element)
    await source().setValue(edited)
    await disclosure.trigger('click')
    await disclosure.trigger('click')
    expect(source().element.value).toBe(edited)
    expect(view.get('.popup-translation').element.tagName).toBe('P')
    await view.get('.popup-copy').trigger('click')
    expect(popup.copy).toHaveBeenCalledExactlyOnceWith(1)
    expect(popup.retranslate).not.toHaveBeenCalled()

    const accepted = deferred<PopupSnapshot>()
    vi.mocked(popup.retranslate).mockReturnValueOnce(accepted.promise)
    await again.trigger('click')
    await again.trigger('click')
    expect(popup.retranslate).toHaveBeenCalledExactlyOnceWith(1, {
      text: edited,
      sourceLang: 'en',
      targetLang: 'zh',
    })
    expect(view.get('.popup-copy').attributes('disabled')).toBeDefined()
    const submitted = { ...original, sourceText: edited, requestId: 3 }
    await source().setValue('Next edit')
    accepted.resolve(snapshot({ ...submitted, status: 'loading' }, 3))
    await flushPromises()
    await publish(snapshot({ ...submitted, status: 'streaming', translation: '新' }, 4))
    expect(source().element.value).toBe('Next edit')
    expect(view.get('.popup-translation').text()).toBe('新')
    await publish(
      snapshot({ ...submitted, translation: '新译文', status: 'success', historySaved: true }, 5),
    )
    expect(source().element.value).toBe('Next edit')
    await disclosure.trigger('click')
    vi.mocked(popup.retranslate).mockResolvedValueOnce(
      snapshot(
        {
          ...submitted,
          sourceText: 'Next edit',
          requestId: 6,
          targetLang: 'fr',
          status: 'loading',
        },
        6,
      ),
    )
    await choose('popup-target', 'fr')
    expect(popup.retranslate).toHaveBeenLastCalledWith(3, {
      text: 'Next edit',
      sourceLang: 'en',
      targetLang: 'fr',
    })
    await publish(
      snapshot(success({ selectionId: 8, requestId: 8, sourceText: 'New selection' }), 9),
    )
    expect(view.find('textarea').exists()).toBe(false)
    await disclosure.trigger('click')
    expect(source().element.value).toBe('New selection')
  })
  it('blocks blank source and duplicate in-flight submissions but can repeat a successful translation', async () => {
    const view = await open(snapshot(success(), 2))
    const again = view.get('.popup-retranslate')
    await view.get('.popup-disclosure').trigger('click')
    await view.get('textarea').setValue(' \n ')
    expect(again.attributes('disabled')).toBeDefined()
    expect(view.get('#popup-target').attributes('disabled')).toBeUndefined()
    await again.trigger('click')
    expect(popup.retranslate).not.toHaveBeenCalled()
    await view.get('textarea').setValue(original.sourceText)
    vi.mocked(popup.retranslate).mockResolvedValueOnce(
      snapshot({ ...original, requestId: 3, status: 'loading' }, 3),
    )
    await again.trigger('click')
    await flushPromises()
    expect(popup.retranslate).toHaveBeenCalledExactlyOnceWith(1, {
      text: original.sourceText,
      sourceLang: 'en',
      targetLang: 'zh',
    })
    expect(again.attributes('disabled')).toBeDefined()
    await publish(
      snapshot({ ...original, requestId: 3, status: 'streaming', translation: '片段' }, 4),
    )
    await again.trigger('click')
    expect(popup.retranslate).toHaveBeenCalledTimes(1)
    await view.get('textarea').setValue('new input while streaming')
    expect(again.attributes('disabled')).toBeUndefined()
    await publish(snapshot(success({ requestId: 3 }), 5))
    again.element.dispatchEvent(new MouseEvent('click', { bubbles: true, detail: 2 }))
    await flushPromises()
    expect(popup.retranslate).toHaveBeenCalledTimes(1)
  })
  it('remembers a target selected with an empty source and uses it on the next manual translation', async () => {
    const view = await open(snapshot(success(), 2))
    await view.get('.popup-disclosure').trigger('click')
    await view.get('textarea').setValue(' ')
    await choose('popup-target', 'de')
    expect(targetLanguage.set).toHaveBeenCalledExactlyOnceWith('de')
    expect(popup.retranslate).not.toHaveBeenCalled()
    await view.get('textarea').setValue('New draft')
    vi.mocked(popup.retranslate).mockResolvedValueOnce(
      snapshot(success({ requestId: 3, targetLang: 'de' }), 3),
    )
    await view.get('.popup-retranslate').trigger('click')
    await flushPromises()
    expect(popup.retranslate).toHaveBeenCalledExactlyOnceWith(1, {
      text: 'New draft',
      sourceLang: 'en',
      targetLang: 'de',
    })
  })
  it('submits one complete pair for a swap and keeps current and cancelled selections local', async () => {
    const view = await open(snapshot(success(), 3))
    vi.mocked(popup.retranslate).mockResolvedValue(
      snapshot(
        { ...original, sourceLang: 'zh', targetLang: 'en', requestId: 4, status: 'loading' },
        4,
      ),
    )
    await view.get('.popup-languages .popup-icon').trigger('click')
    await flushPromises()
    expect(popup.retranslate).toHaveBeenCalledExactlyOnceWith(1, {
      text: 'Original A',
      sourceLang: 'zh',
      targetLang: 'en',
    })
    await choose('popup-target', 'en')
    const source = view.get('#popup-source')
    await source.trigger('click')
    expect(document.getElementById('popup-source-listbox')).not.toBeNull()
    await source.trigger('keydown', { key: 'Escape' })
    expect(popup.retranslate).toHaveBeenCalledTimes(1)
    await publish(snapshot(success({ selectionId: 5, requestId: 5, sourceLang: 'auto' }), 6))
    expect(view.get('.popup-languages .popup-icon').attributes('disabled')).toBeDefined()
    expect(view.get('#popup-target').attributes('data-value')).toBe('en')
  })
  it('keeps equal languages on swap and offers auto detection only for the source', async () => {
    mockTargetLanguage('en')
    const view = await open(snapshot(success({ sourceLang: 'en', targetLang: 'en' }), 3))
    await view.get('.popup-languages .popup-icon').trigger('click')
    await view.get('#popup-source').trigger('click')
    expect(document.querySelector('#popup-source-listbox [data-value="auto"]')).not.toBeNull()
    await view.get('#popup-source').trigger('keydown', { key: 'Escape' })
    await view.get('#popup-target').trigger('click')
    expect(document.querySelector('#popup-target-listbox [data-value="auto"]')).toBeNull()
    expect(popup.retranslate).not.toHaveBeenCalled()
    expect(view.get('.popup-translation').text()).toBe('译文 A')
  })
  it('keeps the latest pair while serializing rapid changes and abandons queued edits for another selection', async () => {
    const view = await open(snapshot(success(), 2))
    const first = deferred<PopupSnapshot>()
    vi.mocked(popup.retranslate).mockReturnValueOnce(first.promise)
    vi.mocked(popup.retranslate).mockResolvedValueOnce(
      snapshot(
        { ...original, sourceLang: 'fr', targetLang: 'ja', requestId: 4, status: 'loading' },
        4,
      ),
    )
    await choose('popup-source', 'fr')
    await choose('popup-target', 'ja')
    expect(view.get('#popup-target').attributes('data-value')).toBe('ja')
    expect(view.get('.popup-copy').attributes('disabled')).toBeDefined()
    expect(popup.retranslate).toHaveBeenCalledTimes(1)
    first.resolve(snapshot({ ...original, sourceLang: 'fr', requestId: 3, status: 'loading' }, 3))
    await flushPromises()
    expect(vi.mocked(popup.retranslate).mock.calls).toEqual([
      [1, { text: 'Original A', sourceLang: 'fr', targetLang: 'zh' }],
      [3, { text: 'Original A', sourceLang: 'fr', targetLang: 'ja' }],
    ])
    const old = deferred<PopupSnapshot>()
    vi.mocked(popup.retranslate).mockReturnValueOnce(old.promise)
    await choose('popup-source', 'en')
    await choose('popup-target', 'ko')
    await publish(snapshot(success({ selectionId: 9, requestId: 9, sourceText: 'B' }), 9))
    old.resolve(snapshot(success({ sourceLang: 'en', targetLang: 'ja', requestId: 5 }), 5))
    await flushPromises()
    expect(popup.retranslate).toHaveBeenCalledTimes(3)
    expect(view.get('#popup-source').attributes('data-value')).toBe('en')
    expect(view.get('#popup-target').attributes('data-value')).toBe('ko')
  })
  it.each([false, true])(
    'consumes the first Escape in the real select, then dismisses a pinned=%s popup',
    async (pinned) => {
      const view = await open(snapshot(success(), 3, pinned))
      const trigger = view.get<HTMLButtonElement>('#popup-source')
      trigger.element.focus()
      await trigger.trigger('click')
      await trigger.trigger('keydown', { key: 'Escape' })
      expect(document.getElementById('popup-source-listbox')).toBeNull()
      expect(document.activeElement).toBe(trigger.element)
      expect(popup.dismiss).not.toHaveBeenCalled()
      await trigger.trigger('keydown', { key: 'Escape', repeat: true })
      expect(popup.dismiss).not.toHaveBeenCalled()
      await trigger.trigger('keydown', { key: 'Escape' })
      expect(popup.dismiss).toHaveBeenCalledExactlyOnceWith(1)
      expect(popup.retranslate).not.toHaveBeenCalled()
    },
  )
  it('ignores late snapshot successes and failures, shows localised errors and retains a result with a history warning', async () => {
    const view = await open(snapshot(success(), 2))
    const slow = deferred<PopupSnapshot>()
    vi.mocked(popup.snapshot).mockReturnValueOnce(slow.promise)
    notify(3)
    await publish(
      snapshot({ ...original, locale: 'en_US', requestId: 4, status: 'error', error: 'model' }, 5),
    )
    slow.resolve(snapshot(success(), 3))
    await flushPromises()
    expect(view.get('.popup-error').text()).toContain('Choose a model')
    const retry = deferred<PopupSnapshot>()
    vi.mocked(popup.retranslate).mockReturnValue(retry.promise)
    await view.get('.popup-retranslate').trigger('click')
    await view.get('.popup-retranslate').trigger('click')
    expect(popup.retranslate).toHaveBeenCalledExactlyOnceWith(4, {
      text: 'Original A',
      sourceLang: 'en',
      targetLang: 'zh',
    })
    retry.resolve(snapshot({ ...original, requestId: 6, status: 'loading' }, 6))
    await publish(
      snapshot(
        { ...original, requestId: 6, status: 'success', translation: 'new', historySaved: false },
        7,
      ),
    )
    expect(view.get('.popup-warning').text()).toBe('历史未保存')
    vi.mocked(popup.copy).mockRejectedValue(new Error('private details'))
    await view.get('.popup-copy').trigger('click')
    await flushPromises()
    expect(view.get('[role="alert"]').text()).toBe('复制失败，请重试。')
    await publish(snapshot(null, 8))
    expect(view.find('main').exists()).toBe(false)
  })
  it('prepares the next loading body before ready while keeping a pinned result readable', async () => {
    const current = snapshot(success(), 3, true)
    const view = await open(current)
    const pending = {
      ...original,
      selectionId: 8,
      requestId: 8,
      locale: 'en_US',
      status: 'preparing',
    } as const
    let staged: Element | undefined
    let stagedLabel: string | null | undefined
    let visibleText: string | undefined
    vi.mocked(popup.ready).mockImplementation(async () => {
      staged = view.find('.popup-body[inert]').element
      stagedLabel = staged?.textContent
      visibleText = view.find('.popup-body:not([inert])').text()
    })
    await publish({ ...current, version: 8, preparing: pending, dismissRequestId: 8 })
    expect(popup.ready).toHaveBeenCalledExactlyOnceWith(8)
    expect(stagedLabel).toContain('Translating')
    expect(visibleText).toBe('译文 A')
    await publish(snapshot({ ...pending, status: 'loading' }, 9, true))
    expect(view.get('.popup-body').element).toBe(staged)
    expect(view.get('.pin').attributes('aria-pressed')).toBe('true')
  })
  it.each([false, true])(
    'dismisses the known candidate while a pinned result is visible, prepared=%s',
    async (prepared) => {
      const current = snapshot(success(), 3, true)
      const view = await open(current)
      const pending = { ...original, selectionId: 8, requestId: 8, status: 'preparing' } as const
      await publish({
        ...current,
        version: 9,
        dismissRequestId: 8,
        preparing: prepared ? pending : null,
      })
      if (prepared) await view.get('header button:last-child').trigger('click')
      else await view.get('main').trigger('keydown', { key: 'Escape' })
      expect(popup.dismiss).toHaveBeenCalledExactlyOnceWith(8)
    },
  )
  it('recovers a failed initial subscription before acknowledging the next selection', async () => {
    vi.useFakeTimers()
    vi.mocked(popup.onUpdate).mockRejectedValueOnce(new Error('temporary'))
    const view = await open(snapshot({ ...original, status: 'preparing' }, 1))
    expect(view.find('[role="alert"]').exists()).toBe(true)
    expect(popup.snapshot).not.toHaveBeenCalled()
    await vi.advanceTimersByTimeAsync(500)
    await flushPromises()
    expect(popup.ready).toHaveBeenCalledExactlyOnceWith(1)
    expect(view.find('[role="alert"]').exists()).toBe(false)
  })
  it('ignores a failed snapshot read after a newer language acknowledgment', async () => {
    const view = await open(snapshot(success(), 2))
    const old = deferred<PopupSnapshot>()
    vi.mocked(popup.snapshot).mockReturnValueOnce(old.promise)
    notify(3)
    vi.mocked(popup.retranslate).mockResolvedValue(
      snapshot(success({ sourceLang: 'fr', requestId: 5 }), 6),
    )
    await choose('popup-source', 'fr')
    await flushPromises()
    old.reject(new Error('old snapshot'))
    await flushPromises()
    expect(view.find('[role="alert"]').exists()).toBe(false)
    expect(view.get('#popup-source').attributes('data-value')).toBe('fr')
  })
  it('closes immediately during a pending language change and drops queued edits', async () => {
    const view = await open(snapshot(success(), 2))
    const changing = deferred<PopupSnapshot>()
    vi.mocked(popup.retranslate).mockReturnValueOnce(changing.promise)
    await choose('popup-source', 'fr')
    await choose('popup-target', 'ja')
    const newer = snapshot(success({ sourceLang: 'fr', requestId: 3 }), 4)
    vi.mocked(popup.snapshot).mockResolvedValue(newer)
    await view.get('header button:last-child').trigger('click')
    await flushPromises()
    expect(vi.mocked(popup.dismiss).mock.calls).toEqual([[1], [3]])
    changing.resolve(newer)
    await flushPromises()
    expect(popup.retranslate).toHaveBeenCalledTimes(1)
  })
  it('cleans up a subscription that resolves after unmount', async () => {
    const setup = deferred<() => void>()
    vi.mocked(popup.onUpdate).mockReturnValue(setup.promise)
    const view = mount(SelectionPopup)
    view.unmount()
    setup.resolve(cleanup)
    await flushPromises()
    expect(cleanup).toHaveBeenCalledOnce()
    expect(popup.snapshot).not.toHaveBeenCalled()
  })
})
