import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  Bootstrap,
  AiConfig,
  AiChannels,
  FavoriteWord,
  HistoryRecord,
  Page,
  Settings,
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
  translate: (request: TranslationRequest) => call<Translation>('translate', { request }),
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
  async onClipboard(handler: (text: string) => void): Promise<UnlistenFn> {
    if (!isTauri()) return () => {}
    return listen<string>('desktop-clipboard', (event) => handler(event.payload))
  },
  async onNavigate(handler: (page: string) => void): Promise<UnlistenFn> {
    if (!isTauri()) return () => {}
    return listen<string>('desktop-navigate', (event) => handler(event.payload))
  },
}

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  if (typeof error === 'string') return error
  return String(error)
}
