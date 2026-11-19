<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import AppIcon from '../components/AppIcon.vue'
import AppSelect from '../components/AppSelect.vue'
import { useSelectionPopup } from '../composables/useSelectionPopup'
import { languageCodes, messageForLocale, useI18n } from '../i18n'

const { t } = useI18n()
const {
  state,
  selection,
  bodies,
  pair,
  pendingRequest,
  sourceText,
  canChangeLanguages,
  canChooseTarget,
  targetError,
  canRetranslate,
  canCopy,
  expanded,
  copied,
  notice,
  action,
  languages,
  retranslate,
} = useSelectionPopup()
const sourceOptions = computed(() => languageCodes.map((value) => ({ value, label: t(value) })))
const targetOptions = computed(() =>
  sourceOptions.value.filter((option) => option.value !== 'auto'),
)
const panel = ref<HTMLElement>()
async function toggleOriginal() {
  expanded.value = !expanded.value
  if (expanded.value) {
    await nextTick()
    panel.value?.querySelector<HTMLTextAreaElement>('#selection-original')?.focus()
  }
}
watch(
  () => selection.value?.requestId,
  () => {
    const body = panel.value?.querySelector<HTMLElement>('.popup-body:not([inert])')
    if (body && document.activeElement?.id !== 'selection-original') body.scrollTop = 0
  },
  { flush: 'post' },
)
const failureKeys = {
  settings: 'popupSettings',
  endpoint: 'popupEndpoint',
  model: 'popupModel',
  key: 'popupKey',
  network: 'popupNetwork',
  timeout: 'popupTimeout',
  incomplete: 'streamIncomplete',
  streamUnsupported: 'streamUnsupported',
  service: 'popupService',
} as const
</script>

<template>
  <main v-if="selection && pair" ref="panel" class="selection-popup" :aria-label="t('popupTitle')">
    <header class="popup-header" @mousedown.left.self.prevent="action('drag')">
      <button
        type="button"
        class="popup-icon pin"
        :aria-label="t('pinWindow')"
        :aria-pressed="state.pinned"
        :title="t('pinWindow')"
        @click="action('pin')"
      >
        <AppIcon name="pin" :size="17" />
      </button>
      <button
        type="button"
        class="popup-icon"
        :aria-label="t('closeWindow')"
        :title="t('closeWindow')"
        @click="action('dismiss')"
      >
        <AppIcon name="close" :size="18" />
      </button>
    </header>
    <div
      v-for="entry in bodies"
      :key="entry.selectionId"
      class="popup-body"
      :inert="entry.selectionId !== selection.selectionId || undefined"
      tabindex="0"
      :aria-label="t('result')"
    >
      <div aria-live="polite" aria-atomic="true" :aria-busy="entry.status === 'streaming'">
        <template v-if="!pendingRequest && entry.status === 'success'">
          <p class="popup-translation">{{ entry.translation }}</p>
          <p v-if="!entry.historySaved" class="popup-warning">{{ t('historyNotSaved') }}</p>
        </template>
        <p v-else-if="!pendingRequest && entry.status === 'streaming'" class="popup-translation">
          {{ entry.translation }}
        </p>
        <p v-else-if="!pendingRequest && entry.status === 'error'" class="popup-error">
          {{ t(failureKeys[entry.error]) }}
        </p>
        <p v-else class="popup-loading">{{ messageForLocale(entry.locale, 'translating') }}</p>
        <p
          v-if="!pendingRequest && entry.targetLang !== pair.targetLang"
          class="popup-result-language"
        >
          {{ t('resultLanguage') }} · {{ t(entry.targetLang) }}
        </p>
      </div>
      <textarea
        v-if="expanded && entry.selectionId === selection.selectionId"
        id="selection-original"
        v-model="sourceText"
        class="popup-original"
        :aria-label="t('source')"
        rows="2"
      ></textarea>
      <p
        v-if="(notice || targetError) && entry.selectionId === selection.selectionId"
        class="popup-error"
        role="alert"
      >
        {{ t(notice || targetError) }}
      </p>
    </div>
    <div class="popup-actions">
      <button
        type="button"
        class="popup-disclosure"
        :aria-expanded="expanded"
        :aria-controls="expanded ? 'selection-original' : undefined"
        @click="toggleOriginal"
      >
        <AppIcon name="chevron" :size="15" />{{ t('viewOriginal') }}
      </button>
      <button
        type="button"
        class="popup-icon popup-copy"
        :disabled="!canCopy"
        :aria-label="t('copyTranslation')"
        :title="t('copyTranslation')"
        @click="action('copy')"
      >
        <AppIcon :name="copied ? 'check' : 'copy'" :size="16" />
      </button>
      <span class="sr-only" role="status">{{ copied ? t('copied') : '' }}</span>
      <button
        type="button"
        class="popup-icon popup-retranslate"
        :disabled="!canRetranslate"
        :aria-label="t('retranslate')"
        :title="t('retranslate')"
        @click="$event.detail < 2 && retranslate()"
      >
        <AppIcon name="translate" :size="17" />
      </button>
    </div>
    <div :key="selection.selectionId" class="popup-languages">
      <AppSelect
        id="popup-source"
        :model-value="pair.sourceLang"
        :options="sourceOptions"
        :disabled="!canChangeLanguages"
        :label="t('sourceLang')"
        @update:model-value="languages($event, pair!.targetLang)"
      />
      <button
        type="button"
        class="popup-icon"
        :disabled="!canChangeLanguages || pair.sourceLang === 'auto'"
        :aria-label="t('swap')"
        :aria-describedby="pair.sourceLang === 'auto' ? 'popup-swap-hint' : undefined"
        :title="t(pair.sourceLang === 'auto' ? 'cannotSwapAuto' : 'swap')"
        @click="languages(pair.targetLang, pair.sourceLang)"
      >
        <AppIcon name="swap" :size="16" />
      </button>
      <span v-if="pair.sourceLang === 'auto'" id="popup-swap-hint" class="sr-only">{{
        t('cannotSwapAuto')
      }}</span>
      <AppSelect
        id="popup-target"
        :model-value="pair.targetLang"
        :options="targetOptions"
        :disabled="!canChooseTarget"
        :label="t('targetLang')"
        @update:model-value="languages(pair!.sourceLang, $event)"
      />
    </div>
  </main>
  <p v-else-if="notice" role="alert" class="popup-error">{{ t(notice) }}</p>
</template>
