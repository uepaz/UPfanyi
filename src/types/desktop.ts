/** 与 Rust model.rs 的 camelCase IPC 数据契约保持一致。 */
export interface Settings {
  locale: string
  minimizeToTray: boolean
  alwaysOnTop: boolean
  // data-v3 保留字段随设置快照存取，界面不再编辑。
  clipboardMonitor: boolean
  clipboardMinLength: number
  hotkeyShowWindow: string
  hotkeyClipboardTranslate: string
  defaultSourceLang: string
  defaultTargetLang: string
  autoTranslate: boolean
}
export interface AiConfig {
  protocol: 'openai' | 'openai-responses'
  endpoint: string
  apiKey: string
  model: string
  systemPrompt: string
}
export interface AiChannel {
  id: string
  name: string
  config: AiConfig
}
export interface AiChannels {
  activeId: string
  channels: AiChannel[]
}
export interface TranslationRequest {
  text: string
  sourceLang: string
  targetLang: string
}
export interface Translation {
  sourceText: string
  translatedText: string
  sourceLang: string
  targetLang: string
  model: string
  createTime: string
  warning: string | null
}
export interface HistoryRecord extends Omit<Translation, 'warning'> {
  id: number
}
export interface FavoriteWord {
  id: number
  word: string
  translation: string
  note: string | null
  createTime: string
}
export interface Page<T> {
  items: T[]
  total: number
}
export interface Bootstrap {
  settings: Settings
  aiChannels: AiChannels
  /** 同一渠道快照派生的活动配置，仅供展示，不单独保存。 */
  aiConfig: AiConfig
  dataDir: string
  warnings: string[]
}
export type Navigation = 'translate' | 'history' | 'favorites' | 'settings'
export interface ReuseText {
  text: string
  sourceLang?: string
  targetLang?: string
}
