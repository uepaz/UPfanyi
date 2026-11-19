// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest'
import { desktop } from '../services/desktop'
import { mount, flushPromises } from '@vue/test-utils'
import App from '../App.vue'
import { bootstrap, track, button, chooseSelect, recordShortcut } from '../test/support'

vi.mock('../services/desktop', () => ({
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
      'onClipboard',
      'onNavigate',
    ].map((key) => [key, vi.fn()]),
  ),
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}))

describe('desktop startup', () => {
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
    vi.mocked(desktop.onClipboard).mock.calls[0]![0]('clipboard')
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
  it('translates the copied selection using saved defaults after manual language changes', async () => {
    const original = bootstrap()
    original.settings.defaultSourceLang = 'en'
    original.settings.defaultTargetLang = 'zh'
    vi.mocked(desktop.bootstrap).mockResolvedValue(original)
    const wrapper = track(mount(App))
    await flushPromises()
    await chooseSelect(wrapper, 'translate-source-lang', 'ja')
    await chooseSelect(wrapper, 'translate-target-lang', 'fr')
    vi.mocked(desktop.onClipboard).mock.calls[0]![0]('clipboard')
    await flushPromises()
    expect(desktop.translate).toHaveBeenCalledExactlyOnceWith({
      text: 'clipboard',
      sourceLang: 'en',
      targetLang: 'zh',
    })
  })
})
