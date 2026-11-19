import { computed, onScopeDispose, ref, shallowRef, watch } from 'vue'
import { errorMessage } from '../services/desktop'
import type { Page } from '../types/desktop'

export function useRecords<T extends { id: number }>(
  fetch: (keyword: string, limit: number, offset: number) => Promise<Page<T>>,
) {
  const keyword = ref('')
  const offset = ref(0)
  const items = shallowRef<T[]>([])
  const total = ref(0)
  const loading = ref(false)
  const error = ref('')
  const limit = 20
  let generation = 0
  let timer: ReturnType<typeof setTimeout> | undefined
  const pages = computed(() => Math.max(1, Math.ceil(total.value / limit)))
  const pageNumber = computed(() => Math.min(pages.value, Math.floor(offset.value / limit) + 1))

  async function load() {
    clearTimeout(timer)
    const current = ++generation
    loading.value = true
    error.value = ''
    try {
      const page = await fetch(keyword.value.trim(), limit, offset.value)
      if (current !== generation) return
      if (offset.value > 0 && offset.value >= page.total) {
        offset.value = Math.max(0, Math.floor((page.total - 1) / limit) * limit)
        return
      }
      items.value = page.items
      total.value = page.total
    } catch (reason) {
      if (current === generation) {
        error.value = errorMessage(reason)
        items.value = []
        total.value = 0
      }
    } finally {
      if (current === generation) loading.value = false
    }
  }
  watch(
    keyword,
    () => {
      clearTimeout(timer)
      ++generation
      loading.value = true
      timer = setTimeout(() => {
        if (offset.value === 0) void load()
        else offset.value = 0
      }, 250)
    },
    { flush: 'sync' },
  )
  watch(offset, () => {
    void load()
  })
  function cancel() {
    clearTimeout(timer)
    ++generation
    loading.value = false
  }
  onScopeDispose(cancel)
  function previous() {
    offset.value = Math.max(0, offset.value - limit)
  }
  function next() {
    if (offset.value + limit < total.value) offset.value += limit
  }
  return {
    keyword,
    offset,
    items,
    total,
    loading,
    error,
    pageNumber,
    pages,
    load,
    previous,
    next,
    cancel,
  }
}
