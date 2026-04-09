<template>
  <div class="space-y-3 rounded-2xl border border-base-300 bg-base-200/35 px-4 py-4">
    <div class="flex items-center justify-between gap-3">
      <div>
        <div class="text-sm font-medium text-base-content/75">搜索洞察</div>
        <div class="text-xs text-base-content/50">本地记录近期搜索、命令和结果打开行为</div>
      </div>
      <button class="btn btn-ghost btn-xs" @click="$emit('clear')">清空统计</button>
    </div>

    <div class="grid gap-3 md:grid-cols-3">
      <div class="rounded-2xl border border-base-300 bg-base-100 px-3 py-3">
        <div class="text-xs text-base-content/50">搜索提交</div>
        <div class="mt-2 text-2xl font-semibold">{{ summary.totalSearches }}</div>
      </div>
      <div class="rounded-2xl border border-base-300 bg-base-100 px-3 py-3">
        <div class="text-xs text-base-content/50">命令执行</div>
        <div class="mt-2 text-2xl font-semibold">{{ summary.totalCommands }}</div>
      </div>
      <div class="rounded-2xl border border-base-300 bg-base-100 px-3 py-3">
        <div class="text-xs text-base-content/50">结果打开</div>
        <div class="mt-2 text-2xl font-semibold">{{ summary.totalOpens }}</div>
      </div>
    </div>

    <div class="grid gap-3 lg:grid-cols-3">
      <div class="space-y-2 rounded-2xl border border-base-300 bg-base-100 px-3 py-3">
        <div class="text-xs font-medium text-base-content/55">高频搜索</div>
        <div v-if="summary.topQueries.length > 0" class="flex flex-wrap gap-2">
          <span v-for="item in summary.topQueries" :key="`query-${item.value}`" class="badge badge-outline badge-sm">
            {{ item.value }} {{ item.count }}
          </span>
        </div>
        <div v-else class="text-xs text-base-content/45">还没有搜索数据</div>
      </div>

      <div class="space-y-2 rounded-2xl border border-base-300 bg-base-100 px-3 py-3">
        <div class="text-xs font-medium text-base-content/55">高频命令</div>
        <div v-if="summary.topCommands.length > 0" class="flex flex-wrap gap-2">
          <span v-for="item in summary.topCommands" :key="`command-${item.value}`" class="badge badge-outline badge-sm font-mono">
            {{ item.value }} {{ item.count }}
          </span>
        </div>
        <div v-else class="text-xs text-base-content/45">还没有命令数据</div>
      </div>

      <div class="space-y-2 rounded-2xl border border-base-300 bg-base-100 px-3 py-3">
        <div class="text-xs font-medium text-base-content/55">常开结果</div>
        <div v-if="summary.topOpenedEntries.length > 0" class="flex flex-wrap gap-2">
          <span v-for="item in summary.topOpenedEntries" :key="`open-${item.value}`" class="badge badge-outline badge-sm">
            {{ item.value }} {{ item.count }}
          </span>
        </div>
        <div v-else class="text-xs text-base-content/45">还没有打开结果记录</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { SearchAnalyticsSummary } from '@/services/searchAnalytics'

defineProps<{
  summary: SearchAnalyticsSummary
}>()

defineEmits<{
  clear: []
}>()
</script>
