import { onScopeDispose, ref, watch } from 'vue'
import { desktop, errorMessage } from '../services/desktop'
import type { Settings, Translation } from '../types/desktop'

export function useTranslation(settings: Settings) {
  const text = ref('')
  const sourceLang = ref(settings.defaultSourceLang)
  const targetLang = ref(settings.defaultTargetLang)
  const auto = ref(settings.autoTranslate)
  const result = ref<Translation | null>(null)
  const pending = ref(false)
  const error = ref('')
  let generation = 0
  let timer: ReturnType<typeof setTimeout> | undefined
  let suppressSchedule = false

  function cancelScheduled() {
    clearTimeout(timer)
    timer = undefined
  }
  // 序号覆盖成功、失败和 finally，过时请求不能解除新请求的加载状态。
  async function run() {
    cancelScheduled()
    const current = ++generation
    if (!text.value.trim()) {
      pending.value = false
      result.value = null
      error.value = ''
      return
    }
    const request = {
      text: text.value,
      sourceLang: sourceLang.value,
      targetLang: targetLang.value,
    }
    pending.value = true
    result.value = null
    error.value = ''
    try {
      const translated = await desktop.translate(request)
      if (current === generation) result.value = translated
    } catch (reason) {
      if (current === generation) error.value = errorMessage(reason)
    } finally {
      if (current === generation) pending.value = false
    }
  }

  watch(
    [text, sourceLang, targetLang],
    () => {
      cancelScheduled()
      ++generation
      pending.value = false
      error.value = ''
      if (!text.value.trim()) result.value = null
      if (!suppressSchedule && auto.value && text.value.trim())
        timer = setTimeout(() => {
          void run()
        }, 450)
    },
    { flush: 'sync' },
  )
  watch(
    auto,
    (value) => {
      cancelScheduled()
      if (value && !suppressSchedule && text.value.trim())
        timer = setTimeout(() => {
          void run()
        }, 450)
    },
    { flush: 'sync' },
  )

  function setTextAndTranslate(value: string, from?: string, to?: string) {
    suppressSchedule = true
    text.value = value
    if (from) sourceLang.value = from
    if (to && to !== 'auto') targetLang.value = to
    suppressSchedule = false
    return run()
  }
  function applyDefaults(value: Settings) {
    suppressSchedule = true
    sourceLang.value = value.defaultSourceLang
    targetLang.value = value.defaultTargetLang
    auto.value = value.autoTranslate
    suppressSchedule = false
  }
  onScopeDispose(() => {
    cancelScheduled()
    ++generation
  })
  return {
    text,
    sourceLang,
    targetLang,
    auto,
    result,
    pending,
    error,
    run,
    setTextAndTranslate,
    applyDefaults,
  }
}
