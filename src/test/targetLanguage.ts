import { vi } from 'vitest'
import { targetLanguage } from '../services/desktop'
import type { TargetLanguage } from '../types/desktop'

/** 用同一个宿主快照和事件订阅表连接测试中的两个窗口。 */
export function mockTargetLanguage(initial: string) {
  let saved = { language: initial, revision: 0 }
  const listeners = new Set<(value: TargetLanguage) => void>()
  vi.mocked(targetLanguage.get).mockImplementation(async () => saved)
  vi.mocked(targetLanguage.set).mockImplementation(async (language) => {
    if (language !== saved.language) {
      saved = { language, revision: saved.revision + 1 }
      listeners.forEach((listen) => listen(saved))
    }
    return saved
  })
  vi.mocked(targetLanguage.onChange).mockImplementation(async (listen) => {
    listeners.add(listen)
    return () => listeners.delete(listen)
  })
}
