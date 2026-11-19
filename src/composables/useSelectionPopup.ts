import { computed, nextTick, onMounted, onScopeDispose, ref, shallowRef } from 'vue'
import { popup, errorMessage } from '../services/desktop'
import { useTargetLanguage } from './useTargetLanguage'
import { setLocale, useI18n } from '../i18n'
import type { PopupSnapshot, TranslationRequest } from '../types/desktop'

export function useSelectionPopup() {
  const { t } = useI18n()
  const state = shallowRef<PopupSnapshot>({
    version: -1,
    dismissRequestId: null,
    preparing: null,
    pinned: false,
    selection: null,
  })
  const selection = computed(() => state.value.selection ?? state.value.preparing)
  const bodies = computed(() =>
    [state.value.selection, state.value.preparing].filter((item) => item !== null),
  )
  const pendingRequest = shallowRef<TranslationRequest | null>(null)
  const sharedTarget = useTargetLanguage(() => selection.value?.targetLang ?? 'en')
  const pair = computed(() => {
    const current = pendingRequest.value ?? selection.value
    return current && { sourceLang: current.sourceLang, targetLang: sharedTarget.language.value }
  })
  const active = computed(() => {
    const current = state.value.selection
    return current?.requestId === state.value.dismissRequestId ? current : null
  })
  // 草稿属于整次选区。同一选区的翻译增量、受理回执或置顶快照都不能覆盖正在输入的文字。
  const editedSource = ref<string | null>(null)
  const sourceText = computed({
    get: () => editedSource.value ?? selection.value?.sourceText ?? '',
    set: (text: string) => {
      editedSource.value = text
    },
  })
  const canChangeLanguages = computed(() => !!active.value && !!sourceText.value.trim())
  const canRetranslate = computed(() => {
    const current = active.value
    return (
      !!current &&
      canChangeLanguages.value &&
      !pendingRequest.value &&
      (current.status === 'success' ||
        current.status === 'error' ||
        current.targetLang !== sharedTarget.language.value ||
        sourceText.value !== current.sourceText)
    )
  })
  const canCopy = computed(() => active.value?.status === 'success' && !pendingRequest.value)
  const expanded = ref(false)
  const copied = ref(false)
  const notice = ref('')
  const activeActions = new Set<string>()
  let disposed = false
  let unsubscribe: (() => void) | undefined
  let copyTimer: ReturnType<typeof setTimeout> | undefined
  let reconnect: ReturnType<typeof setTimeout> | undefined
  let queue = Promise.resolve()
  let editGeneration = 0
  let read = 0
  let acknowledged = 0

  function isCurrent(id: number) {
    return !disposed && active.value?.requestId === id
  }

  async function accept(value: PopupSnapshot) {
    if (disposed || value.version <= state.value.version) return
    const current = value.selection ?? value.preparing
    if (current?.selectionId !== selection.value?.selectionId) {
      editGeneration++
      editedSource.value = null
      pendingRequest.value = null
      expanded.value = false
      queue = Promise.resolve()
    }
    if (current?.requestId !== selection.value?.requestId) {
      clearTimeout(copyTimer)
      copied.value = false
      notice.value = ''
    }
    state.value = value
    if (!current) return
    setLocale(current.locale)
    document.documentElement.lang = current.locale.startsWith('en') ? 'en' : 'zh-CN'
    document.title = t('popupTitle')
    const pending = value.preparing?.requestId
    if (pending !== undefined && acknowledged !== pending) {
      await nextTick()
      if (!disposed && state.value.preparing?.requestId === pending && acknowledged !== pending) {
        acknowledged = pending
        try {
          await popup.ready(pending)
        } catch {
          if (state.value.preparing?.requestId === pending) {
            acknowledged = 0
            notice.value = 'popupActionFailed'
          }
        }
      }
    }
  }

  async function refresh() {
    const ticket = ++read
    const version = state.value.version
    try {
      await accept(await popup.snapshot())
    } catch (reason) {
      if (!disposed && ticket === read && state.value.version === version)
        notice.value =
          errorMessage(reason) === 'DESKTOP_REQUIRED' ? 'desktopRequired' : 'popupActionFailed'
    }
  }

  async function subscribe() {
    try {
      const cleanup = await popup.onUpdate((version) => {
        if (!disposed && version > state.value.version) void refresh()
      })
      if (disposed) {
        cleanup()
        return
      }
      unsubscribe = cleanup
      await refresh()
    } catch {
      if (!disposed) {
        notice.value = 'popupActionFailed'
        reconnect = setTimeout(() => void subscribe(), 500)
      }
    }
  }
  onMounted(subscribe)

  // 点击时截取原文和语言。连续操作只排队等待宿主受理，无需等待模型生成完毕。
  function submit(sourceLang: string, targetLang: string, saved?: Promise<boolean>) {
    const current = active.value
    if (!current || !canChangeLanguages.value || targetLang === 'auto') return
    const id = current.selectionId
    const generation = editGeneration
    const request = { text: sourceText.value, sourceLang, targetLang }
    pendingRequest.value = request
    notice.value = ''
    copied.value = false
    queue = queue.then(async () => {
      try {
        if (saved && !(await saved)) return
        const latest = active.value
        if (disposed || generation !== editGeneration || latest?.selectionId !== id) return
        await accept(await popup.retranslate(latest.requestId, request))
      } catch {
        if (!disposed && pendingRequest.value === request) notice.value = 'popupActionFailed'
      } finally {
        if (pendingRequest.value === request) pendingRequest.value = null
      }
    })
  }

  function languages(sourceLang: string, targetLang: string) {
    const previous = pair.value
    if (!active.value || !previous) return
    const changedTarget = targetLang !== previous.targetLang
    if (changedTarget || sourceLang !== previous.sourceLang) {
      const saved = changedTarget ? sharedTarget.choose(targetLang) : undefined
      submit(sourceLang, targetLang, saved)
    }
  }

  function retranslate() {
    const current = active.value
    if (current && canRetranslate.value) submit(current.sourceLang, sharedTarget.language.value)
  }

  async function action(kind: 'copy' | 'pin' | 'dismiss' | 'drag') {
    const selectedId = selection.value?.selectionId
    if (pendingRequest.value && kind === 'pin') await queue
    const current = selection.value
    if (disposed || !current || current.selectionId !== selectedId) return
    const id =
      kind === 'dismiss' ? (state.value.dismissRequestId ?? current.requestId) : current.requestId
    const key = kind + id
    if (activeActions.has(key)) return
    activeActions.add(key)
    notice.value = ''
    try {
      if (kind === 'drag') {
        await popup.startDragging()
      } else if (kind === 'pin') {
        await popup.pin(id, !state.value.pinned)
        await refresh()
      } else if (kind === 'copy') {
        if (!canCopy.value) return
        await popup.copy(id)
        if (isCurrent(id) && canCopy.value) {
          copied.value = true
          clearTimeout(copyTimer)
          copyTimer = setTimeout(() => {
            copied.value = false
          }, 1300)
        }
      } else {
        // 关闭立即送达。若在途改向刚被受理，只重核同一选区的最新尝试。
        const changing = !!pendingRequest.value
        editGeneration++
        pendingRequest.value = null
        await popup.dismiss(id)
        if (changing && !disposed) {
          const latest = await popup.snapshot()
          if (
            latest.selection &&
            latest.dismissRequestId === latest.selection.requestId &&
            latest.selection.selectionId === selectedId &&
            latest.selection.requestId !== id
          )
            await popup.dismiss(latest.selection.requestId)
        }
      }
    } catch {
      if (isCurrent(id)) notice.value = kind === 'copy' ? 'popupCopyFailed' : 'popupActionFailed'
    } finally {
      activeActions.delete(key)
    }
  }

  function escape(event: KeyboardEvent) {
    if (event.key === 'Escape' && !event.defaultPrevented && !event.repeat && !event.isComposing) {
      event.preventDefault()
      void action('dismiss')
    }
  }
  onMounted(() => window.addEventListener('keydown', escape))
  onScopeDispose(() => {
    disposed = true
    unsubscribe?.()
    clearTimeout(reconnect)
    clearTimeout(copyTimer)
    window.removeEventListener('keydown', escape)
  })

  return {
    state,
    selection,
    bodies,
    pair,
    pendingRequest,
    sourceText,
    canChangeLanguages,
    canChooseTarget: computed(() => !!active.value),
    targetError: sharedTarget.error,
    canRetranslate,
    canCopy,
    copied,
    expanded,
    notice,
    action,
    languages,
    retranslate,
  }
}
