<script setup lang="ts">
import { computed, nextTick, onScopeDispose, ref, watch } from 'vue'
import AppIcon from './AppIcon.vue'
import { onSelectScopeChange } from '../composables/useSelectScope'

const props = defineProps<{
  id: string
  modelValue: string
  options: string[]
  label: string
  describedBy?: string
  disabled?: boolean
  placeholder?: string
}>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()
const root = ref<HTMLElement>()
const input = ref<HTMLInputElement>()
const menu = ref<HTMLElement>()
const query = ref(props.modelValue)
const showAll = ref(false)
const open = ref(false)
const active = ref(-1)
const placement = ref<Record<string, string>>({})
const filtered = computed(() => {
  const needle = showAll.value ? '' : query.value.toLocaleLowerCase()
  return [...new Set(props.options)].filter((option) => option.toLocaleLowerCase().includes(needle))
})

function close() {
  open.value = false
  active.value = -1
  showAll.value = false
}
onSelectScopeChange(close)
function position() {
  const bounds = root.value?.getBoundingClientRect()
  if (!bounds) return
  // 使用与普通选择框相同的 body 弹层，避免设置页滚动容器裁切候选。
  const below = window.innerHeight - bounds.bottom - 12
  const above = bounds.top - 12
  const height = Math.min(filtered.value.length * 42 + 12, 240)
  const upward = below < height && above > below
  placement.value = {
    left: `${Math.max(8, Math.min(bounds.left, window.innerWidth - bounds.width - 8))}px`,
    width: `${Math.min(bounds.width, window.innerWidth - 16)}px`,
    maxHeight: `${Math.min(height, Math.max(42, upward ? above : below))}px`,
    ...(upward
      ? { bottom: `${window.innerHeight - bounds.top + 6}px` }
      : { top: `${bounds.bottom + 6}px` }),
  }
}
function show() {
  if (props.disabled || input.value?.matches(':disabled') || !filtered.value.length) return
  position()
  open.value = true
}
function scrollActive() {
  void nextTick(() => {
    menu.value?.children[active.value]?.scrollIntoView?.({ block: 'nearest' })
  })
}
function choose(index: number) {
  const value = filtered.value[index]
  if (props.disabled || input.value?.matches(':disabled') || value === undefined) return
  query.value = value
  emit('update:modelValue', value)
  input.value?.focus()
  close()
}
function edited(event: Event) {
  if (props.disabled || input.value?.matches(':disabled')) return
  query.value = (event.target as HTMLInputElement).value
  showAll.value = false
  active.value = -1
  emit('update:modelValue', query.value)
  if (filtered.value.length) show()
  else close()
}
function toggle() {
  if (props.disabled || input.value?.matches(':disabled')) return
  if (open.value && showAll.value) close()
  else {
    // 展开箭头提供完整列表，手填模型不匹配时也能直接改选，输入才启用过滤。
    showAll.value = true
    input.value?.focus()
    show()
  }
}
function keydown(event: KeyboardEvent) {
  if (props.disabled || input.value?.matches(':disabled') || event.isComposing) return
  if (event.key === 'Tab') close()
  else if (event.key === 'Escape') {
    if (open.value) event.preventDefault()
    close()
  } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
    // 闭合时方向键与展开箭头一致；未匹配的手填值无需清空即可浏览完整列表。
    if (!open.value) showAll.value = true
    if (!filtered.value.length) return
    event.preventDefault()
    show()
    active.value =
      active.value < 0
        ? event.key === 'ArrowDown'
          ? 0
          : filtered.value.length - 1
        : (active.value + (event.key === 'ArrowDown' ? 1 : -1) + filtered.value.length) %
          filtered.value.length
    scrollActive()
  } else if (event.key === 'Enter' && open.value) {
    // 候选确认不能顺带提交整个设置表单；任意手填值仍由输入事件保留。
    event.preventDefault()
    if (active.value >= 0) choose(active.value)
    else close()
  }
  // Home/End 与 Space 保留文本输入语义，不接管输入光标和普通字符。
}
function outside(event: Event) {
  const target = event.target as Node
  if (!root.value?.contains(target) && !menu.value?.contains(target)) close()
}
function viewportChanged() {
  if (open.value) position()
}
watch(
  () => props.modelValue,
  (value) => (query.value = value),
)
watch(
  () => props.disabled,
  (disabled) => disabled && close(),
)
watch(
  filtered,
  (values, previous) => {
    const value = previous[active.value]
    active.value = value === undefined ? -1 : values.indexOf(value)
    if (!values.length) close()
    viewportChanged()
  },
  { flush: 'sync' },
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
  <div ref="root" class="app-combobox" :class="{ disabled }">
    <input
      :id="id"
      ref="input"
      class="app-combobox-input"
      :value="modelValue"
      :disabled="disabled"
      :placeholder="placeholder"
      autocomplete="off"
      spellcheck="false"
      role="combobox"
      :aria-label="label"
      aria-autocomplete="list"
      aria-haspopup="listbox"
      :aria-expanded="open"
      :aria-controls="`${id}-listbox`"
      :aria-describedby="describedBy"
      :aria-activedescendant="open && active >= 0 ? `${id}-option-${active}` : undefined"
      @focus="show"
      @click="show"
      @input="edited"
      @keydown="keydown"
      @blur="close"
    />
    <button
      class="app-combobox-toggle"
      type="button"
      tabindex="-1"
      :disabled="disabled || !options.length"
      :aria-label="label"
      :aria-expanded="open"
      :aria-controls="`${id}-listbox`"
      @mousedown.prevent
      @click="toggle"
    >
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
  </div>
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
        v-for="(option, index) in filtered"
        :id="`${id}-option-${index}`"
        :key="option"
        class="app-select-option"
        :class="{ active: active === index, selected: option === modelValue }"
        role="option"
        :data-value="option"
        :aria-selected="option === modelValue"
        @pointermove="active = index"
        @click="choose(index)"
      >
        <span>{{ option }}</span
        ><AppIcon v-if="option === modelValue" name="check" :size="17" />
      </div>
    </div>
  </Teleport>
</template>
