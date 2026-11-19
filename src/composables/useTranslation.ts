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
  const partial = ref('')
  const error = ref('')
  let generation = 0
  let timer: ReturnType<typeof setTimeout> | undefined
  let suppressSchedule = false
  let syncingTarget = false

  function cancelScheduled() {
    clearTimeout(timer)
    timer = undefined
  }
  // 序号覆盖成功、失败和 finally，过时请求不能解除新请求的加载状态。
  async function run() {
    return translateTo(targetLang.value)
  }
  async function translateTo(target: string) {
    cancelScheduled()
    const current = ++generation
    partial.value = ''
    if (!text.value.trim()) {
      pending.value = false
      result.value = null
      error.value = ''
      return
    }
    const request = {
      text: text.value,
      sourceLang: sourceLang.value,
      targetLang: target,
    }
    pending.value = true
    result.value = null
    error.value = ''
    try {
      const translated = await desktop.translate(request, (piece) => {
        if (current === generation && pending.value) partial.value += piece
      })
      if (current === generation) result.value = translated
    } catch (reason) {
      if (current === generation) error.value = errorMessage(reason)
    } finally {
      if (current === generation) {
        pending.value = false
        partial.value = ''
      }
    }
  }

  watch(
    [text, sourceLang, targetLang],
    () => {
      cancelScheduled()
      // 被动同步只更新下次翻译的方向，已发出的请求仍按原方向展示其结果。
      if (syncingTarget) return
      ++generation
      partial.value = ''
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
    suppressSchedule = false
    // 从历史重译保留该条记录的方向，一次性请求不修改两个选择框的偏好。
    return translateTo(to && to !== 'auto' ? to : targetLang.value)
  }
  function syncTarget(value: string) {
    syncingTarget = true
    targetLang.value = value
    syncingTarget = false
  }
  function applyDefaults(value: Settings) {
    suppressSchedule = true
    sourceLang.value = value.defaultSourceLang
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
    partial,
    pending,
    error,
    run,
    setTextAndTranslate,
    applyDefaults,
    syncTarget,
  }
}
