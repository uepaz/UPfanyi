<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{ timestamp: string }>()

function twoDigits(value: number) {
  return String(value).padStart(2, '0')
}

const formatted = computed(() => {
  const date = new Date(props.timestamp)
  if (Number.isNaN(date.getTime())) return ''

  const year = String(date.getFullYear()).padStart(4, '0')
  const day = `${year}-${twoDigits(date.getMonth() + 1)}-${twoDigits(date.getDate())}`
  const time = [date.getHours(), date.getMinutes(), date.getSeconds()].map(twoDigits).join(':')
  return `${day} ${time}`
})
</script>

<template>
  <time v-if="formatted" :datetime="timestamp">{{ formatted }}</time>
  <template v-else>—</template>
</template>
