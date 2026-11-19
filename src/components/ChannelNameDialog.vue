<script setup lang="ts">
import { onMounted, onScopeDispose, ref, useId } from 'vue'
import { useI18n } from '../i18n'

const props = defineProps<{ title: string; initialName: string; error?: string }>()
const emit = defineEmits<{ cancel: []; confirm: [name: string] }>()
const { t } = useI18n()
const id = useId()
const name = ref(props.initialName)
const dialog = ref<HTMLElement>()
const input = ref<HTMLInputElement>()
const composing = ref(false)
let previousFocus: HTMLElement | null = null
function keydown(event: KeyboardEvent) {
  if (event.isComposing) return
  if (event.key === 'Escape') {
    event.preventDefault()
    emit('cancel')
  }
  if (event.key !== 'Tab') return
  const controls = Array.from(dialog.value?.querySelectorAll<HTMLElement>('input, button') ?? [])
  const first = controls[0]
  const last = controls.at(-1)
  if (!first || !last) return
  if (!dialog.value?.contains(document.activeElement)) {
    event.preventDefault()
    ;(event.shiftKey ? last : first).focus()
  } else if (event.shiftKey && document.activeElement === first) {
    event.preventDefault()
    last.focus()
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault()
    first.focus()
  }
}
function confirm() {
  if (!composing.value) emit('confirm', name.value)
}
function inputKeydown(event: KeyboardEvent) {
  // 输入法确认候选的 Enter 不应同时提交渠道名称。
  if (event.key === 'Enter' && (event.isComposing || event.keyCode === 229)) event.preventDefault()
}
onMounted(() => {
  previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
  input.value?.focus()
  input.value?.select()
  document.addEventListener('keydown', keydown)
})
onScopeDispose(() => {
  document.removeEventListener('keydown', keydown)
  if (previousFocus?.isConnected) previousFocus.focus()
})
</script>
<template>
  <Teleport to="body">
    <div class="confirmation-backdrop">
      <form
        ref="dialog"
        class="confirmation card channel-name-dialog"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="`${id}-title`"
        @submit.prevent="confirm"
      >
        <h2 :id="`${id}-title`">{{ title }}</h2>
        <label class="field" :for="`${id}-name`">
          <span>{{ t('channelName') }}</span>
          <input
            :id="`${id}-name`"
            ref="input"
            v-model="name"
            autocomplete="off"
            :aria-invalid="!!error"
            :aria-describedby="error ? `${id}-error` : undefined"
            @compositionstart="composing = true"
            @compositionend="composing = false"
            @keydown="inputKeydown"
          />
        </label>
        <p v-if="error" :id="`${id}-error`" class="channel-name-error" role="alert">{{ error }}</p>
        <div class="confirmation-actions">
          <button class="button subtle" type="button" @click="emit('cancel')">
            {{ t('cancel') }}
          </button>
          <button class="button primary" type="submit">{{ t('confirm') }}</button>
        </div>
      </form>
    </div>
  </Teleport>
</template>
