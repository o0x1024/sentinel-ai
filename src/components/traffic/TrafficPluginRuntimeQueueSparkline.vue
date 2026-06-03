<template>
  <div class="rounded border border-base-300/80 bg-base-200/25 p-2">
    <div class="flex items-center justify-between gap-2 text-[11px]">
      <span class="text-base-content/55">{{ label }}</span>
      <span class="font-medium text-base-content/80">{{ value }}</span>
    </div>
    <div class="mt-2 h-10">
      <svg class="h-full w-full" viewBox="0 0 120 40" preserveAspectRatio="none" aria-hidden="true">
        <path d="M 0 37 L 120 37" class="fill-none stroke-base-300/70" stroke-width="1" />
        <path
          v-if="path"
          :d="path"
          fill="none"
          :class="strokeClass"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
        />
      </svg>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { buildTrafficPluginRuntimeSparklinePath } from './trafficPluginRuntimeQueueHistorySupport'

const props = withDefaults(
  defineProps<{
    label: string
    value: string
    values: number[]
    strokeClass?: string
  }>(),
  {
    strokeClass: 'text-primary',
  }
)

const path = computed(() => buildTrafficPluginRuntimeSparklinePath(props.values, 120, 40))
</script>
