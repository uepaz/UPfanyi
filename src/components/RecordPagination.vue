<script setup lang="ts">
import { useI18n } from '../i18n'
defineProps<{ total: number; page: number; pages: number; loading: boolean }>()
defineEmits<{ previous: []; next: [] }>()
const { t } = useI18n()
</script>
<template>
  <div class="pagination">
    <span>{{ total }} {{ t('totalRecords') }}</span>
    <div>
      <button
        class="button subtle small"
        :disabled="page <= 1 || loading"
        @click="$emit('previous')"
      >
        {{ t('previous') }}</button
      ><span>{{ page }} / {{ pages }}</span
      ><button
        class="button subtle small"
        :disabled="page >= pages || loading"
        @click="$emit('next')"
      >
        {{ t('next') }}
      </button>
    </div>
  </div>
</template>
