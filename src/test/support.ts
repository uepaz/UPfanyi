import { effectScope, nextTick, type EffectScope } from 'vue'
import type { VueWrapper } from '@vue/test-utils'
import { afterEach, beforeEach, vi } from 'vitest'
import { useTranslation } from '../composables/useTranslation'
import { setLocale } from '../i18n'
import { desktop } from '../services/desktop'
import { UPFANYI_AI_ENDPOINT } from '../services/aiConfig'
import type {
  Bootstrap,
  AiConfig,
  FavoriteWord,
  HistoryRecord,
  Settings,
  Translation,
} from '../types/desktop'

export const settings: Settings = {
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
export function bootstrap(): Bootstrap {
  const config = { ...aiConfig }
  return {
    settings: { ...settings },
    aiConfig: config,
    aiChannels: {
      activeId: fixtureChannelId,
      channels: [
        {
          id: 'upfanyi',
          name: 'UP翻译官方 AI',
          config: { ...aiConfig, endpoint: UPFANYI_AI_ENDPOINT, apiKey: '', model: '' },
        },
        { id: fixtureChannelId, name: 'Fixture service', config },
      ],
    },
    dataDir: 'C:\\upfanyi-tests\\data-v3',
    warnings: [],
  }
}
export const fixtureChannelId = '11111111-1111-4111-8111-111111111111'
export const aiConfig: AiConfig = {
  protocol: 'openai',
  endpoint: 'https://api.openai.com',
  apiKey: 'original-secret',
  model: 'fixture-model',
  systemPrompt: 'Translate from {source_lang} to {target_lang}. Only output the translation.',
}
export function translation(sourceText = 'one', translatedText = '一'): Translation {
  return {
    sourceText,
    translatedText,
    sourceLang: 'en',
    targetLang: 'zh',
    model: 'fixture-model',
    createTime: '2026-10-03T00:00:00.000Z',
    warning: null,
  }
}
export const history: HistoryRecord = { id: 1, ...translation() }
export const favorite: FavoriteWord = {
  id: 1,
  word: 'one',
  translation: '一',
  note: 'original',
  createTime: '2026-10-03T00:00:00.000Z',
}
export function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (value: unknown) => void
  const promise = new Promise<T>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}
const wrappers: VueWrapper[] = []
export const scopes: EffectScope[] = []
export function track<T extends VueWrapper>(wrapper: T): T {
  wrappers.push(wrapper)
  return wrapper
}
export function state(autoTranslate = false) {
  const scope = effectScope()
  scopes.push(scope)
  return scope.run(() => useTranslation({ ...settings, autoTranslate }))!
}
export function button(wrapper: VueWrapper, label: string) {
  const found = wrapper.findAll('button').find((value) => value.text() === label)
  if (!found) throw new Error(`Button missing: ${label}`)
  return found
}
beforeEach(() => {
  vi.resetAllMocks()
  setLocale('zh_CN')
  vi.mocked(desktop.bootstrap).mockResolvedValue(bootstrap())
  vi.mocked(desktop.translate).mockResolvedValue(translation())
  vi.mocked(desktop.onNavigate).mockResolvedValue(() => {})
  vi.mocked(desktop.listHistory).mockResolvedValue({ items: [history], total: 1 })
  vi.mocked(desktop.listFavorites).mockResolvedValue({ items: [favorite], total: 1 })
  vi.mocked(desktop.saveFavorite).mockResolvedValue(favorite)
})
afterEach(() => {
  wrappers.splice(0).forEach((wrapper) => wrapper.unmount())
  scopes.splice(0).forEach((scope) => scope.stop())
  vi.useRealTimers()
})

/** 通过实际弹层选项点击，避免测试绕过控件交互。 */
export async function chooseSelect(wrapper: VueWrapper, id: string, value: string) {
  await wrapper.get(`#${id}`).trigger('click')
  const option = document
    .getElementById(`${id}-listbox`)
    ?.querySelector<HTMLElement>(`[data-value="${value}"]`)
  if (!option) throw new Error(`Select option missing: ${id}/${value}`)
  option.click()
  await nextTick()
}

/** 用实际键盘事件录制，避免直接赋值绕过 readonly 控件。 */
export async function recordShortcut(wrapper: VueWrapper, id: string, event: KeyboardEventInit) {
  const input = wrapper.get(`#${id}`)
  await input.trigger('click')
  await input.trigger('keydown', event)
}
