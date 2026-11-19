// @vitest-environment jsdom
import { describe, expect, it, vi } from 'vitest'
import { desktop } from '../services/desktop'
import { UPFANYI_AI_ENDPOINT } from '../services/aiConfig'
import { mount, flushPromises, DOMWrapper, type VueWrapper } from '@vue/test-utils'
import SettingsView from './SettingsView.vue'
import {
  bootstrap,
  track,
  button,
  deferred,
  translation,
  chooseSelect,
  recordShortcut,
  fixtureChannelId,
} from '../test/support'
import type { AiChannels, Translation } from '../types/desktop'

vi.mock('../services/desktop', () => ({
  desktop: Object.fromEntries(
    [
      'bootstrap',
      'translate',
      'testAiConnection',
      'openOfficialWebsite',
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

/** 从实际模型弹层读取候选，再关闭；不绕过可编辑选择器。 */
async function modelOptions(wrapper: VueWrapper) {
  await wrapper.get('.app-combobox-toggle').trigger('click')
  const values = Array.from(document.querySelectorAll('#ai-model-listbox [role="option"]')).map(
    (item) => item.getAttribute('data-value'),
  )
  await wrapper.get('#ai-model').trigger('keydown', { key: 'Escape' })
  return values
}
function activeConfig(channels: AiChannels) {
  return channels.channels.find((item) => item.id === channels.activeId)!.config
}
function officialBootstrap() {
  const data = bootstrap()
  data.aiChannels.activeId = 'upfanyi'
  data.aiChannels.channels[0]!.config = { ...data.aiConfig, endpoint: UPFANYI_AI_ENDPOINT }
  data.aiConfig = data.aiChannels.channels[0]!.config
  return data
}
function nameDialog() {
  return new DOMWrapper(document.querySelector<HTMLElement>('.channel-name-dialog')!)
}
async function submitName(name: string) {
  const dialog = nameDialog()
  await dialog.get('input').setValue(name)
  await dialog.trigger('submit')
}
async function cancelDialog() {
  const element = document.querySelector<HTMLElement>('[aria-modal="true"]')!
  const dialog = new DOMWrapper(element)
  await dialog
    .findAll('button')
    .find((item) => item.text() === '取消')!
    .trigger('click')
}

describe('settings snapshots', () => {
  it('records only the selected shortcut and stops pending capture during explicit save', async () => {
    const original = bootstrap()
    original.settings.clipboardMonitor = true
    const pending = deferred<ReturnType<typeof bootstrap>>()
    vi.mocked(desktop.saveSettings).mockReturnValue(pending.promise)
    const wrapper = track(mount(SettingsView, { props: { data: original } }))
    await button(wrapper, '桌面与快捷键').trigger('click')
    const card = wrapper.get('#clipboard-hotkey').element.closest('section')!
    expect(card.querySelectorAll('[role="switch"]')).toHaveLength(1)
    expect(card.querySelector('input[type="number"]')).toBeNull()
    await recordShortcut(wrapper, 'clipboard-hotkey', {
      key: 'Enter',
      code: 'Enter',
      ctrlKey: true,
      altKey: true,
    })
    expect(desktop.saveSettings).not.toHaveBeenCalled()
    expect((wrapper.get('#show-window-hotkey').element as HTMLInputElement).value).toBe(
      'Ctrl+Shift+T',
    )
    expect((wrapper.get('#clipboard-hotkey').element as HTMLInputElement).value).toBe(
      'Ctrl+Alt+Enter',
    )
    await wrapper.get('#show-window-hotkey').trigger('click')
    await button(wrapper, '偏好').trigger('click')
    await wrapper.get('#show-window-hotkey').trigger('keydown', { key: 'z', code: 'KeyZ' })
    await button(wrapper, '桌面与快捷键').trigger('click')
    expect((wrapper.get('#show-window-hotkey').element as HTMLInputElement).value).toBe(
      'Ctrl+Shift+T',
    )
    await wrapper.get('#show-window-hotkey').trigger('click')
    await wrapper.get('form').trigger('submit')
    expect(wrapper.findAll('.shortcut-recorder.recording')).toHaveLength(0)
    expect(wrapper.get('#show-window-hotkey').attributes('disabled')).toBeDefined()
    expect(wrapper.get('#clipboard-hotkey').attributes('disabled')).toBeDefined()
    const [settings, channels] = vi.mocked(desktop.saveSettings).mock.calls[0]!
    expect(settings).toEqual({ ...original.settings, hotkeyClipboardTranslate: 'Ctrl+Alt+Enter' })
    expect(channels).toEqual(original.aiChannels)
    expect(original.settings.hotkeyClipboardTranslate).toBe('Ctrl+Shift+C')
    pending.resolve({ ...original, settings })
    await flushPromises()
    expect(wrapper.emitted('saved')).toHaveLength(1)
    await wrapper.setProps({ data: { ...original, settings } })
    expect(wrapper.find('.settings-page .unsaved-badge').exists()).toBe(false)
  })
  it('shows only official and add action on first start without requesting or saving', async () => {
    const data = officialBootstrap()
    data.aiConfig.apiKey = ''
    data.aiConfig.model = ''
    data.aiChannels.channels = data.aiChannels.channels.filter((item) => item.id === 'upfanyi')
    const wrapper = track(mount(SettingsView, { props: { data } }))
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe('upfanyi')
    expect(wrapper.get('#ai-interface').attributes('data-value')).toBe('openai')
    expect(wrapper.find('#ai-endpoint').exists()).toBe(false)
    expect(wrapper.find('.official-website-button').exists()).toBe(true)
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe('')
    expect((wrapper.get('#ai-model').element as HTMLInputElement).value).toBe('')
    expect(wrapper.find('.settings-page .unsaved-badge').exists()).toBe(false)
    expect(wrapper.find('.channel-actions').exists()).toBe(false)
    await wrapper.get('#ai-channel').trigger('click')
    expect(
      Array.from(document.querySelectorAll('#ai-channel-listbox [role="option"]')).map(
        (item) => item.textContent,
      ),
    ).toEqual(['UP翻译官方 AI', '添加渠道'])
    await wrapper.get('#ai-channel').trigger('keydown', { key: 'Escape' })
    expect(desktop.listAiModels).not.toHaveBeenCalled()
    expect(desktop.testAiConnection).not.toHaveBeenCalled()
    expect(desktop.saveSettings).not.toHaveBeenCalled()
  })
  it('shows the native data directory without an obsolete import status', () => {
    const data = bootstrap()
    const wrapper = track(mount(SettingsView, { props: { data } }))
    const dataInfo = wrapper.get('.data-info')
    expect(data).not.toHaveProperty('legacyImported')
    expect(dataInfo.get('summary').text()).toBe('本地数据目录')
    expect(dataInfo.get('code').text()).toBe('C:\\upfanyi-tests\\data-v3')
    expect(dataInfo.findAll('p')).toHaveLength(0)
    expect(wrapper.text()).not.toContain('已导入旧版本数据')
    const preferences = wrapper.get('.settings-section')
    expect(preferences.findAll('.settings-card')).toHaveLength(1)
    expect(preferences.get('h2').text()).toBe('通用偏好')
    expect(preferences.findAll('[role="switch"]')).toHaveLength(1)
    expect(wrapper.find('#settings-source-lang').exists()).toBe(false)
    expect(wrapper.find('#settings-target-lang').exists()).toBe(false)
  })
  it('retains draft values between sections and shows save failures beside the save controls', async () => {
    vi.mocked(desktop.saveSettings).mockRejectedValue(new Error('disk full'))
    const data = bootstrap()
    Object.assign(data.settings, {
      defaultSourceLang: 'ja',
      defaultTargetLang: 'fr',
      autoTranslate: true,
    })
    const wrapper = track(mount(SettingsView, { props: { data } }))
    await button(wrapper, '桌面与快捷键').trigger('click')
    await recordShortcut(wrapper, 'show-window-hotkey', {
      key: 'z',
      code: 'KeyZ',
      ctrlKey: true,
      altKey: true,
    })
    await button(wrapper, 'AI 翻译').trigger('click')
    await wrapper.get('input[type="password"]').setValue('new-secret')
    await button(wrapper, '偏好').trigger('click')
    await chooseSelect(wrapper, 'settings-locale', 'en_US')
    await wrapper.get('.settings-section input[role="switch"]').setValue(true)
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    const call = vi.mocked(desktop.saveSettings).mock.calls[0]!
    expect(call[0].hotkeyShowWindow).toBe('Ctrl+Alt+Z')
    expect(call[0]).toMatchObject({
      locale: 'en_US',
      defaultSourceLang: 'ja',
      defaultTargetLang: 'fr',
      autoTranslate: true,
      alwaysOnTop: true,
    })
    expect(activeConfig(call[1]).apiKey).toBe('new-secret')
    expect(wrapper.get('.settings-footer [role="alert"]').text()).toContain('disk full')
    expect(wrapper.find('.settings-page .unsaved-badge').exists()).toBe(true)
  })
  it('opens the corresponding section for invalid hidden draft fields', async () => {
    const data = bootstrap()
    data.settings.hotkeyClipboardTranslate = ''
    const wrapper = track(mount(SettingsView, { props: { data }, attachTo: document.body }))
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(button(wrapper, '桌面与快捷键').attributes('aria-pressed')).toBe('true')
    expect(document.activeElement?.id).toBe('clipboard-hotkey')
    expect(wrapper.get('.settings-footer [role="alert"]').text()).toContain('快捷键')
    expect(desktop.saveSettings).not.toHaveBeenCalled()
  })
  it('cancels edits without writing settings or mutating bootstrap', async () => {
    const original = bootstrap()
    const wrapper = track(mount(SettingsView, { props: { data: original } }))
    await recordShortcut(wrapper, 'show-window-hotkey', {
      key: 'z',
      code: 'KeyZ',
      ctrlKey: true,
      altKey: true,
    })
    await wrapper.get('input[type="password"]').setValue('changed-secret')
    await chooseSelect(wrapper, 'settings-locale', 'en_US')
    await wrapper.get('.settings-section input[role="switch"]').setValue(true)
    await button(wrapper, '取消').trigger('click')
    expect((wrapper.get('#show-window-hotkey').element as HTMLInputElement).value).toBe(
      'Ctrl+Shift+T',
    )
    expect((wrapper.get('input[type="password"]').element as HTMLInputElement).value).toBe(
      'original-secret',
    )
    expect(original.aiConfig.apiKey).toBe('original-secret')
    expect(wrapper.get('#settings-locale').attributes('data-value')).toBe('zh_CN')
    expect(
      (wrapper.get('.settings-section input[role="switch"]').element as HTMLInputElement).checked,
    ).toBe(false)
    expect(original.settings.defaultSourceLang).toBe('auto')
    expect(desktop.saveSettings).not.toHaveBeenCalled()
  })
  it('uses an independent connection-test snapshot and ignores edited results', async () => {
    const pending = deferred<Translation>()
    vi.mocked(desktop.testAiConnection).mockReturnValue(pending.promise)
    const wrapper = track(mount(SettingsView, { props: { data: bootstrap() } }))
    await button(wrapper, '测试连接').trigger('click')
    await wrapper.get('input[type="password"]').setValue('new-secret')
    expect(vi.mocked(desktop.testAiConnection).mock.calls[0]![0].apiKey).toBe('original-secret')
    pending.resolve(translation())
    await flushPromises()
    expect(wrapper.text()).not.toContain('连接成功')
    expect(desktop.saveSettings).not.toHaveBeenCalled()
  })
  it('ignores a connection-test completion after cancelling the identical configuration', async () => {
    const pending = deferred<Translation>()
    vi.mocked(desktop.testAiConnection).mockReturnValue(pending.promise)
    const wrapper = track(mount(SettingsView, { props: { data: bootstrap() } }))
    await button(wrapper, '测试连接').trigger('click')
    await button(wrapper, '取消').trigger('click')
    pending.resolve(translation())
    await flushPromises()
    expect(wrapper.text()).not.toContain('连接成功')
    expect(desktop.saveSettings).not.toHaveBeenCalled()
  })
  it('invalidates a test on edits even when the original value is restored', async () => {
    const pending = deferred<Translation>()
    vi.mocked(desktop.testAiConnection).mockReturnValue(pending.promise)
    const wrapper = track(mount(SettingsView, { props: { data: bootstrap() } }))
    await button(wrapper, '测试连接').trigger('click')
    await wrapper.get('#ai-model').setValue('changed-model')
    await wrapper.get('#ai-model').setValue('fixture-model')
    pending.reject(new Error('outdated failure'))
    await flushPromises()
    expect(wrapper.text()).not.toContain('outdated failure')
    expect(button(wrapper, '测试连接').attributes('disabled')).toBeUndefined()
  })
  it('keeps a new connection test pending when an edited older test completes', async () => {
    const old = deferred<Translation>()
    const current = deferred<Translation>()
    vi.mocked(desktop.testAiConnection)
      .mockReturnValueOnce(old.promise)
      .mockReturnValueOnce(current.promise)
    const wrapper = track(mount(SettingsView, { props: { data: bootstrap() } }))
    await button(wrapper, '测试连接').trigger('click')
    await wrapper.get('#ai-model').setValue('changed-model')
    await button(wrapper, '测试连接').trigger('click')
    old.resolve(translation('old', 'outdated translation'))
    await flushPromises()
    expect(button(wrapper, '测试中…').attributes('disabled')).toBeDefined()
    expect(wrapper.text()).not.toContain('outdated translation')
    current.resolve(translation('new', 'current translation'))
    await flushPromises()
    expect(wrapper.get('.notice.success').text()).toContain('current translation')
    expect(desktop.saveSettings).not.toHaveBeenCalled()
  })
  it('keeps edited values and reports failed persistence', async () => {
    vi.mocked(desktop.saveSettings).mockRejectedValue(new Error('disk full'))
    const wrapper = track(mount(SettingsView, { props: { data: bootstrap() } }))
    await recordShortcut(wrapper, 'show-window-hotkey', {
      key: 'z',
      code: 'KeyZ',
      ctrlKey: true,
      altKey: true,
    })
    await wrapper.get('#ai-api-key').setValue('user-failed-draft')
    await chooseSelect(wrapper, 'ai-channel', 'upfanyi')
    await wrapper.get('#ai-api-key').setValue('official-failed-draft')
    await chooseSelect(wrapper, 'ai-channel', fixtureChannelId)
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(wrapper.get('[role="alert"]').text()).toContain('disk full')
    expect(wrapper.text()).not.toContain('设置已保存')
    expect(wrapper.emitted('saved')).toBeUndefined()
    expect((wrapper.get('#show-window-hotkey').element as HTMLInputElement).value).toBe(
      'Ctrl+Alt+Z',
    )
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe('user-failed-draft')
    await chooseSelect(wrapper, 'ai-channel', 'upfanyi')
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe(
      'official-failed-draft',
    )
  })
  it('preserves an explicitly cleared credential in the saved snapshot', async () => {
    vi.mocked(desktop.saveSettings).mockResolvedValue(bootstrap())
    const wrapper = track(mount(SettingsView, { props: { data: bootstrap() } }))
    await wrapper.get('input[type="password"]').setValue('')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(activeConfig(vi.mocked(desktop.saveSettings).mock.calls[0]![1]).apiKey).toBe('')
  })
  it('fetches draft models without selecting one, saving, translating or altering connection success', async () => {
    const original = bootstrap()
    original.aiConfig.model = ''
    vi.mocked(desktop.testAiConnection).mockResolvedValue(translation())
    vi.mocked(desktop.listAiModels).mockResolvedValue(['first-model', 'second-model'])
    const wrapper = track(mount(SettingsView, { props: { data: original } }))
    await wrapper.get('#ai-api-key').setValue('draft-key')
    await button(wrapper, '测试连接').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('连接成功')
    await button(wrapper, '获取模型').trigger('click')
    await flushPromises()
    expect(vi.mocked(desktop.listAiModels).mock.calls[0]![0]).toEqual({
      ...original.aiConfig,
      apiKey: 'draft-key',
    })
    expect(await modelOptions(wrapper)).toEqual(['first-model', 'second-model'])
    expect((wrapper.get('#ai-model').element as HTMLInputElement).value).toBe('')
    expect(wrapper.get('#ai-model-status').text()).toContain('2')
    expect(wrapper.text()).toContain('连接成功')
    expect(desktop.saveSettings).not.toHaveBeenCalled()
    expect(desktop.translate).not.toHaveBeenCalled()
    expect(original.aiConfig.apiKey).toBe('original-secret')
  })
  it('shows loading, empty and failure feedback, permits retries and keeps the manual model', async () => {
    const pending = deferred<string[]>()
    vi.mocked(desktop.listAiModels)
      .mockReturnValueOnce(pending.promise)
      .mockRejectedValueOnce(new Error('HTTP 401'))
      .mockResolvedValueOnce(['chosen'])
    const wrapper = track(mount(SettingsView, { props: { data: bootstrap() } }))
    await button(wrapper, '获取模型').trigger('click')
    expect(button(wrapper, '获取中…').attributes('disabled')).toBeDefined()
    pending.resolve([])
    await flushPromises()
    expect(wrapper.get('#ai-model-status').text()).toContain('服务未返回模型')
    await button(wrapper, '获取模型').trigger('click')
    await flushPromises()
    expect(wrapper.get('#ai-model-status [role="alert"]').text()).toContain('HTTP 401')
    expect(wrapper.get('#ai-model-status').text()).toContain('手动填写')
    await wrapper.get('#ai-model').setValue('unlisted-model')
    await button(wrapper, '重新获取').trigger('click')
    await flushPromises()
    expect(await modelOptions(wrapper)).toHaveLength(1)
    expect((wrapper.get('#ai-model').element as HTMLInputElement).value).toBe('unlisted-model')
    await button(wrapper, '取消').trigger('click')
    expect((wrapper.get('#ai-model').element as HTMLInputElement).value).toBe('fixture-model')
    expect(await modelOptions(wrapper)).toHaveLength(0)
    expect(desktop.saveSettings).not.toHaveBeenCalled()
  })
  it('cancels a pending list and ignores responses after bootstrap reload', async () => {
    const cancelled = deferred<string[]>()
    const reloaded = deferred<string[]>()
    vi.mocked(desktop.listAiModels)
      .mockReturnValueOnce(cancelled.promise)
      .mockReturnValueOnce(reloaded.promise)
    const wrapper = track(mount(SettingsView, { props: { data: bootstrap() } }))
    await button(wrapper, '获取模型').trigger('click')
    await button(wrapper, '取消').trigger('click')
    await button(wrapper, '获取模型').trigger('click')
    cancelled.resolve(['cancelled'])
    await flushPromises()
    expect(button(wrapper, '获取中…').attributes('disabled')).toBeDefined()
    await wrapper.setProps({ data: bootstrap() })
    reloaded.reject(new Error('outdated failure'))
    await flushPromises()
    expect(wrapper.get('#ai-model-status').text()).toBe('')
    expect(await modelOptions(wrapper)).toHaveLength(0)
    expect(button(wrapper, '获取模型').attributes('disabled')).toBeUndefined()
  })
  it('saves the Responses mode selected in the draft', async () => {
    vi.mocked(desktop.saveSettings).mockResolvedValue(bootstrap())
    const wrapper = track(mount(SettingsView, { props: { data: bootstrap() } }))
    await chooseSelect(wrapper, 'ai-interface', 'openai-responses')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(activeConfig(vi.mocked(desktop.saveSettings).mock.calls[0]![1])).toMatchObject({
      protocol: 'openai-responses',
      endpoint: 'https://api.openai.com',
    })
  })
  it('fetches and saves a domain-only draft without adding request paths or changing its base on an API switch', async () => {
    const original = bootstrap()
    vi.mocked(desktop.listAiModels).mockResolvedValue(['available-model'])
    vi.mocked(desktop.saveSettings).mockResolvedValue(bootstrap())
    const wrapper = track(mount(SettingsView, { props: { data: original } }))
    const address = 'relay.example.com:8443/v1?tenant=test'
    await wrapper.get('#ai-endpoint').setValue(address)
    await wrapper.get('#ai-model').setValue('')
    await button(wrapper, '获取模型').trigger('click')
    await flushPromises()
    expect(vi.mocked(desktop.listAiModels).mock.calls[0]![0]).toEqual({
      ...original.aiConfig,
      endpoint: address,
      model: '',
    })
    expect((wrapper.get('#ai-endpoint').element as HTMLInputElement).value).toBe(address)
    await chooseSelect(wrapper, 'ai-interface', 'openai-responses')
    expect((wrapper.get('#ai-endpoint').element as HTMLInputElement).value).toBe(address)
    await wrapper.get('#ai-model').setValue('available-model')
    expect(wrapper.get('.ai-state').text()).toBe('已配置')
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(activeConfig(vi.mocked(desktop.saveSettings).mock.calls[0]![1])).toEqual({
      ...original.aiConfig,
      endpoint: address,
      protocol: 'openai-responses',
      model: 'available-model',
    })
    expect(original.aiConfig.endpoint).toBe('https://api.openai.com')
  })
  it('uses only official draft credentials for explicit requests and retains the five-field saved configuration', async () => {
    const original = bootstrap()
    const saved = officialBootstrap()
    Object.assign(saved.aiConfig, {
      protocol: 'openai-responses',
      apiKey: 'official-fixture-key',
      model: 'official-fixture-model',
    })
    vi.mocked(desktop.listAiModels).mockResolvedValue(['official-fixture-model'])
    vi.mocked(desktop.testAiConnection).mockResolvedValue(translation('test', 'official-result'))
    vi.mocked(desktop.saveSettings).mockResolvedValue(saved)
    const wrapper = track(mount(SettingsView, { props: { data: original } }))
    await chooseSelect(wrapper, 'ai-channel', 'upfanyi')
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe('')
    expect((wrapper.get('#ai-model').element as HTMLInputElement).value).toBe('')
    expect(wrapper.find('#ai-endpoint').exists()).toBe(false)
    expect(wrapper.find('.official-website-button').exists()).toBe(true)
    expect(desktop.listAiModels).not.toHaveBeenCalled()
    expect(desktop.testAiConnection).not.toHaveBeenCalled()
    expect(desktop.saveSettings).not.toHaveBeenCalled()
    await chooseSelect(wrapper, 'ai-interface', 'openai-responses')
    await wrapper.get('#ai-api-key').setValue('official-fixture-key')
    await button(wrapper, '获取模型').trigger('click')
    await flushPromises()
    expect(vi.mocked(desktop.listAiModels).mock.calls[0]![0]).toEqual({
      ...saved.aiConfig,
      model: '',
    })
    await wrapper.get('#ai-model').setValue('official-fixture-model')
    await button(wrapper, '测试连接').trigger('click')
    await flushPromises()
    expect(vi.mocked(desktop.testAiConnection).mock.calls[0]![0]).toEqual(saved.aiConfig)
    await wrapper.get('form').trigger('submit')
    await flushPromises()
    expect(vi.mocked(desktop.saveSettings).mock.calls[0]![1]).toEqual(saved.aiChannels)
    expect(wrapper.emitted('saved')?.[0]?.[0]).toEqual(saved)
    await wrapper.setProps({ data: saved })
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe('upfanyi')
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe(
      'official-fixture-key',
    )
    expect(Object.keys(saved.aiConfig).sort()).toEqual([
      'apiKey',
      'endpoint',
      'model',
      'protocol',
      'systemPrompt',
    ])
    expect(original.aiConfig.apiKey).toBe('original-secret')
    expect(original.aiConfig.endpoint).toBe('https://api.openai.com')
    expect(desktop.translate).not.toHaveBeenCalled()
  })
  it('preserves all channel drafts on switching and restores the saved selection when cancelling', async () => {
    const original = officialBootstrap()
    const wrapper = track(mount(SettingsView, { props: { data: original } }))
    await chooseSelect(wrapper, 'ai-channel', 'upfanyi')
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe('original-secret')
    expect((wrapper.get('#ai-model').element as HTMLInputElement).value).toBe('fixture-model')
    await chooseSelect(wrapper, 'ai-channel', fixtureChannelId)
    expect((wrapper.get('#ai-endpoint').element as HTMLInputElement).value).toBe(
      'https://api.openai.com',
    )
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe('original-secret')
    expect((wrapper.get('#ai-model').element as HTMLInputElement).value).toBe('fixture-model')
    await wrapper.get('#ai-api-key').setValue('user-draft-key')
    await chooseSelect(wrapper, 'ai-channel', 'upfanyi')
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe('original-secret')
    await wrapper.get('#ai-api-key').setValue('official-draft-key')
    await chooseSelect(wrapper, 'ai-channel', fixtureChannelId)
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe('user-draft-key')
    expect((wrapper.get('#ai-system-prompt').element as HTMLTextAreaElement).value).toBe(
      original.aiConfig.systemPrompt,
    )
    await button(wrapper, '取消').trigger('click')
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe('upfanyi')
    expect(wrapper.find('#ai-endpoint').exists()).toBe(false)
    expect(wrapper.find('.official-website-button').exists()).toBe(true)
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe('original-secret')
    expect((wrapper.get('#ai-model').element as HTMLInputElement).value).toBe('fixture-model')
    expect(desktop.saveSettings).not.toHaveBeenCalled()
    expect(desktop.listAiModels).not.toHaveBeenCalled()
    expect(desktop.testAiConnection).not.toHaveBeenCalled()
  })
  it.each(['success', 'failure'] as const)(
    'ignores stale model and connection %s after selecting another channel',
    async (outcome) => {
      const oldModels = deferred<string[]>()
      const oldTest = deferred<Translation>()
      const currentModels = deferred<string[]>()
      const currentTest = deferred<Translation>()
      vi.mocked(desktop.listAiModels)
        .mockReturnValueOnce(oldModels.promise)
        .mockReturnValueOnce(currentModels.promise)
      vi.mocked(desktop.testAiConnection)
        .mockReturnValueOnce(oldTest.promise)
        .mockReturnValueOnce(currentTest.promise)
      const data = bootstrap()
      const secondId = '22222222-2222-4222-8222-222222222222'
      data.aiChannels.channels.push({
        id: secondId,
        name: 'Same configuration',
        config: { ...data.aiConfig },
      })
      const wrapper = track(mount(SettingsView, { props: { data } }))
      await button(wrapper, '获取模型').trigger('click')
      await button(wrapper, '测试连接').trigger('click')
      await chooseSelect(wrapper, 'ai-channel', secondId)
      await chooseSelect(wrapper, 'ai-channel', fixtureChannelId)
      await button(wrapper, '获取模型').trigger('click')
      await button(wrapper, '测试连接').trigger('click')
      if (outcome === 'success') {
        oldModels.resolve(['outdated-model'])
        oldTest.resolve(translation('old', 'outdated-translation'))
      } else {
        oldModels.reject(new Error('outdated-model-error'))
        oldTest.reject(new Error('outdated-connection-error'))
      }
      await flushPromises()
      expect(wrapper.text()).not.toContain('outdated')
      expect(await modelOptions(wrapper)).toHaveLength(0)
      expect(button(wrapper, '获取中…').attributes('disabled')).toBeDefined()
      expect(button(wrapper, '测试中…').attributes('disabled')).toBeDefined()
      currentModels.resolve(['official-fixture-model'])
      currentTest.resolve(translation('new', 'official-result'))
      await flushPromises()
      expect(await modelOptions(wrapper)).toEqual(['official-fixture-model'])
      expect(wrapper.get('.notice.success').text()).toContain('official-result')
      expect(desktop.saveSettings).not.toHaveBeenCalled()
    },
  )
  it('keeps a user channel identity and address editor even when its address is the official domain', async () => {
    const wrapper = track(
      mount(SettingsView, { props: { data: bootstrap() }, attachTo: document.body }),
    )
    await button(wrapper, 'AI 翻译').trigger('click')
    const endpoint = wrapper.get('#ai-endpoint')
    await endpoint.trigger('focus')
    await endpoint.setValue('uptoken.top')
    expect(wrapper.find('#ai-endpoint').exists()).toBe(true)
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(fixtureChannelId)
    expect(wrapper.find('.official-website-button').exists()).toBe(false)
    await endpoint.setValue('uptoken.top.example')
    await endpoint.trigger('blur')
    expect(wrapper.get('#ai-endpoint').element).toBe(endpoint.element)
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(fixtureChannelId)
    await endpoint.trigger('focus')
    await endpoint.setValue('uptoken.top')
    await endpoint.setValue('uptoken.top/v1')
    expect(wrapper.find('#ai-endpoint').exists()).toBe(true)
    await endpoint.trigger('blur')
    expect(wrapper.find('#ai-endpoint').exists()).toBe(true)
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(fixtureChannelId)
    await button(wrapper, '取消').trigger('click')
    expect((wrapper.get('#ai-endpoint').element as HTMLInputElement).value).toBe(
      'https://api.openai.com',
    )
    expect(wrapper.find('.official-website-button').exists()).toBe(false)
    expect(desktop.openOfficialWebsite).not.toHaveBeenCalled()
  })
  it('opens the official website without editing configuration or disturbing loaded model and connection results', async () => {
    const data = officialBootstrap()
    vi.mocked(desktop.listAiModels).mockResolvedValue(['available-model'])
    vi.mocked(desktop.testAiConnection).mockResolvedValue(translation('test', 'connection-result'))
    vi.mocked(desktop.openOfficialWebsite).mockResolvedValue()
    const wrapper = track(mount(SettingsView, { props: { data } }))
    await button(wrapper, '获取模型').trigger('click')
    await button(wrapper, '测试连接').trigger('click')
    await flushPromises()
    await button(wrapper, '访问官方网站').trigger('click')
    await flushPromises()
    expect(desktop.openOfficialWebsite).toHaveBeenCalledExactlyOnceWith()
    expect(wrapper.find('.unsaved-badge').exists()).toBe(false)
    expect(await modelOptions(wrapper)).toEqual(['available-model'])
    expect(wrapper.get('.notice.success').text()).toContain('connection-result')
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe(
      data.aiConfig.apiKey,
    )
    expect((wrapper.get('#ai-model').element as HTMLInputElement).value).toBe(data.aiConfig.model)
    expect(desktop.listAiModels).toHaveBeenCalledTimes(1)
    expect(desktop.testAiConnection).toHaveBeenCalledTimes(1)
    expect(desktop.saveSettings).not.toHaveBeenCalled()
    expect(desktop.translate).not.toHaveBeenCalled()
  })
  it('closes open menus and disables all setting selectors and website during saving', async () => {
    const data = officialBootstrap()
    const pending = deferred<ReturnType<typeof bootstrap>>()
    vi.mocked(desktop.saveSettings).mockReturnValue(pending.promise)
    const wrapper = track(mount(SettingsView, { props: { data } }))
    await wrapper.get('#ai-model').setValue('changed-model')
    await wrapper.get('#ai-channel').trigger('click')
    expect(document.getElementById('ai-channel-listbox')).not.toBeNull()
    await wrapper.get('#settings-locale').trigger('click')
    expect(document.getElementById('settings-locale-listbox')).not.toBeNull()
    await wrapper.get('form').trigger('submit')
    expect(document.getElementById('ai-channel-listbox')).toBeNull()
    expect(document.getElementById('settings-locale-listbox')).toBeNull()
    for (const id of ['ai-channel', 'ai-protocol', 'ai-interface', 'settings-locale']) {
      expect(wrapper.get(`#${id}`).attributes('disabled')).toBeDefined()
      await wrapper.get(`#${id}`).trigger('click')
      expect(document.getElementById(`${id}-listbox`)).toBeNull()
    }
    expect(wrapper.get('.official-website-button').attributes('disabled')).toBeDefined()
    await wrapper.get('.official-website-button').trigger('click')
    expect(desktop.openOfficialWebsite).not.toHaveBeenCalled()
    pending.resolve(data)
    await flushPromises()
  })
})

describe('named channel drafts', () => {
  it('cancels adding without changing candidates, then creates, renames and saves independent configurations', async () => {
    const initial = bootstrap()
    vi.mocked(desktop.listAiModels).mockResolvedValue(['retained-model'])
    vi.mocked(desktop.testAiConnection).mockResolvedValue(translation('test', 'retained-result'))
    vi.mocked(desktop.saveSettings).mockImplementation(async (settings, aiChannels) => ({
      ...initial,
      settings,
      aiChannels,
      aiConfig: activeConfig(aiChannels),
    }))
    const wrapper = track(
      mount(SettingsView, { props: { data: initial }, attachTo: document.body }),
    )
    await button(wrapper, 'AI 翻译').trigger('click')
    await button(wrapper, '获取模型').trigger('click')
    await button(wrapper, '测试连接').trigger('click')
    await flushPromises()
    await chooseSelect(wrapper, 'ai-channel', 'add-channel')
    expect(document.activeElement).toBe(nameDialog().get('input').element)
    await cancelDialog()
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(fixtureChannelId)
    expect(await modelOptions(wrapper)).toEqual(['retained-model'])
    expect(wrapper.get('.notice.success').text()).toContain('retained-result')
    expect(wrapper.find('.unsaved-badge').exists()).toBe(false)
    await chooseSelect(wrapper, 'ai-channel', 'add-channel')
    await submitName('  ollama  ')
    expect(wrapper.get('#ai-channel').text()).toBe('ollama')
    const newId = wrapper.get('#ai-channel').attributes('data-value')!
    expect(newId).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/)
    for (const id of ['ai-endpoint', 'ai-api-key', 'ai-model'])
      expect((wrapper.get(`#${id}`).element as HTMLInputElement).value).toBe('')
    expect(wrapper.get('#ai-interface').attributes('data-value')).toBe('openai')
    expect((wrapper.get('#ai-system-prompt').element as HTMLTextAreaElement).value).toContain(
      'You are a professional translator.',
    )
    await wrapper.get('#ai-endpoint').setValue('relay.example.com')
    await wrapper.get('#ai-api-key').setValue('work-fictional-key')
    await wrapper.get('#ai-model').setValue('manual-work-model')
    await wrapper.get('#ai-system-prompt').setValue('Work prompt')
    await chooseSelect(wrapper, 'ai-interface', 'openai-responses')
    await button(wrapper, '重命名渠道').trigger('click')
    await submitName('Renamed relay')
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(newId)
    await chooseSelect(wrapper, 'ai-channel', fixtureChannelId)
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe('original-secret')
    await wrapper.get('#ai-api-key').setValue('')
    await chooseSelect(wrapper, 'ai-channel', newId)
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe(
      'work-fictional-key',
    )
    expect((wrapper.get('#ai-model').element as HTMLInputElement).value).toBe('manual-work-model')
    await wrapper.get('.settings-form').trigger('submit')
    await flushPromises()
    const saved = wrapper.emitted('saved')![0]![0] as ReturnType<typeof bootstrap>
    expect(saved.aiChannels.activeId).toBe(newId)
    expect(saved.aiChannels.channels).toHaveLength(3)
    expect(saved.aiChannels.channels.find((item) => item.id === newId)).toEqual({
      id: newId,
      name: 'Renamed relay',
      config: {
        protocol: 'openai-responses',
        endpoint: 'relay.example.com',
        apiKey: 'work-fictional-key',
        model: 'manual-work-model',
        systemPrompt: 'Work prompt',
      },
    })
    expect(
      saved.aiChannels.channels.find((item) => item.id === fixtureChannelId)!.config.apiKey,
    ).toBe('')
    expect(initial.aiChannels.channels).toHaveLength(2)
    expect(initial.aiConfig.apiKey).toBe('original-secret')
    await wrapper.setProps({ data: saved })
    expect(wrapper.get('#ai-channel').text()).toBe('Renamed relay')
    expect(wrapper.find('.unsaved-badge').exists()).toBe(false)
    expect(desktop.translate).not.toHaveBeenCalled()
  })
  it('validates names without changing the channel and keeps IME confirmation inside the dialog', async () => {
    const wrapper = track(
      mount(SettingsView, { props: { data: bootstrap() }, attachTo: document.body }),
    )
    await chooseSelect(wrapper, 'ai-channel', 'add-channel')
    const input = nameDialog().get('input')
    await input.trigger('compositionstart')
    await input.trigger('keydown', { key: 'Escape', isComposing: true })
    await input.setValue('中文渠道')
    await nameDialog().trigger('submit')
    expect(document.querySelector('.channel-name-dialog')).not.toBeNull()
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(fixtureChannelId)
    await input.trigger('compositionend')
    for (const [name, message] of [
      ['', '请填写'],
      ['Fixture SERVICE', '已存在'],
      ['UPfanyi official AI', '官方保留'],
      ['x'.repeat(61), '60'],
      ['bad\u0000name', '控制字符'],
    ]) {
      await submitName(name!)
      expect(nameDialog().get('[role="alert"]').text()).toContain(message!)
      expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(fixtureChannelId)
      expect(wrapper.find('.unsaved-badge').exists()).toBe(false)
    }
    await cancelDialog()
    expect(document.activeElement).toBe(wrapper.get('#ai-channel').element)
    await button(wrapper, '重命名渠道').trigger('click')
    await submitName('  Fixture service  ')
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(fixtureChannelId)
    expect(wrapper.find('.unsaved-badge').exists()).toBe(false)
  })
  it('cancels deletion unchanged, then deletes a pending active channel atomically and restores everything on cancel', async () => {
    const data = bootstrap()
    const pendingModels = deferred<string[]>()
    const pendingTest = deferred<Translation>()
    vi.mocked(desktop.listAiModels).mockReturnValue(pendingModels.promise)
    vi.mocked(desktop.testAiConnection).mockReturnValue(pendingTest.promise)
    const wrapper = track(mount(SettingsView, { props: { data } }))
    await button(wrapper, '获取模型').trigger('click')
    await button(wrapper, '测试连接').trigger('click')
    await button(wrapper, '删除渠道').trigger('click')
    await cancelDialog()
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(fixtureChannelId)
    expect(button(wrapper, '获取中…').attributes('disabled')).toBeDefined()
    expect(button(wrapper, '测试中…').attributes('disabled')).toBeDefined()
    await button(wrapper, '删除渠道').trigger('click')
    await new DOMWrapper(document.querySelector<HTMLElement>('[role="alertdialog"]')!)
      .findAll('button')
      .find((item) => item.text() === '删除')!
      .trigger('click')
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe('upfanyi')
    expect(wrapper.find('.channel-actions').exists()).toBe(false)
    expect(wrapper.find('#ai-endpoint').exists()).toBe(false)
    pendingModels.resolve(['deleted-channel-model'])
    pendingTest.reject(new Error('deleted-channel-test'))
    await flushPromises()
    expect(wrapper.text()).not.toContain('deleted-channel')
    expect(await modelOptions(wrapper)).toEqual([])
    expect(button(wrapper, '获取模型').attributes('disabled')).toBeUndefined()
    expect(button(wrapper, '测试连接').attributes('disabled')).toBeUndefined()
    await wrapper.get('#ai-channel').trigger('click')
    expect(
      document.querySelector(`#ai-channel-listbox [data-value="${fixtureChannelId}"]`),
    ).toBeNull()
    await wrapper.get('#ai-channel').trigger('keydown', { key: 'Escape' })
    await button(wrapper, '取消').trigger('click')
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(fixtureChannelId)
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe('original-secret')
    expect(desktop.saveSettings).not.toHaveBeenCalled()
    expect(data.aiChannels.channels).toHaveLength(2)
  })
  it('restores added, renamed, deleted channels and all edited fields when cancelling the whole page', async () => {
    const initial = bootstrap()
    const wrapper = track(mount(SettingsView, { props: { data: initial } }))
    await button(wrapper, '重命名渠道').trigger('click')
    await submitName('Draft name')
    await wrapper.get('#ai-api-key').setValue('draft-key')
    await chooseSelect(wrapper, 'ai-channel', 'add-channel')
    await submitName('New draft')
    const addedId = wrapper.get('#ai-channel').attributes('data-value')!
    await chooseSelect(wrapper, 'ai-channel', fixtureChannelId)
    await button(wrapper, '删除渠道').trigger('click')
    await new DOMWrapper(document.querySelector<HTMLElement>('[role="alertdialog"]')!)
      .findAll('button')
      .find((item) => item.text() === '删除')!
      .trigger('click')
    await wrapper.get('#ai-api-key').setValue('official-draft-key')
    await button(wrapper, '取消').trigger('click')
    expect(wrapper.get('#ai-channel').attributes('data-value')).toBe(fixtureChannelId)
    expect(wrapper.get('#ai-channel').text()).toBe('Fixture service')
    expect((wrapper.get('#ai-api-key').element as HTMLInputElement).value).toBe('original-secret')
    await wrapper.get('#ai-channel').trigger('click')
    expect(document.querySelector(`#ai-channel-listbox [data-value="${addedId}"]`)).toBeNull()
    expect(
      document.querySelector(`#ai-channel-listbox [data-value="${fixtureChannelId}"]`)?.textContent,
    ).toBe('Fixture service')
    expect(wrapper.find('.unsaved-badge').exists()).toBe(false)
    expect(desktop.saveSettings).not.toHaveBeenCalled()
  })
})
