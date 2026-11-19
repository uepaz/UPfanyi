import { Channel, invoke, isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  Bootstrap,
  AiConfig,
  AiChannels,
  FavoriteWord,
  HistoryRecord,
  Page,
  Settings,
  PopupSnapshot,
  TargetLanguage,
  Translation,
  TranslationRequest,
} from '../types/desktop'

// 浏览器预览只展示页面；桌面权限、网络与数据操作统一经过 Rust。
function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) return Promise.reject(new Error('DESKTOP_REQUIRED'))
  return invoke<T>(command, args)
}
export const desktop = {
  bootstrap: () => call<Bootstrap>('bootstrap'),
  async translate(request: TranslationRequest, onDelta: (text: string) => void) {
    if (!isTauri()) throw new Error('DESKTOP_REQUIRED')
    return invoke<Translation>('translate', { request, onDelta: new Channel<string>(onDelta) })
  },
  testAiConnection: (config: AiConfig) => call<Translation>('test_ai_connection', { config }),
  listAiModels: (config: AiConfig) => call<string[]>('list_ai_models', { config }),
  openOfficialWebsite: () => call<void>('open_official_website'),
  saveSettings: (settings: Settings, aiChannels: AiChannels) =>
    call<Bootstrap>('save_settings', { settings, aiChannels }),
  listHistory: (keyword: string, limit: number, offset: number) =>
    call<Page<HistoryRecord>>('list_history', { keyword, limit, offset }),
  deleteHistory: (id: number) => call<void>('delete_history', { id }),
  clearHistory: () => call<void>('clear_history'),
  listFavorites: (keyword: string, limit: number, offset: number) =>
    call<Page<FavoriteWord>>('list_favorites', { keyword, limit, offset }),
  saveFavorite: (word: string, translation: string, note: string | null) =>
    call<FavoriteWord>('save_favorite', { word, translation, note }),
  deleteFavorite: (id: number) => call<void>('delete_favorite', { id }),
  exportFavorites: () => call<string | null>('export_favorites'),
  copyText: (text: string) => call<void>('copy_text', { text }),
  async onNavigate(handler: (page: string) => void): Promise<UnlistenFn> {
    if (!isTauri()) return () => {}
    return listen<string>('desktop-navigate', (event) => handler(event.payload))
  },
}

export const popup = {
  startDragging: () => call<void>('popup_start_drag'),
  snapshot: () => call<PopupSnapshot>('popup_snapshot'),
  ready: (requestId: number) => call<void>('popup_ready', { requestId }),
  retranslate: (requestId: number, request: TranslationRequest) =>
    call<PopupSnapshot>('popup_retranslate', { requestId, request }),
  pin: (requestId: number, pinned: boolean) => call<void>('popup_pin', { requestId, pinned }),
  copy: (requestId: number) => call<void>('popup_copy', { requestId }),
  dismiss: (requestId: number) => call<void>('popup_dismiss', { requestId }),
  async onUpdate(handler: (version: number) => void): Promise<UnlistenFn> {
    if (!isTauri()) return () => {}
    return listen<number>('selection-popup-updated', (event) => handler(event.payload), {
      target: { kind: 'WebviewWindow', label: 'selection-popup' },
    })
  },
}

export const targetLanguage = {
  get: () => call<TargetLanguage>('get_target_language'),
  set: (language: string) => call<TargetLanguage>('set_target_language', { language }),
  async onChange(handler: (value: TargetLanguage) => void): Promise<UnlistenFn> {
    if (!isTauri()) return () => {}
    return listen<TargetLanguage>('target-language-changed', (event) => handler(event.payload))
  },
}

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === 'string') return error
  return String(error)
}
