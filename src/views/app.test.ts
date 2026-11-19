// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { desktop, popup, targetLanguage } from '../services/desktop'
import { mount, flushPromises } from '@vue/test-utils'
import App from '../App.vue'
import SelectionPopup from './SelectionPopup.vue'
import type { PopupSnapshot, TargetLanguage } from '../types/desktop'
import { mockTargetLanguage } from '../test/targetLanguage'
import { bootstrap, track, button, chooseSelect, recordShortcut, deferred } from '../test/support'

vi.mock('../services/desktop', () => ({
  targetLanguage: { get: vi.fn(), set: vi.fn(), onChange: vi.fn() },
  popup: { snapshot: vi.fn(), retranslate: vi.fn(), onUpdate: vi.fn() },
  desktop: Object.fromEntries(
    [
      'bootstrap',
      'translate',
      'testAiConnection',
      'listAiModels',
      'saveSettings',
      'listHistory',
      'deleteHistory',
      'clearHistory',
      'listFavorites',
      'saveFavorite',
      'deleteFavorite',
      'exportFavorites',
      'copyText',
      'onNavigate',
    ].map((key) => [key, vi.fn()]),
  ),
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}))

beforeEach(() => mockTargetLanguage('en'))

describe('desktop startup', () => {
  it('shares the target both ways, preserves drafts and only translates in the initiating window', async () => {
    const data = bootstrap()
    data.settings.autoTranslate = true
    vi.mocked(desktop.bootstrap).mockResolvedValue(data)
    const selection = {
      selectionId: 1,
      requestId: 1,
      locale: 'zh_CN',
      sourceText: 'Selected text',
      sourceLang: 'zh',
      targetLang: 'en',
      status: 'success' as const,
      translation: 'Original result',
      historySaved: true,
    }
    const snapshot: PopupSnapshot = {
      selection,
      preparing: null,
      version: 1,
      dismissRequestId: 1,
      pinned: true,
    }
    vi.mocked(popup.snapshot).mockResolvedValue(snapshot)
    vi.mocked(popup.onUpdate).mockResolvedValue(() => {})
    vi.mocked(popup.retranslate).mockImplementation(async (id, request) => ({
      ...snapshot,
      version: id + 1,
      dismissRequestId: id + 1,
      selection: { ...selection, ...request, sourceText: request.text, requestId: id + 1 },
    }))
    const main = track(mount(App))
    const small = track(mount(SelectionPopup))
    await flushPromises()
    await small.get('.popup-disclosure').trigger('click')
    await small.get('textarea').setValue('Edited selection')
    await chooseSelect(main, 'translate-target-lang', 'ja')
    await flushPromises()
    expect(small.get('#popup-target').attributes('data-value')).toBe('ja')
    expect(small.get<HTMLTextAreaElement>('textarea').element.value).toBe('Edited selection')
    expect(small.get('.popup-result-language').text()).toContain('英语')
    expect(small.get('.popup-translation').text()).toBe('Original result')
    expect(popup.retranslate).not.toHaveBeenCalled()
    await small.get('.popup-retranslate').trigger('click')
    await flushPromises()
    expect(popup.retranslate).toHaveBeenCalledExactlyOnceWith(1, {
      text: 'Edited selection',
      sourceLang: 'zh',
      targetLang: 'ja',
    })
    const editor = main.get<HTMLTextAreaElement>('.translate-page textarea')
    await editor.setValue('Main draft')
    await editor.trigger('keydown', { key: 'Enter' })
    await flushPromises()
    vi.mocked(desktop.translate).mockClear()
    await main.get('[aria-label="设置"]').trigger('click')
    await chooseSelect(main, 'settings-locale', 'en_US')
    await chooseSelect(small, 'popup-target', 'fr')
    await flushPromises()
    expect(main.get('#translate-target-lang').attributes('data-value')).toBe('fr')
    expect(main.get('#translate-source-lang').attributes('data-value')).toBe('auto')
    expect(main.get('#settings-locale').attributes('data-value')).toBe('en_US')
    expect(editor.element.value).toBe('Main draft')
    expect(vi.mocked(targetLanguage.set).mock.calls).toEqual([['ja'], ['fr']])
    expect(popup.retranslate).toHaveBeenCalledTimes(2)
    await new Promise((resolve) => setTimeout(resolve, 500))
    expect(desktop.translate).not.toHaveBeenCalled()
    expect(desktop.saveSettings).not.toHaveBeenCalled()
    await main.get('[aria-label="翻译"]').trigger('click')
    await editor.trigger('keydown', { key: 'Enter' })
    expect(desktop.translate).toHaveBeenCalledExactlyOnceWith(
      { text: 'Main draft', sourceLang: 'auto', targetLang: 'fr' },
      expect.any(Function),
    )
    const reopened = track(mount(App))
    await flushPromises()
    expect(reopened.get('#translate-target-lang').attributes('data-value')).toBe('fr')
  })

  it('ignores a late initial read and older events, and restores the last saved choice on failure', async () => {
    const startup = deferred<TargetLanguage>()
    vi.mocked(targetLanguage.get).mockReturnValueOnce(startup.promise)
    const main = track(mount(App))
    await flushPromises()
    const notify = vi.mocked(targetLanguage.onChange).mock.calls[0]![0]
    notify({ language: 'de', revision: 3 })
    startup.resolve({ language: 'en', revision: 0 })
    notify({ language: 'fr', revision: 2 })
    await flushPromises()
    expect(main.get('#translate-target-lang').attributes('data-value')).toBe('de')
    vi.mocked(targetLanguage.set).mockRejectedValueOnce(new Error('write failed'))
    await chooseSelect(main, 'translate-target-lang', 'ko')
    await flushPromises()
    expect(main.get('#translate-target-lang').attributes('data-value')).toBe('de')
    expect(main.get('[role="alert"]').text()).toContain('目标语言保存失败')
    expect(desktop.translate).not.toHaveBeenCalled()
  })

  it('keeps rapid clicks in order and honors a newer change that arrived before the save receipt', async () => {
    const main = track(mount(App))
    await flushPromises()
    const first = deferred<TargetLanguage>()
    const second = deferred<TargetLanguage>()
    vi.mocked(targetLanguage.set)
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise)
    await chooseSelect(main, 'translate-target-lang', 'fr')
    await chooseSelect(main, 'translate-target-lang', 'de')
    expect(main.get('#translate-target-lang').attributes('data-value')).toBe('de')
    expect(targetLanguage.set).toHaveBeenCalledExactlyOnceWith('fr')
    first.resolve({ language: 'fr', revision: 1 })
    await flushPromises()
    expect(vi.mocked(targetLanguage.set).mock.calls).toEqual([['fr'], ['de']])
    const notify = vi.mocked(targetLanguage.onChange).mock.calls[0]![0]
    notify({ language: 'ja', revision: 3 })
    second.resolve({ language: 'de', revision: 2 })
    await flushPromises()
    expect(main.get('#translate-target-lang').attributes('data-value')).toBe('ja')
    expect(desktop.translate).not.toHaveBeenCalled()
  })
  it('closes body popups when native events or settings sections switch the visible scope', async () => {
    vi.mocked(desktop.listAiModels).mockResolvedValue(['available-model'])
    const wrapper = track(mount(App))
    await flushPromises()
    await wrapper.get('#translate-source-lang').trigger('click')
    expect(document.getElementById('translate-source-lang-listbox')).not.toBeNull()
    vi.mocked(desktop.onNavigate).mock.calls[0]![0]('settings')
    await flushPromises()
    expect(document.querySelector('[role="listbox"]')).toBeNull()
    await button(wrapper, 'AI 翻译').trigger('click')
    await wrapper.get('#ai-channel').trigger('click')
    expect(document.getElementById('ai-channel-listbox')).not.toBeNull()
    await button(wrapper, '偏好').trigger('click')
    expect(document.querySelector('[role="listbox"]')).toBeNull()
    await button(wrapper, 'AI 翻译').trigger('click')
    await button(wrapper, '获取模型').trigger('click')
    await flushPromises()
    await wrapper.get('.app-combobox-toggle').trigger('click')
    expect(document.getElementById('ai-model-listbox')).not.toBeNull()
    await wrapper.get('[aria-label="翻译"]').trigger('click')
    await flushPromises()
    expect(document.querySelector('[role="listbox"]')).toBeNull()
    expect(desktop.saveSettings).not.toHaveBeenCalled()
  })
  it('guides an unconfigured first launch directly to AI setup', async () => {
    const initial = bootstrap()
    initial.aiConfig.apiKey = ''
    initial.aiConfig.model = ''
    vi.mocked(desktop.bootstrap).mockResolvedValue(initial)
    const wrapper = track(mount(App))
    await flushPromises()
    expect(wrapper.get('.setup-hint').text()).toContain('先填写 AI 服务地址、模型及所需密钥')
    expect(wrapper.findAll('.language-toolbar [role="combobox"]')).toHaveLength(2)
    await button(wrapper, '配置 AI 翻译').trigger('click')
    await flushPromises()
    expect(button(wrapper, 'AI 翻译').attributes('aria-pressed')).toBe('true')
    expect(wrapper.get('.ai-card').isVisible()).toBe(true)
    expect(desktop.translate).not.toHaveBeenCalled()
  })
  it('preserves settings drafts while navigating and only writes on explicit save', async () => {
    const wrapper = track(mount(App))
    await flushPromises()
    await wrapper.get('[aria-label="设置"]').trigger('click')
    await button(wrapper, '桌面与快捷键').trigger('click')
    await recordShortcut(wrapper, 'show-window-hotkey', {
      key: 'z',
      code: 'KeyZ',
      ctrlKey: true,
      altKey: true,
    })
    await wrapper.get('[aria-label="历史记录"]').trigger('click')
    await wrapper.get('[aria-label="设置"]').trigger('click')
    expect((wrapper.get('#show-window-hotkey').element as HTMLInputElement).value).toBe(
      'Ctrl+Alt+Z',
    )
    expect(wrapper.get('.settings-page .unsaved-badge').text()).toBe('有未保存的修改')
    expect(desktop.saveSettings).not.toHaveBeenCalled()
    await wrapper.get('#clipboard-hotkey').trigger('click')
    await wrapper.get('[aria-label="历史记录"]').trigger('click')
    await wrapper.get('#clipboard-hotkey').trigger('keydown', {
      key: 'x',
      code: 'KeyX',
      ctrlKey: true,
      altKey: true,
    })
    await wrapper.get('[aria-label="设置"]').trigger('click')
    expect((wrapper.get('#clipboard-hotkey').element as HTMLInputElement).value).toBe(
      'Ctrl+Shift+C',
    )
    await button(wrapper, '取消').trigger('click')
    expect((wrapper.get('#show-window-hotkey').element as HTMLInputElement).value).toBe(
      'Ctrl+Shift+T',
    )
  })
  it('does not query history and favorites until their pages are activated', async () => {
    const wrapper = track(mount(App))
    await flushPromises()
    expect(desktop.listHistory).not.toHaveBeenCalled()
    expect(desktop.listFavorites).not.toHaveBeenCalled()
    await wrapper.get('[aria-label="收藏词本"]').trigger('click')
    await flushPromises()
    expect(desktop.listFavorites).toHaveBeenCalledTimes(1)
    expect(desktop.listHistory).not.toHaveBeenCalled()
  })
  it('shows the desktop requirement on unsupported browser previews', async () => {
    vi.mocked(desktop.bootstrap).mockRejectedValue(new Error('DESKTOP_REQUIRED'))
    const wrapper = track(mount(App))
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('需要在 Tauri 桌面应用中运行')
    expect(wrapper.find('.translate-page').exists()).toBe(false)
  })
  it('dismisses the settings confirmation two seconds after the most recent successful save', async () => {
    vi.useFakeTimers()
    const data = bootstrap()
    const first = deferred<ReturnType<typeof bootstrap>>()
    vi.mocked(desktop.saveSettings)
      .mockReturnValueOnce(first.promise)
      .mockImplementation(async (settings, aiChannels) => ({ ...data, settings, aiChannels }))
    const wrapper = track(mount(App))
    await flushPromises()
    await wrapper.get('[aria-label="设置"]').trigger('click')
    const form = wrapper.get('.settings-form')
    const toggle = form.get('.settings-section input[role="switch"]')
    const notice = () => form.find('.settings-footer [role="status"]')

    await toggle.setValue(true)
    await form.trigger('submit')
    await vi.advanceTimersByTimeAsync(2500)
    expect(notice().exists()).toBe(false)
    first.resolve({ ...data, settings: { ...data.settings, alwaysOnTop: true } })
    await flushPromises()
    expect(notice().text()).toBe('设置已保存')

    await vi.advanceTimersByTimeAsync(1000)
    await toggle.setValue(false)
    await form.trigger('submit')
    await flushPromises()
    await vi.advanceTimersByTimeAsync(1999)
    expect(notice().text()).toBe('设置已保存')
    await vi.advanceTimersByTimeAsync(1)
    expect(notice().exists()).toBe(false)
  })
  it('changes the entire interface after saving the locale', async () => {
    const wrapper = track(mount(App))
    await flushPromises()
    await wrapper.get('[aria-label="设置"]').trigger('click')
    const saved = bootstrap()
    saved.settings.locale = 'en_US'
    vi.mocked(desktop.saveSettings).mockResolvedValue(saved)
    await chooseSelect(wrapper, 'settings-locale', 'en_US')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(wrapper.get('.app-shell').attributes('lang')).toBe('en')
    expect(wrapper.get('.notice.success').text()).toBe('Settings saved')
    expect(wrapper.find('[aria-label="History"]').exists()).toBe(true)
    expect(wrapper.get('#settings-locale').attributes('aria-label')).toBe('Interface language')
    expect(wrapper.get('#settings-locale').text()).toBe('English')
    expect(wrapper.find('#settings-source-lang').exists()).toBe(false)
    expect(wrapper.get('#translate-source-lang').attributes('aria-label')).toBe('Source language')
    await wrapper.get('#translate-target-lang').trigger('click')
    expect(document.getElementById('translate-target-lang-listbox')?.textContent).toContain(
      'English',
    )
    expect(
      document
        .getElementById('translate-target-lang-listbox')
        ?.querySelector('[data-value="auto"]'),
    ).toBeNull()
    await wrapper.get('#translate-target-lang').trigger('keydown', { key: 'Escape' })
  })
  it('preserves the main editor and its language choices when the tray opens settings', async () => {
    const original = bootstrap()
    original.settings.defaultSourceLang = 'en'
    original.settings.defaultTargetLang = 'zh'
    mockTargetLanguage('zh')
    vi.mocked(desktop.bootstrap).mockResolvedValue(original)
    const wrapper = track(mount(App))
    await flushPromises()
    await chooseSelect(wrapper, 'translate-source-lang', 'ja')
    await chooseSelect(wrapper, 'translate-target-lang', 'fr')
    const editor = wrapper.get('.translate-page textarea')
    await editor.setValue('main editor draft')
    vi.mocked(desktop.onNavigate).mock.calls[0]![0]('settings')
    await flushPromises()
    expect(desktop.translate).not.toHaveBeenCalled()
    await wrapper.get('[aria-label="翻译"]').trigger('click')
    await editor.trigger('keydown', { key: 'Enter' })
    expect(desktop.translate).toHaveBeenCalledExactlyOnceWith(
      {
        text: 'main editor draft',
        sourceLang: 'ja',
        targetLang: 'fr',
      },
      expect.any(Function),
    )
  })
})
