<template>
  <div class="border border-base-300 rounded-lg overflow-hidden">
    <div class="px-4 py-3 bg-base-200 flex items-center justify-between">
      <div class="font-semibold text-sm">最近发现</div>
      <button class="btn btn-xs btn-ghost" @click="$emit('refresh')" :disabled="disabled">
        <i class="fas fa-rotate mr-1"></i>
        刷新
      </button>
    </div>
    <div v-if="windowedRecentFindings.length === 0" class="p-4 text-sm text-base-content/60">
      当前没有与该智能体关联的发现。
    </div>
    <div v-else class="divide-y divide-base-300 max-h-[320px] overflow-y-auto">
      <div v-for="finding in windowedRecentFindings" :key="finding.id" class="p-4">
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="font-medium text-sm break-words">{{ finding.title }}</div>
            <div class="text-xs text-base-content/60 mt-1 break-all">{{ finding.url || '-' }}</div>
          </div>
          <div class="flex flex-col items-end gap-1 shrink-0">
            <span class="badge badge-xs" :class="severityClass(finding.severity)">
              {{ finding.severity }}
            </span>
            <span class="badge badge-xs" :class="getSystemAgentFindingStageBadgeClass(finding)">
              {{ getSystemAgentFindingStageLabel(finding) }}
            </span>
          </div>
        </div>
        <div class="text-xs text-base-content/60 mt-2 flex flex-wrap gap-2">
          <span>{{ finding.vuln_type }}</span>
          <span>命中 {{ finding.hit_count }}</span>
          <span>{{ formatDate(finding.last_seen_at) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import type { SystemAgentFindingSummary } from '../systemAgentSettingsSupport'
import {
  getSystemAgentFindingStageBadgeClass,
  getSystemAgentFindingStageLabel,
} from './systemAgentFindingPresentation'

const props = defineProps<{
  findings: SystemAgentFindingSummary[]
  disabled?: boolean
  windowStart: number
}>()

defineEmits<{
  refresh: []
}>()

const windowedRecentFindings = computed(() => {
  return props.findings.filter(finding => {
    const seenAt = new Date(finding.last_seen_at).getTime()
    return Number.isFinite(seenAt) && seenAt >= props.windowStart
  })
})

function severityClass(severity: string) {
  if (severity === 'high' || severity === 'critical') return 'badge-error'
  if (severity === 'medium') return 'badge-warning'
  return 'badge-success'
}

function formatDate(value?: string | null) {
  if (!value) return '-'
  try {
    return new Date(value).toLocaleString()
  } catch {
    return value
  }
}
</script>
