import { onScopeDispose, reactive, watch } from 'vue'
import { desktop, errorMessage } from '../services/desktop'
import type { AiConfig } from '../types/desktop'

export function useAiModels(config: () => AiConfig) {
  const state = reactive({ pending: false, error: '', items: [] as string[], loaded: false })
  let generation = 0
  let disposed = false

  function reset() {
    ++generation
    state.pending = false
    state.error = ''
    state.items = []
    state.loaded = false
  }

  // 只绑定服务身份；编辑模型或提示词不丢弃候选，改回原地址也不接受旧请求。
  watch([() => config().protocol, () => config().endpoint, () => config().apiKey], reset, {
    flush: 'sync',
  })
  onScopeDispose(() => {
    disposed = true
    ++generation
  })

  async function load() {
    if (disposed || state.pending) return
    const snapshot = { ...config() }
    const current = ++generation
    state.pending = true
    state.error = ''
    state.items = []
    state.loaded = false
    try {
      const items = await desktop.listAiModels(snapshot)
      if (current !== generation) return
      state.items = items
      state.loaded = true
    } catch (reason) {
      if (current === generation) state.error = errorMessage(reason)
    } finally {
      if (current === generation) state.pending = false
    }
  }

  return { state, load, reset }
}
