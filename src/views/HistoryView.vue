<script setup lang="ts">
import { ref, watch } from 'vue'
import AppIcon from '../components/AppIcon.vue'
import Notice from '../components/Notice.vue'
import RecordPagination from '../components/RecordPagination.vue'
import ConfirmAction from '../components/ConfirmAction.vue'
import { useRecords } from '../composables/useRecords'
import { useI18n } from '../i18n'
import { desktop, errorMessage } from '../services/desktop'
import type { HistoryRecord, ReuseText } from '../types/desktop'
const emit = defineEmits<{ reuse: [value: ReuseText] }>()
const props = withDefaults(defineProps<{ active?: boolean }>(), { active: true })
const { t, failure } = useI18n()
const { keyword, items, total, loading, error, pageNumber, pages, load, previous, next, cancel } =
  useRecords(desktop.listHistory)
const selected = ref<HistoryRecord | null>(null)
const confirm = ref<'delete' | 'clear' | null>(null)
const busy = ref(false)
const actionError = ref('')
const message = ref('')
const searchInput = ref<HTMLInputElement | null>(null)
watch(items, (value) => {
  if (selected.value) selected.value = value.find((item) => item.id === selected.value?.id) ?? null
})
watch(selected, () => {
  confirm.value = null
  message.value = ''
})
watch(
  () => props.active,
  (value) => {
    if (value) void load()
    else {
      cancel()
      confirm.value = null
    }
  },
  { immediate: true },
)
async function remove() {
  const id = selected.value?.id
  if (confirm.value === 'delete' && id === undefined) return
  busy.value = true
  actionError.value = ''
  message.value = ''
  const clearAll = confirm.value === 'clear'
  try {
    if (clearAll) await desktop.clearHistory()
    else await desktop.deleteHistory(id!)
    if (clearAll || selected.value?.id === id) selected.value = null
    confirm.value = null
    await load()
  } catch (reason) {
    actionError.value = errorMessage(reason)
  } finally {
    busy.value = false
  }
}
async function copy(value: string) {
  actionError.value = ''
  try {
    await desktop.copyText(value)
    message.value = t('copied')
  } catch (reason) {
    actionError.value = errorMessage(reason)
  }
}
</script>
<template>
  <section class="page records-page">
    <header class="page-heading">
      <h1>{{ t('history') }}</h1>
      <button class="button subtle small" :disabled="busy" @click="confirm = 'clear'">
        <AppIcon name="trash" :size="16" />{{ t('clearHistory') }}
      </button>
    </header>
    <Notice v-if="error" kind="error"
      >{{ t('listError') }} · {{ failure(error) }}
      <button class="inline-button" @click="load">{{ t('retry') }}</button></Notice
    ><Notice v-if="actionError" kind="error"
      >{{ t('operationError') }} · {{ failure(actionError) }}</Notice
    ><Notice v-if="message" kind="success">{{ message }}</Notice>
    <ConfirmAction
      v-if="confirm"
      :message="confirm === 'clear' ? t('confirmClear') : t('confirmDelete')"
      :busy="busy"
      :fallback-focus="searchInput"
      @cancel="confirm = null"
      @confirm="remove"
    />
    <div class="records-layout">
      <section class="record-list card">
        <label class="search-box"
          ><AppIcon name="search" :size="18" /><input
            ref="searchInput"
            v-model="keyword"
            :placeholder="t('searchHistory')"
            :aria-label="t('searchHistory')"
        /></label>
        <div class="record-list-content" :aria-busy="loading">
          <div v-if="loading" class="empty-state">
            <span class="spinner"></span>{{ t('loading') }}
          </div>
          <div v-else-if="error" class="empty-state">{{ t('listError') }}</div>
          <div v-else-if="!items.length" class="empty-state">
            <AppIcon name="history" :size="32" /><span>{{
              keyword ? t('emptySearch') : t('emptyHistory')
            }}</span>
          </div>
          <button
            v-for="item in items"
            v-else
            :key="item.id"
            :class="['record-item', { selected: selected?.id === item.id }]"
            :aria-pressed="selected?.id === item.id"
            @click="selected = item"
          >
            <strong>{{ item.sourceText }}</strong
            ><span>{{ item.translatedText }}</span
            ><small>{{ item.model }} <span>·</span> {{ item.createTime }}</small>
          </button>
        </div>
        <RecordPagination
          :total="total"
          :page="pageNumber"
          :pages="pages"
          :loading="loading"
          @previous="previous"
          @next="next"
        />
      </section>
      <section class="record-detail card">
        <div v-if="!selected" class="empty-state">
          <AppIcon name="history" :size="36" /><span>{{ t('selectRecord') }}</span>
        </div>
        <template v-else
          ><header class="detail-heading">
            <span class="model-badge" :title="selected.model">{{ selected.model }}</span
            ><small>{{ selected.createTime }}</small>
          </header>
          <div class="detail-text">
            <label>{{ t('source') }} · {{ t(selected.sourceLang) }}</label>
            <p>{{ selected.sourceText }}</p>
            <button class="button subtle small" @click="copy(selected.sourceText)">
              <AppIcon name="copy" :size="15" />{{ t('copy') }}
            </button>
          </div>
          <div class="detail-text translated">
            <label>{{ t('result') }} · {{ t(selected.targetLang) }}</label>
            <p>{{ selected.translatedText }}</p>
            <button class="button subtle small" @click="copy(selected.translatedText)">
              <AppIcon name="copy" :size="15" />{{ t('copy') }}
            </button>
          </div>
          <footer class="detail-actions">
            <button
              class="button primary small"
              @click="
                emit('reuse', {
                  text: selected.sourceText,
                  sourceLang: selected.sourceLang,
                  targetLang: selected.targetLang,
                })
              "
            >
              <AppIcon name="arrow" :size="16" />{{ t('reuse') }}</button
            ><button
              class="icon-button danger-text"
              :disabled="busy"
              :aria-label="t('deleteRecord')"
              @click="confirm = 'delete'"
            >
              <AppIcon name="trash" :size="18" />
            </button></footer
        ></template>
      </section>
    </div>
  </section>
</template>
