<script setup lang="ts">
defineProps<{ kind?: 'error' | 'success' | 'warning' | 'info' }>()
</script>
<template>
  <div :class="['notice', kind ?? 'info']" :role="kind === 'error' ? 'alert' : 'status'">
    <slot />
  </div>
</template>
