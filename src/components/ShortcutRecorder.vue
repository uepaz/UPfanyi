<script setup lang="ts">
import { computed, onScopeDispose, ref, watch } from 'vue'
import { useI18n } from '../i18n'
import { onSelectScopeChange } from '../composables/useSelectScope'
import { shortcutFromEvent, shortcutModifiers } from '../services/shortcuts'

const props = defineProps<{
  id: string
  modelValue: string
  label: string
  disabled?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const { t } = useI18n()
const input = ref<HTMLInputElement>()
const recording = ref(false)
const preview = ref('')
const unsupported = ref(false)
let disposed = false
const value = computed(() => {
  const text = recording.value ? preview.value || t('hotkeyWaiting') : props.modelValue
  return text
    .split('+')
    .map((token) => (token === 'Super' ? 'Win' : token))
    .join('+')
})
function stop() {
  recording.value = false
  preview.value = ''
  unsupported.value = false
}
function start() {
  if (disposed || props.disabled || input.value?.matches(':disabled')) return
  recording.value = true
  preview.value = ''
  unsupported.value = false
}
function consume(event: KeyboardEvent) {
  event.preventDefault()
  event.stopPropagation()
}
function keydown(event: KeyboardEvent) {
  if (disposed || props.disabled || input.value?.matches(':disabled')) return
  const modifiers = shortcutModifiers(event)
  if (!recording.value) {
    // 录制结束后继续按住 Enter 也不能意外提交设置。
    if (event.key === 'Enter' || event.key === ' ') {
      consume(event)
      if (!modifiers.length && !event.repeat && !event.isComposing && event.keyCode !== 229) start()
    }
    return
  }
  if (event.isComposing || event.keyCode === 229 || event.repeat || event.key === 'Process') {
    consume(event)
    return
  }
  if (event.key === 'Tab' && !event.ctrlKey && !event.altKey && !event.metaKey) {
    stop()
    return
  }
  consume(event)
  if (event.key === 'Escape' && !modifiers.length) {
    stop()
    return
  }
  if (['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) {
    preview.value = modifiers.length ? `${modifiers.join('+')}+…` : ''
    unsupported.value = false
    return
  }
  const shortcut = shortcutFromEvent(event)
  if (!shortcut) {
    unsupported.value = true
    return
  }
  stop()
  emit('update:modelValue', shortcut)
}
function keyup(event: KeyboardEvent) {
  if (!recording.value) return
  event.stopPropagation()
  if (event.isComposing || event.keyCode === 229) return
  const modifiers = shortcutModifiers(event)
  preview.value = modifiers.length ? `${modifiers.join('+')}+…` : ''
}
onSelectScopeChange(stop)
watch(() => [props.disabled, props.modelValue], stop, { flush: 'sync' })
window.addEventListener('blur', stop)
onScopeDispose(() => {
  disposed = true
  stop()
  window.removeEventListener('blur', stop)
})
</script>

<template>
  <div class="shortcut-recorder" :class="{ recording }">
    <div class="shortcut-recorder-control">
      <input
        :id="id"
        ref="input"
        :value="value"
        readonly
        :disabled="disabled"
        :aria-label="label"
        :aria-describedby="`${id}-status hotkey-hint`"
        :aria-invalid="unsupported"
        autocomplete="off"
        spellcheck="false"
        @click="start"
        @keydown="keydown"
        @keyup="keyup"
        @blur="stop"
        @paste.prevent
        @drop.prevent
      />
      <span class="shortcut-recorder-badge" aria-hidden="true">{{
        t(recording ? 'hotkeyRecording' : 'hotkeyClickRecord')
      }}</span>
    </div>
    <small
      :id="`${id}-status`"
      class="shortcut-recorder-status"
      :class="{ invalid: unsupported }"
      aria-live="polite"
      >{{ unsupported ? t('hotkeyUnsupported') : recording ? t('hotkeyRecordCancel') : '' }}</small
    >
  </div>
</template>
