<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import AppIcon from '../components/AppIcon.vue'
import Notice from '../components/Notice.vue'
import RecordPagination from '../components/RecordPagination.vue'
import ConfirmAction from '../components/ConfirmAction.vue'
import { useRecords } from '../composables/useRecords'
import { useI18n } from '../i18n'
import { desktop, errorMessage } from '../services/desktop'
import type { FavoriteWord, ReuseText } from '../types/desktop'
const emit = defineEmits<{ reuse: [value: ReuseText] }>()
const props = withDefaults(defineProps<{ active?: boolean }>(), { active: true })
const { t, failure } = useI18n()
const { keyword, items, total, loading, error, pageNumber, pages, load, previous, next, cancel } =
  useRecords(desktop.listFavorites)
const selected = ref<FavoriteWord | null>(null)
// 草稿按记录保存，切换记录、搜索或导航不能静默覆盖用户已输入的备注。
const noteDrafts = ref<Record<number, string>>({})
const note = computed({
  get: () =>
    selected.value ? (noteDrafts.value[selected.value.id] ?? selected.value.note ?? '') : '',
  set: (value) => {
    if (selected.value) noteDrafts.value[selected.value.id] = value
  },
})
const noteChanged = computed(() => !!selected.value && note.value !== (selected.value.note ?? ''))
const confirmDelete = ref(false)
const busy = ref(false)
const exporting = ref(false)
const actionError = ref('')
const message = ref('')
const searchInput = ref<HTMLInputElement | null>(null)
watch(selected, () => {
  confirmDelete.value = false
  message.value = ''
})
watch(items, (value) => {
  if (selected.value) selected.value = value.find((item) => item.id === selected.value?.id) ?? null
})
watch(
  () => props.active,
  (value) => {
    if (value) void load()
    else {
      cancel()
      confirmDelete.value = false
    }
  },
  { immediate: true },
)
async function remove() {
  if (!selected.value) return
  const id = selected.value.id
  busy.value = true
  actionError.value = ''
  message.value = ''
  try {
    await desktop.deleteFavorite(id)
    delete noteDrafts.value[id]
    if (selected.value?.id === id) selected.value = null
    confirmDelete.value = false
    await load()
  } catch (reason) {
    actionError.value = errorMessage(reason)
  } finally {
    busy.value = false
  }
}
async function saveNote() {
  if (!selected.value) return
  const snapshot = selected.value
  const value = note.value
  busy.value = true
  actionError.value = ''
  message.value = ''
  try {
    const saved = await desktop.saveFavorite(snapshot.word, snapshot.translation, value)
    if (noteDrafts.value[snapshot.id] === value) delete noteDrafts.value[snapshot.id]
    if (selected.value?.id === snapshot.id) selected.value = saved
    await load()
    message.value = t('noteSaved')
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
async function exportAll() {
  exporting.value = true
  actionError.value = ''
  message.value = ''
  try {
    const path = await desktop.exportFavorites()
    if (path) message.value = `${t('exported')} ${path}`
  } catch (reason) {
    actionError.value = errorMessage(reason)
  } finally {
    exporting.value = false
  }
}
</script>
<template>
  <section class="page records-page">
    <header class="page-heading">
      <h1>{{ t('favorites') }}</h1>
      <button class="button subtle small" :disabled="exporting" @click="exportAll">
        <AppIcon name="download" :size="16" />{{ t('export') }}
      </button>
    </header>
    <Notice v-if="error" kind="error"
      >{{ t('listError') }} · {{ failure(error) }}
      <button class="inline-button" @click="load">{{ t('retry') }}</button></Notice
    ><Notice v-if="actionError" kind="error"
      >{{ t('operationError') }} · {{ failure(actionError) }}</Notice
    ><Notice v-if="message" kind="success">{{ message }}</Notice>
    <ConfirmAction
      v-if="confirmDelete"
      :message="t('confirmDelete')"
      :busy="busy"
      :fallback-focus="searchInput"
      @cancel="confirmDelete = false"
      @confirm="remove"
    />
    <div class="records-layout">
      <section class="record-list card">
        <label class="search-box"
          ><AppIcon name="search" :size="18" /><input
            ref="searchInput"
            v-model="keyword"
            :placeholder="t('searchFavorites')"
            :aria-label="t('searchFavorites')"
        /></label>
        <div class="record-list-content" :aria-busy="loading">
          <div v-if="loading" class="empty-state">
            <span class="spinner"></span>{{ t('loading') }}
          </div>
          <div v-else-if="error" class="empty-state">{{ t('listError') }}</div>
          <div v-else-if="!items.length" class="empty-state">
            <AppIcon name="favorites" :size="32" /><span>{{
              keyword ? t('emptySearch') : t('emptyFavorites')
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
            <strong>{{ item.word }}</strong
            ><span>{{ item.translation }}</span
            ><small>{{
              noteDrafts[item.id] !== undefined && noteDrafts[item.id] !== (item.note ?? '')
                ? t('unsaved')
                : item.note || item.createTime
            }}</small>
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
          <AppIcon name="favorites" :size="36" /><span>{{ t('selectRecord') }}</span>
        </div>
        <template v-else
          ><header class="detail-heading">
            <span class="record-badge">{{ t('favorites') }}</span
            ><small>{{ selected.createTime }}</small>
          </header>
          <div class="detail-text">
            <label>{{ t('source') }}</label>
            <p>{{ selected.word }}</p>
            <button class="button subtle small" @click="copy(selected.word)">
              <AppIcon name="copy" :size="15" />{{ t('copy') }}
            </button>
          </div>
          <div class="detail-text translated">
            <label>{{ t('result') }}</label>
            <p>{{ selected.translation }}</p>
            <button class="button subtle small" @click="copy(selected.translation)">
              <AppIcon name="copy" :size="15" />{{ t('copy') }}
            </button>
          </div>
          <div class="note-editor">
            <label for="favorite-note"
              >{{ t('note') }}
              <span v-if="noteChanged" class="unsaved-badge">{{ t('unsaved') }}</span></label
            ><textarea
              id="favorite-note"
              v-model="note"
              :placeholder="t('notePlaceholder')"
              :disabled="busy"
            ></textarea
            ><button class="button subtle small" :disabled="busy || !noteChanged" @click="saveNote">
              {{ busy ? t('saving') : t('saveNote') }}
            </button>
            <button
              v-if="noteChanged"
              class="button subtle small"
              :disabled="busy"
              :aria-label="t('discardNote')"
              @click="selected && delete noteDrafts[selected.id]"
            >
              {{ t('cancel') }}
            </button>
          </div>
          <footer class="detail-actions">
            <button class="button primary small" @click="emit('reuse', { text: selected.word })">
              <AppIcon name="arrow" :size="16" />{{ t('reuse') }}</button
            ><button
              class="icon-button danger-text"
              :disabled="busy"
              :aria-label="t('deleteRecord')"
              @click="confirmDelete = true"
            >
              <AppIcon name="trash" :size="18" />
            </button></footer
        ></template>
      </section>
    </div>
  </section>
</template>
