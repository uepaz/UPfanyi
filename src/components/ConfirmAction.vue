<script setup lang="ts">
import { onMounted, onUnmounted, ref, useId, watch } from 'vue'
import { useI18n } from '../i18n'
const props = defineProps<{ message: string; busy: boolean; fallbackFocus?: HTMLElement | null }>()
const emit = defineEmits<{ cancel: []; confirm: [] }>()
const { t } = useI18n()
const id = useId()
const dialog = ref<HTMLElement | null>(null)
const cancelButton = ref<HTMLButtonElement | null>(null)
let previousFocus: HTMLElement | null = null
onMounted(() => {
  previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null
  if (props.busy) dialog.value?.focus()
  else cancelButton.value?.focus()
  document.addEventListener('keydown', keydown)
})
onUnmounted(() => {
  document.removeEventListener('keydown', keydown)
  if (previousFocus?.isConnected) previousFocus.focus()
  else props.fallbackFocus?.focus()
})
watch(
  () => props.busy,
  (value) => {
    if (value) dialog.value?.focus()
    else cancelButton.value?.focus()
  },
)
function keydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault()
    if (!props.busy) emit('cancel')
  }
  if (event.key !== 'Tab') return
  const buttons = Array.from(
    dialog.value?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? [],
  )
  if (!buttons.length) {
    event.preventDefault()
    dialog.value?.focus()
    return
  }
  const first = buttons[0]!
  const last = buttons[buttons.length - 1]!
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
</script>
<template>
  <div class="confirmation-backdrop">
    <section
      ref="dialog"
      tabindex="-1"
      class="confirmation card"
      role="alertdialog"
      aria-modal="true"
      :aria-labelledby="id"
    >
      <p :id="id">{{ message }}</p>
      <div class="confirmation-actions">
        <button ref="cancelButton" class="button subtle" :disabled="busy" @click="emit('cancel')">
          {{ t('cancel') }}
        </button>
        <button class="button danger" :disabled="busy" @click="emit('confirm')">
          {{ busy ? t('saving') : t('delete') }}
        </button>
      </div>
    </section>
  </div>
</template>
