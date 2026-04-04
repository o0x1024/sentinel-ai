<template>
  <div class="border border-base-300 rounded-lg overflow-hidden">
    <div class="px-4 py-3 bg-base-200 flex items-center justify-between">
      <div class="font-semibold text-sm">最近版本</div>
      <button class="btn btn-xs btn-ghost" @click="$emit('refresh')" :disabled="disabled">
        <i class="fas fa-rotate mr-1"></i>
        刷新
      </button>
    </div>
    <div v-if="versions.length === 0" class="p-4 text-sm text-base-content/60">
      当前还没有版本快照。
    </div>
    <div v-else class="divide-y divide-base-300 max-h-[320px] overflow-y-auto">
      <div v-for="version in versions" :key="version.id" class="p-4">
        <div class="flex items-center justify-between gap-3">
          <div class="text-sm font-medium">配置快照</div>
          <span class="text-xs text-base-content/60">{{ formatDate(version.createdAt) }}</span>
        </div>
        <details class="mt-2">
          <summary class="cursor-pointer text-xs text-base-content/50 select-none">
            查看版本详情
          </summary>
          <pre class="mt-3 p-3 bg-base-200 rounded-lg text-xs overflow-auto">{{ JSON.stringify(version.snapshot, null, 2) }}</pre>
        </details>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { SystemAgentProfileVersionPayload } from '../systemAgentSettingsSupport'

defineProps<{
  versions: SystemAgentProfileVersionPayload[]
  disabled?: boolean
}>()

defineEmits<{
  refresh: []
}>()

function formatDate(value?: string | null) {
  if (!value) return '-'
  try {
    return new Date(value).toLocaleString()
  } catch {
    return value
  }
}
</script>
