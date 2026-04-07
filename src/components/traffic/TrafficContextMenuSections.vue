<template>
  <template v-for="(section, sectionIndex) in sections" :key="section.key">
    <div v-if="sectionIndex > 0" class="divider my-1 h-0"></div>
    <button
      v-for="item in section.items"
      :key="item.key"
      class="w-full px-4 py-2 text-left text-sm hover:bg-base-200 flex items-center gap-2"
      :disabled="item.disabled"
      :class="{ 'opacity-50 cursor-not-allowed': item.disabled }"
      @click="item.onClick"
    >
      <i :class="item.iconClass"></i>
      {{ t(`${labelPrefix}.${item.labelKey}`) }}
    </button>
  </template>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { TrafficContextMenuSection } from './trafficContextMenuSectionSupport'

defineProps<{
  sections: TrafficContextMenuSection[]
  labelPrefix: string
}>()

const { t } = useI18n()
</script>
