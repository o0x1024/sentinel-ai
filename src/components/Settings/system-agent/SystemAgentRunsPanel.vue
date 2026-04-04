<template>
  <div class="border border-base-300 rounded-lg overflow-hidden">
    <div class="px-4 py-3 bg-base-200 flex items-center justify-between">
      <div class="font-semibold text-sm">运行记录</div>
      <button class="btn btn-xs btn-ghost" @click="$emit('refresh')" :disabled="disabled">
        <i class="fas fa-rotate mr-1"></i>
        刷新
      </button>
    </div>
    <div v-if="runs.length === 0" class="p-4 text-sm text-base-content/60">
      当前没有运行记录。
    </div>
    <div v-else class="divide-y divide-base-300 max-h-[420px] overflow-y-auto">
      <div v-for="run in runs" :key="run.id" class="p-4">
        <div class="flex items-center justify-between gap-3">
          <div class="text-sm font-medium">
            {{ formatRunStatus(run.status) }}
          </div>
          <span
            class="badge badge-sm"
            :class="statusBadgeClass(run.status)"
          >
            {{ formatRunStatus(run.status) }}
          </span>
        </div>
        <div class="text-xs text-base-content/60 mt-1">
          {{ formatTriggerEvent(run.triggerEvent) }} · {{ formatDate(run.startedAt) }}
        </div>
        <details class="mt-2">
          <summary class="cursor-pointer text-xs text-base-content/50 select-none">
            查看运行详情
          </summary>
          <div class="mt-2 text-xs text-base-content/60 font-mono break-all">
            {{ run.id }}
          </div>
        </details>
        <pre v-if="run.output" class="mt-3 p-3 bg-base-200 rounded-lg text-xs overflow-auto">{{ JSON.stringify(run.output, null, 2) }}</pre>
        <div v-else-if="run.errorMessage" class="mt-3 p-3 rounded-lg bg-error/10 text-error text-xs whitespace-pre-wrap">{{ run.errorMessage }}</div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { SystemAgentRunPayload } from '../systemAgentSettingsSupport'

defineProps<{
  runs: SystemAgentRunPayload[]
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

function formatRunStatus(status: string) {
  if (status === 'completed') return '已完成'
  if (status === 'failed') return '失败'
  if (status === 'running') return '运行中'
  if (status === 'queued') return '排队中'
  if (status === 'retrying') return '重试中'
  if (status === 'dead_letter') return '已转死信'
  return status || '未知状态'
}

function statusBadgeClass(status: string) {
  if (status === 'completed') return 'badge-success'
  if (status === 'failed' || status === 'dead_letter') return 'badge-error'
  if (status === 'queued' || status === 'retrying') return 'badge-warning'
  return 'badge-info'
}

function formatTriggerEvent(triggerEvent?: string | null) {
  if (!triggerEvent || triggerEvent === 'manual') return '手动触发'
  if (triggerEvent === 'traffic.cluster.ready') return '流量聚类事件'
  if (triggerEvent === 'traffic.hypothesis.ready') return '风险假设事件'
  return triggerEvent
}
</script>
