<template>
  <div class="border border-base-300 rounded-lg overflow-hidden">
    <div class="px-4 py-3 bg-base-200 text-sm font-semibold">系统智能体列表</div>
    <div v-if="loading" class="p-6 text-center">
      <span class="loading loading-spinner loading-md"></span>
    </div>
    <div v-else-if="items.length === 0" class="p-6 text-sm text-base-content/60">
      当前还没有可用系统智能体。
    </div>
    <div v-else class="max-h-[720px] overflow-y-auto">
      <button
        v-for="item in items"
        :key="item.id"
        class="w-full text-left px-4 py-3 border-b border-base-300 hover:bg-base-200/70 transition-colors"
        :class="{ 'bg-primary/10': selectedId === item.id }"
        @click="$emit('select', item.id)"
      >
        <div class="flex items-center justify-between gap-2">
          <div class="font-medium truncate">{{ item.name }}</div>
          <div class="flex gap-1">
            <span class="badge badge-xs" :class="item.modeBadge.className">
              {{ item.modeBadge.label }}
            </span>
            <span class="badge badge-xs" :class="item.enabled ? 'badge-success' : 'badge-ghost'">
              {{ item.enabled ? '已启用' : '已停用' }}
            </span>
          </div>
        </div>
        <div class="text-xs text-base-content/70 mt-2 line-clamp-2">{{ item.description }}</div>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { SystemAgentListItem } from '../systemAgentSettingsSupport'

defineProps<{
  loading: boolean
  items: SystemAgentListItem[]
  selectedId: string
}>()

defineEmits<{
  select: [profileId: string]
}>()
</script>
