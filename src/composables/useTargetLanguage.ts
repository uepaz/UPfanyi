import { computed, onMounted, onScopeDispose, ref, shallowRef } from 'vue'
import { targetLanguage } from '../services/desktop'
import type { TargetLanguage } from '../types/desktop'

export function useTargetLanguage(initial: () => string) {
  const saved = shallowRef<TargetLanguage>()
  const pending = ref<string>()
  const language = computed(() => pending.value ?? saved.value?.language ?? initial())
  const error = ref('')
  let disposed = false
  let choice = 0
  let writes = Promise.resolve()
  let unlisten: (() => void) | undefined
  let retry: ReturnType<typeof setTimeout> | undefined
  let connecting = false

  // 两个窗口的事件、首次读取及保存回执可能乱序，只接受更新的已提交版本。
  function accept(value: TargetLanguage) {
    if (!disposed && (!saved.value || value.revision > saved.value.revision)) saved.value = value
  }

  async function connect() {
    if (disposed || connecting) return
    connecting = true
    clearTimeout(retry)
    try {
      if (!unlisten) {
        const cleanup = await targetLanguage.onChange(accept)
        if (disposed) return cleanup()
        unlisten = cleanup
      }
      accept(await targetLanguage.get())
      if (!disposed && error.value === 'targetLanguageSyncFailed') error.value = ''
    } catch {
      if (!disposed) {
        error.value = 'targetLanguageSyncFailed'
        retry = setTimeout(() => void connect(), 1500)
      }
    } finally {
      connecting = false
    }
  }

  function choose(value: string): Promise<boolean> {
    if (value === language.value) return Promise.resolve(true)
    const current = ++choice
    pending.value = value
    error.value = ''
    // 同一窗口按用户点击顺序提交。旧回执不能盖住更晚的本地选择。
    const write = writes.then(async () => {
      try {
        accept(await targetLanguage.set(value))
        return true
      } catch {
        if (!disposed && current === choice) error.value = 'targetLanguageSaveFailed'
        return false
      } finally {
        if (!disposed && current === choice) pending.value = undefined
      }
    })
    writes = write.then(() => {})
    return write
  }

  onMounted(() => {
    window.addEventListener('focus', connect)
    void connect()
  })
  onScopeDispose(() => {
    disposed = true
    unlisten?.()
    clearTimeout(retry)
    window.removeEventListener('focus', connect)
  })
  return { language, error, choose }
}
