<script setup lang="ts">
import { computed, nextTick, onScopeDispose, ref, watch } from 'vue'
import AppIcon from './AppIcon.vue'
import { onSelectScopeChange } from '../composables/useSelectScope'

const props = defineProps<{
  id: string
  modelValue: string
  options: { value: string; label: string; disabled?: boolean; action?: string }[]
  label: string
  describedBy?: string
  disabled?: boolean
}>()
const emit = defineEmits<{ 'update:modelValue': [value: string]; action: [value: string] }>()
const trigger = ref<HTMLButtonElement>()
const menu = ref<HTMLElement>()
const open = ref(false)
const active = ref(-1)
const placement = ref<Record<string, string>>({})
const selected = computed(() =>
  props.options.find((item) => !item.action && item.value === props.modelValue),
)
const enabled = computed(() =>
  props.options.flatMap((item, index) => (item.disabled ? [] : [index])),
)

function close() {
  open.value = false
}
onSelectScopeChange(close)
function position() {
  const bounds = trigger.value?.getBoundingClientRect()
  if (!bounds) return
  // 弹层挂载到 body，避免设置滚动区域裁切；空间不足时优先向上展开。
  const below = window.innerHeight - bounds.bottom - 12
  const above = bounds.top - 12
  const height = Math.min(props.options.length * 42 + 12, 240)
  const upward = below < height && above > below
  const available = Math.max(42, upward ? above : below)
  placement.value = {
    left: `${Math.max(8, Math.min(bounds.left, window.innerWidth - bounds.width - 8))}px`,
    width: `${Math.min(bounds.width, window.innerWidth - 16)}px`,
    maxHeight: `${Math.min(height, available)}px`,
    ...(upward
      ? { bottom: `${window.innerHeight - bounds.top + 6}px` }
      : { top: `${bounds.bottom + 6}px` }),
  }
}
function scrollActive() {
  void nextTick(() => {
    menu.value
      ?.querySelector(`#${props.id}-option-${active.value}`)
      ?.scrollIntoView?.({ block: 'nearest' })
  })
}
function show() {
  if (props.disabled || trigger.value?.matches(':disabled')) return
  active.value = props.options.findIndex(
    (item) => !item.action && item.value === props.modelValue && !item.disabled,
  )
  if (active.value < 0) active.value = enabled.value[0] ?? -1
  position()
  open.value = true
  scrollActive()
}
function choose(index: number) {
  const option = props.options[index]
  if (props.disabled || !option || option.disabled) return
  close()
  trigger.value?.focus()
  // 添加入口只触发动作，不污染选择值或持久化的渠道身份。
  if (option.action) emit('action', option.action)
  else emit('update:modelValue', option.value)
}
function keydown(event: KeyboardEvent) {
  if (props.disabled || trigger.value?.matches(':disabled')) return
  if (event.key === 'Tab') {
    close()
    return
  }
  if (event.key === 'Escape') {
    event.preventDefault()
    close()
    return
  }
  if (!['ArrowDown', 'ArrowUp', 'Home', 'End', 'Enter', ' '].includes(event.key)) return
  event.preventDefault()
  if (!open.value) {
    show()
    if (event.key === 'Home') active.value = enabled.value[0] ?? -1
    if (event.key === 'End') active.value = enabled.value.at(-1) ?? -1
    scrollActive()
    return
  }
  if (event.key === 'Enter' || event.key === ' ') {
    choose(active.value)
    return
  }
  const indices = enabled.value
  const current = indices.indexOf(active.value)
  if (event.key === 'Home') active.value = indices[0] ?? -1
  else if (event.key === 'End') active.value = indices.at(-1) ?? -1
  else
    active.value =
      indices[(current + (event.key === 'ArrowDown' ? 1 : -1) + indices.length) % indices.length] ??
      -1
  scrollActive()
}
function outside(event: Event) {
  const target = event.target as Node
  if (!trigger.value?.contains(target) && !menu.value?.contains(target)) close()
}
function viewportChanged() {
  if (open.value) position()
}
watch(
  () => props.disabled,
  (disabled) => {
    if (disabled) close()
  },
)
watch(
  () => props.options,
  () => {
    if (open.value && !enabled.value.includes(active.value)) active.value = enabled.value[0] ?? -1
    viewportChanged()
  },
)
document.addEventListener('pointerdown', outside)
window.addEventListener('resize', viewportChanged)
window.addEventListener('scroll', viewportChanged, true)
onScopeDispose(() => {
  document.removeEventListener('pointerdown', outside)
  window.removeEventListener('resize', viewportChanged)
  window.removeEventListener('scroll', viewportChanged, true)
})
</script>

<template>
  <button
    :id="id"
    ref="trigger"
    type="button"
    class="app-select-trigger"
    role="combobox"
    :disabled="disabled"
    :data-value="modelValue"
    :aria-label="label"
    aria-haspopup="listbox"
    :aria-expanded="open"
    :aria-controls="`${id}-listbox`"
    :aria-describedby="describedBy"
    :aria-activedescendant="open && active >= 0 ? `${id}-option-${active}` : undefined"
    @click="open ? close() : show()"
    @keydown="keydown"
    @blur="close"
  >
    <span>{{ selected?.label ?? modelValue }}</span>
    <svg
      class="app-select-chevron"
      :class="{ expanded: open }"
      width="16"
      height="16"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
      aria-hidden="true"
    >
      <path d="m6 9 6 6 6-6" />
    </svg>
  </button>
  <Teleport to="body">
    <div
      v-if="open"
      :id="`${id}-listbox`"
      ref="menu"
      class="app-select-menu"
      role="listbox"
      :aria-label="label"
      :style="placement"
      @mousedown.prevent
    >
      <div
        v-for="(option, index) in options"
        :id="`${id}-option-${index}`"
        :key="option.value"
        role="option"
        :data-value="option.value"
        :data-action="option.action"
        :aria-selected="option.action ? undefined : option.value === modelValue"
        :aria-disabled="option.disabled ?? false"
        class="app-select-option"
        :class="{
          active: active === index,
          selected: !option.action && option.value === modelValue,
          'app-select-action': !!option.action,
        }"
        @pointermove="!option.disabled && (active = index)"
        @click="choose(index)"
      >
        <span>{{ option.label }}</span
        ><AppIcon v-if="!option.action && option.value === modelValue" name="check" :size="17" />
      </div>
    </div>
  </Teleport>
</template>
