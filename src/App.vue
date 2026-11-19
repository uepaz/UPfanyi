<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from 'vue'
import AppIcon from './components/AppIcon.vue'
import Notice from './components/Notice.vue'
import { setLocale, useI18n } from './i18n'
import { desktop, errorMessage } from './services/desktop'
import { provideSelectScope } from './composables/useSelectScope'
import type { Bootstrap, Navigation, ReuseText } from './types/desktop'
import FavoritesView from './views/FavoritesView.vue'
import HistoryView from './views/HistoryView.vue'
import SettingsView from './views/SettingsView.vue'
import TranslateView from './views/TranslateView.vue'
const { t, failure, locale } = useI18n()
const data = ref<Bootstrap | null>(null)
const page = ref<Navigation>('translate')
provideSelectScope(page)
const loading = ref(true)
const error = ref('')
const eventError = ref('')
const translationView = ref<InstanceType<typeof TranslateView> | null>(null)
const settingsView = ref<InstanceType<typeof SettingsView> | null>(null)
const pages: Navigation[] = ['translate', 'history', 'favorites', 'settings']
let disposed = false
let unlisten: (() => void) | undefined
function saved(value: Bootstrap) {
  data.value = value
  setLocale(value.settings.locale)
}
async function bootstrap() {
  loading.value = true
  error.value = ''
  try {
    saved(await desktop.bootstrap())
  } catch (reason) {
    error.value = errorMessage(reason)
  } finally {
    loading.value = false
  }
}
async function reuse(value: ReuseText) {
  page.value = 'translate'
  await nextTick()
  await translationView.value?.setTextAndTranslate(value.text, value.sourceLang, value.targetLang)
}
async function configureAi() {
  page.value = 'settings'
  await nextTick()
  settingsView.value?.openAiSettings()
}
onMounted(async () => {
  await bootstrap()
  if (disposed) return
  try {
    const cleanup = await desktop.onNavigate((value) => {
      if (value === 'settings') page.value = 'settings'
    })
    if (disposed) cleanup()
    else unlisten = cleanup
  } catch (reason) {
    if (!disposed) eventError.value = errorMessage(reason)
  }
})
onUnmounted(() => {
  disposed = true
  unlisten?.()
})
</script>
<template>
  <div class="app-shell" :lang="locale.startsWith('en') ? 'en' : 'zh-CN'">
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-symbol"><AppIcon name="translate" :size="24" /></div>
        <strong>{{ t('appName') }}</strong>
      </div>
      <nav :aria-label="t('appName')">
        <button
          v-for="item in pages"
          :key="item"
          :class="['nav-item', { active: page === item }]"
          :aria-label="t(item)"
          :title="t(item)"
          :aria-current="page === item ? 'page' : undefined"
          @click="page = item"
        >
          <AppIcon :name="item" :size="20" /><span>{{ t(item) }}</span>
        </button>
      </nav>
      <div class="sidebar-footer">UP · 2.0</div>
    </aside>
    <main class="workspace">
      <div class="workspace-content">
        <Notice v-if="eventError" kind="error">{{ failure(eventError) }}</Notice
        ><Notice v-for="warning in data?.warnings ?? []" :key="warning" kind="warning">{{
          warning
        }}</Notice>
        <div v-if="loading" class="startup-state">
          <span class="spinner"></span>
          <p>{{ t('loading') }}</p>
        </div>
        <div v-else-if="error" class="startup-state">
          <div class="startup-icon"><AppIcon name="translate" :size="32" /></div>
          <h1>{{ t('startupError') }}</h1>
          <Notice kind="error">{{ failure(error) }}</Notice>
          <p>{{ t('startupHint') }}</p>
          <button class="button primary" @click="bootstrap">{{ t('retry') }}</button>
        </div>
        <template v-else-if="data">
          <TranslateView
            v-show="page === 'translate'"
            ref="translationView"
            :data="data"
            @configure="configureAi"
          />
          <HistoryView v-show="page === 'history'" :active="page === 'history'" @reuse="reuse" />
          <FavoritesView
            v-show="page === 'favorites'"
            :active="page === 'favorites'"
            @reuse="reuse"
          />
          <SettingsView
            v-show="page === 'settings'"
            ref="settingsView"
            :data="data"
            @saved="saved"
          />
        </template>
      </div>
    </main>
  </div>
</template>
