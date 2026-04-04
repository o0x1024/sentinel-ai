<template>
  <div class="rounded-lg border border-base-300 bg-base-200/40 p-4">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div class="space-y-1">
        <div class="flex flex-wrap items-center gap-2">
          <div class="text-lg font-semibold">{{ name }}</div>
          <span class="badge badge-sm" :class="modeBadge.className">
            {{ modeBadge.label }}
          </span>
        </div>
        <div class="text-sm text-base-content/70">
          {{ description }}
        </div>
        <div v-if="passiveEventName" class="text-xs text-base-content/60">
          自动事件：{{ passiveEventName }}
        </div>
      </div>
      <label class="label cursor-pointer justify-start gap-3">
        <input :checked="enabled" type="checkbox" class="toggle toggle-primary" @change="handleToggle" />
        <span class="label-text">启用</span>
      </label>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { SystemAgentModeBadge } from './systemAgentRegistry'

const props = defineProps<{
  name: string
  description: string
  modeBadge: SystemAgentModeBadge
  passiveEventName: string
  enabled: boolean
}>()

const emit = defineEmits<{
  'update:enabled': [value: boolean]
}>()

function handleToggle(event: Event) {
  const target = event.target as HTMLInputElement | null
  emit('update:enabled', target?.checked === true)
}
</script>
