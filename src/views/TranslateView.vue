<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import AppIcon from '../components/AppIcon.vue'
import AppSelect from '../components/AppSelect.vue'
import Notice from '../components/Notice.vue'
import { useTranslation } from '../composables/useTranslation'
import { useTargetLanguage } from '../composables/useTargetLanguage'
import { languageCodes, useI18n } from '../i18n'
import { desktop, errorMessage } from '../services/desktop'
import { aiConfigStatus } from '../services/aiConfig'
import type { Bootstrap } from '../types/desktop'

const props = defineProps<{ data: Bootstrap }>()
const emit = defineEmits<{ configure: [] }>()
const { t, failure } = useI18n()
const sourceOptions = computed(() => languageCodes.map((value) => ({ value, label: t(value) })))
const targetOptions = computed(() => sourceOptions.value.filter(({ value }) => value !== 'auto'))
const {
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
} = useTranslation(props.data.settings)
const sharedTarget = useTargetLanguage(() => props.data.settings.defaultTargetLang)
const { error: targetError } = sharedTarget
watch(sharedTarget.language, syncTarget, { flush: 'sync' })
const actionError = ref('')
const message = ref('')
const savingFavorite = ref(false)
const needsSetup = computed(() => !aiConfigStatus(props.data.aiConfig).configured)
watch(
  () => props.data.settings,
  (value) => applyDefaults(value),
)
watch(result, () => {
  actionError.value = ''
  message.value = ''
})
function swap() {
  if (sourceLang.value === 'auto') {
    actionError.value = t('cannotSwapAuto')
    return
  }
  const source = sourceLang.value
  sourceLang.value = targetLang.value
  chooseTarget(source)
}
function chooseTarget(value: string) {
  if (value === targetLang.value) return
  targetLang.value = value
  void sharedTarget.choose(value)
}
function keydown(event: KeyboardEvent) {
  if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
    event.preventDefault()
    void run()
  }
}
async function copy(value: string) {
  actionError.value = ''
  message.value = ''
  try {
    await desktop.copyText(value)
    message.value = t('copied')
  } catch (reason) {
    actionError.value = errorMessage(reason)
  }
}
async function favorite() {
  if (!result.value) return
  // 始终使用已展示结果的原译文快照，输入框后续修改不改变收藏配对。
  const snapshot = result.value
  savingFavorite.value = true
  actionError.value = ''
  message.value = ''
  try {
    await desktop.saveFavorite(snapshot.sourceText, snapshot.translatedText, null)
    message.value = t('favorited')
  } catch (reason) {
    actionError.value = errorMessage(reason)
  } finally {
    savingFavorite.value = false
  }
}
defineExpose({ setTextAndTranslate })
</script>
<template>
  <section class="page translate-page">
    <header class="page-heading">
      <h1>{{ t('translate') }}</h1>
      <label class="toggle-label"
        ><input v-model="auto" type="checkbox" role="switch" /><span>{{
          t('autoTranslate')
        }}</span></label
      >
    </header>
    <div class="language-toolbar card">
      <label for="translate-source-lang"
        ><span class="sr-only">{{ t('sourceLang') }}</span
        ><AppSelect
          id="translate-source-lang"
          v-model="sourceLang"
          :label="t('sourceLang')"
          :options="sourceOptions"
      /></label>
      <button
        class="icon-button swap-button"
        :aria-label="t('swap')"
        :title="t('swap')"
        @click="swap"
      >
        <AppIcon name="swap" />
      </button>
      <label for="translate-target-lang"
        ><span class="sr-only">{{ t('targetLang') }}</span
        ><AppSelect
          id="translate-target-lang"
          :model-value="targetLang"
          :label="t('targetLang')"
          :options="targetOptions"
          @update:model-value="chooseTarget"
      /></label>
    </div>
    <div v-if="needsSetup" class="setup-hint card" role="status">
      <span>{{ t('firstUseHint') }}</span
      ><button class="button primary small" @click="emit('configure')">
        {{ t('configureAi') }}
      </button>
    </div>
    <Notice v-if="error" kind="error">{{ t('requestError') }} · {{ failure(error) }}</Notice>
    <Notice v-if="actionError" kind="error">{{ failure(actionError) }}</Notice>
    <Notice v-if="targetError" kind="error">{{ t(targetError) }}</Notice>
    <Notice v-if="message" kind="success">{{ message }}</Notice>
    <div class="translation-panels">
      <section class="text-panel card">
        <header>
          <span>{{ t('source') }}</span
          ><button
            class="icon-button"
            :disabled="!text"
            :aria-label="t('clear')"
            @click="text = ''"
          >
            <AppIcon name="close" :size="17" />
          </button>
        </header>
        <textarea
          v-model="text"
          :placeholder="t('inputPlaceholder')"
          :aria-label="t('source')"
          spellcheck="false"
          @keydown="keydown"
        ></textarea>
        <footer>
          <span>{{ text.length }} {{ t('characters') }}</span
          ><button
            class="icon-button"
            :disabled="!text"
            :aria-label="t('copy') + ' ' + t('source')"
            @click="copy(text)"
          >
            <AppIcon name="copy" :size="18" />
          </button>
        </footer>
      </section>
      <section class="text-panel card result-panel" :aria-busy="pending">
        <header>
          <span>{{ t('result') }}</span
          ><span v-if="result" class="model-badge" :title="result.model">{{ result.model }}</span>
        </header>
        <div class="result-content" aria-live="polite">
          <div v-if="pending && !partial" class="pending-line">
            <span class="spinner"></span>{{ t('translating') }}
          </div>
          <p v-if="pending && partial">{{ partial }}</p>
          <p v-else-if="result">{{ result.translatedText }}</p>
          <div v-else-if="!pending" class="result-empty">
            <AppIcon name="translate" :size="36" /><span>{{ t('resultPlaceholder') }}</span>
          </div>
        </div>
        <div v-if="result && result.sourceText !== text" class="paired-source">
          <span>{{ t('source') }}</span
          >{{ result.sourceText }}
        </div>
        <footer>
          <span v-if="result">{{ t(result.sourceLang) }} → {{ t(result.targetLang) }}</span
          ><span v-else></span>
          <div>
            <button
              class="icon-button"
              :disabled="!result || savingFavorite"
              :aria-label="t('favorite')"
              :title="t('favorite')"
              @click="favorite"
            >
              <AppIcon name="favorites" :size="18" /></button
            ><button
              class="icon-button"
              :disabled="!result"
              :aria-label="t('copy') + ' ' + t('result')"
              @click="result && copy(result.translatedText)"
            >
              <AppIcon name="copy" :size="18" />
            </button>
          </div>
        </footer>
      </section>
    </div>
    <Notice v-if="result?.warning" kind="warning">{{ result.warning }}</Notice>
    <div class="translate-footer">
      <span>{{ t('shortcutHint') }}</span
      ><button class="button primary" :disabled="!text.trim()" @click="run">
        <span v-if="pending" class="spinner"></span><AppIcon v-else name="arrow" :size="18" />{{
          pending ? t('translating') : t('translateNow')
        }}
      </button>
    </div>
  </section>
</template>
