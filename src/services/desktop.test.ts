// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { desktop } from './desktop'
import type { AiChannels, AiConfig, Settings, TranslationRequest } from '../types/desktop'
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }))
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }))
beforeEach(() => {
  vi.resetAllMocks()
  vi.mocked(isTauri).mockReturnValue(true)
})
describe('desktop IPC contract', () => {
  it('rejects browser actions before invoking a desktop command', async () => {
    vi.mocked(isTauri).mockReturnValue(false)
    await expect(desktop.bootstrap()).rejects.toThrow('DESKTOP_REQUIRED')
    await expect(desktop.copyText('test')).rejects.toThrow('DESKTOP_REQUIRED')
    await expect(desktop.openOfficialWebsite()).rejects.toThrow('DESKTOP_REQUIRED')
    await expect(
      desktop.listAiModels({
        protocol: 'openai',
        endpoint: '',
        apiKey: '',
        model: '',
        systemPrompt: '',
      }),
    ).rejects.toThrow('DESKTOP_REQUIRED')
    expect(invoke).not.toHaveBeenCalled()
  })
  it('passes the translation DTO under the named request argument', async () => {
    const request: TranslationRequest = { text: 'test', sourceLang: 'auto', targetLang: 'en' }
    await desktop.translate(request)
    expect(invoke).toHaveBeenCalledExactlyOnceWith('translate', { request })
  })
  it('opens only the official website command without accepting URL or configuration arguments', async () => {
    await desktop.openOfficialWebsite()
    expect(invoke).toHaveBeenCalledExactlyOnceWith('open_official_website', undefined)
  })
  it('passes one AI configuration to connection testing without saving it', async () => {
    const config: AiConfig = {
      protocol: 'openai',
      endpoint: 'http://localhost:11434',
      apiKey: '',
      model: 'fixture-model',
      systemPrompt: '',
    }
    await desktop.testAiConnection(config)
    expect(invoke).toHaveBeenCalledExactlyOnceWith('test_ai_connection', { config })
  })
  it('saves the complete channel collection and active identity with preferences', async () => {
    const settings: Settings = {
      locale: 'zh_CN',
      minimizeToTray: true,
      alwaysOnTop: false,
      clipboardMonitor: false,
      clipboardMinLength: 1,
      hotkeyShowWindow: 'Ctrl+Shift+T',
      hotkeyClipboardTranslate: 'Ctrl+Shift+C',
      defaultSourceLang: 'auto',
      defaultTargetLang: 'en',
      autoTranslate: false,
    }
    const aiChannels: AiChannels = {
      activeId: 'b3e3782e-9c85-4eb2-b902-9b837c628320',
      channels: [
        {
          id: 'upfanyi',
          name: 'UP翻译官方 AI',
          config: {
            protocol: 'openai',
            endpoint: 'https://uptoken.top',
            apiKey: '',
            model: '',
            systemPrompt: '',
          },
        },
        {
          id: 'b3e3782e-9c85-4eb2-b902-9b837c628320',
          name: '本地模型',
          config: {
            protocol: 'openai',
            endpoint: 'http://localhost:11434',
            apiKey: '',
            model: 'fixture-model',
            systemPrompt: '',
          },
        },
      ],
    }
    await desktop.saveSettings(settings, aiChannels)
    expect(invoke).toHaveBeenCalledExactlyOnceWith('save_settings', { settings, aiChannels })
  })
  it('fetches models with the unsaved Responses configuration and no selected model', async () => {
    const config: AiConfig = {
      protocol: 'openai-responses',
      endpoint: 'https://example.com/v1/responses',
      apiKey: 'fictional',
      model: '',
      systemPrompt: '',
    }
    vi.mocked(invoke).mockResolvedValue(['available-model'])
    expect(await desktop.listAiModels(config)).toEqual(['available-model'])
    expect(invoke).toHaveBeenCalledExactlyOnceWith('list_ai_models', { config })
  })
  it('uses the original favorite text, translation and nullable note arguments', async () => {
    await desktop.saveFavorite('word', '译文', null)
    expect(invoke).toHaveBeenCalledExactlyOnceWith('save_favorite', {
      word: 'word',
      translation: '译文',
      note: null,
    })
  })
  it('forwards the native export cancellation rather than fabricating a path', async () => {
    vi.mocked(invoke).mockResolvedValue(null)
    expect(await desktop.exportFavorites()).toBeNull()
  })
  it('unwraps the clipboard payload and exposes listener cleanup', async () => {
    const cleanup = vi.fn()
    vi.mocked(listen).mockResolvedValue(cleanup)
    const handler = vi.fn()
    const unlisten = await desktop.onClipboard(handler)
    const callback = vi.mocked(listen).mock.calls[0]![1]
    callback({ event: 'desktop-clipboard', id: 1, payload: 'copied text' })
    expect(handler).toHaveBeenCalledExactlyOnceWith('copied text')
    unlisten()
    expect(cleanup).toHaveBeenCalledTimes(1)
  })
})
