<template>
  <div v-if="items.length > 0" class="space-y-1 text-[11px]">
    <div
      v-for="item in items"
      :key="item.key"
      :class="taskMetaToneClass(item.tone)"
    >
      <span class="font-medium">{{ $t(item.labelKey) }}:</span>
      {{ item.value }}
    </div>
  </div>
</template>

<script setup lang="ts">
import type { TaskMetaItem } from './taskMetaPresentation'
import { taskMetaToneClass } from './taskMetaPresentation'

defineProps<{
  items: TaskMetaItem[]
}>()
</script>
