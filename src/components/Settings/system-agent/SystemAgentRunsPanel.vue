<template>
  <div class="border border-base-300 rounded-lg overflow-hidden">
    <div class="px-4 py-3 bg-base-200 flex items-center justify-between">
      <div class="font-semibold text-sm">运行记录</div>
      <button class="btn btn-xs btn-ghost" @click="$emit('refresh')" :disabled="disabled">
        <i class="fas fa-rotate mr-1"></i>
        刷新
      </button>
    </div>
    <div v-if="runs.length === 0" class="p-4 text-sm text-base-content/60">当前没有运行记录。</div>
    <div v-else class="divide-y divide-base-300 max-h-[420px] overflow-y-auto">
      <div v-for="run in runs" :key="run.id" class="p-4">
        <div class="flex items-center justify-between gap-3">
          <div class="text-sm font-medium">
            {{ formatRunStatus(run.status) }}
          </div>
          <span class="badge badge-sm" :class="statusBadgeClass(run.status)">
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
        <div
          v-if="getContextExtractionSummary(run)"
          class="mt-3 rounded-lg border border-base-300 bg-base-200/40 p-3 space-y-3"
        >
          <div class="flex flex-wrap items-center gap-2">
            <div class="text-xs font-semibold">上下文抽取命中</div>
            <span class="badge badge-info badge-xs">
              动作 {{ getContextExtractionSummary(run)?.actionKind }}
            </span>
            <span class="badge badge-outline badge-xs">
              {{ getContextExtractionSummary(run)?.actionSource }}
            </span>
            <span
              v-if="getContextExtractionSummary(run)?.matchedAlias"
              class="badge badge-ghost badge-xs font-mono"
            >
              alias {{ getContextExtractionSummary(run)?.matchedAlias }}
            </span>
          </div>

          <div v-if="getContextExtractionSummary(run)?.principalMatches.length" class="space-y-1">
            <div class="text-[11px] font-medium text-base-content/70">主体字段</div>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="entry in getContextExtractionSummary(run)?.principalMatches"
                :key="`principal-${entry.configuredKey}-${entry.matchedKey}-${entry.source}`"
                class="badge badge-outline badge-sm"
              >
                {{ formatExtractionEntry(entry) }}
              </span>
            </div>
          </div>

          <div v-if="getContextExtractionSummary(run)?.resourceMatches.length" class="space-y-1">
            <div class="text-[11px] font-medium text-base-content/70">资源字段</div>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="entry in getContextExtractionSummary(run)?.resourceMatches"
                :key="`resource-${entry.configuredKey}-${entry.matchedKey}-${entry.source}`"
                class="badge badge-outline badge-sm"
              >
                {{ formatExtractionEntry(entry) }}
              </span>
            </div>
          </div>

          <div v-if="getContextExtractionSummary(run)?.authMatches.length" class="space-y-1">
            <div class="text-[11px] font-medium text-base-content/70">认证线索</div>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="entry in getContextExtractionSummary(run)?.authMatches"
                :key="`auth-${entry.configuredKey}-${entry.matchedKey}-${entry.source}`"
                class="badge badge-outline badge-sm"
              >
                {{ formatExtractionEntry(entry) }}
              </span>
            </div>
          </div>
        </div>
        <div
          v-if="getSemanticAbstractionSummary(run)"
          class="mt-3 rounded-lg border border-base-300 bg-base-200/40 p-3 space-y-3"
        >
          <div class="flex flex-wrap items-center gap-2">
            <div class="text-xs font-semibold">语义抽象</div>
            <span class="badge badge-secondary badge-xs">
              {{ formatSemanticSource(getSemanticAbstractionSummary(run)?.source) }}
            </span>
            <span
              v-if="getSemanticAbstractionSummary(run)?.signature"
              class="badge badge-ghost badge-xs font-mono"
            >
              {{ getSemanticAbstractionSummary(run)?.signature }}
            </span>
          </div>

          <div v-if="getSemanticAbstractionSummary(run)?.actionCandidates.length" class="space-y-1">
            <div class="text-[11px] font-medium text-base-content/70">动作语义</div>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="entry in getSemanticAbstractionSummary(run)?.actionCandidates"
                :key="`action-${entry.kind}-${entry.source}-${entry.confidence}`"
                class="badge badge-sm"
                :class="semanticConfidenceBadgeClass(entry.confidence)"
                :title="entry.reason"
              >
                {{ formatSemanticActionEntry(entry) }}
              </span>
            </div>
          </div>

          <div
            v-if="getSemanticAbstractionSummary(run)?.principalCandidates.length"
            class="space-y-1"
          >
            <div class="text-[11px] font-medium text-base-content/70">主体候选</div>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="entry in getSemanticAbstractionSummary(run)?.principalCandidates"
                :key="`semantic-principal-${entry.field}-${entry.source}-${entry.confidence}`"
                class="badge badge-sm"
                :class="semanticConfidenceBadgeClass(entry.confidence)"
                :title="entry.reason"
              >
                {{ formatSemanticCandidateEntry(entry) }}
              </span>
            </div>
          </div>

          <div
            v-if="getSemanticAbstractionSummary(run)?.resourceCandidates.length"
            class="space-y-1"
          >
            <div class="text-[11px] font-medium text-base-content/70">资源候选</div>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="entry in getSemanticAbstractionSummary(run)?.resourceCandidates"
                :key="`semantic-resource-${entry.field}-${entry.source}-${entry.confidence}`"
                class="badge badge-sm"
                :class="semanticConfidenceBadgeClass(entry.confidence)"
                :title="entry.reason"
              >
                {{ formatSemanticCandidateEntry(entry) }}
              </span>
            </div>
          </div>

          <div
            v-if="getSemanticAbstractionSummary(run)?.credentialCandidates.length"
            class="space-y-1"
          >
            <div class="text-[11px] font-medium text-base-content/70">凭证候选</div>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="entry in getSemanticAbstractionSummary(run)?.credentialCandidates"
                :key="`semantic-credential-${entry.field}-${entry.source}-${entry.confidence}`"
                class="badge badge-sm"
                :class="semanticConfidenceBadgeClass(entry.confidence)"
                :title="entry.reason"
              >
                {{ formatSemanticCandidateEntry(entry) }}
              </span>
            </div>
          </div>

          <div v-if="getSemanticAbstractionSummary(run)?.stateCandidates.length" class="space-y-1">
            <div class="text-[11px] font-medium text-base-content/70">状态候选</div>
            <div class="flex flex-wrap gap-2">
              <span
                v-for="entry in getSemanticAbstractionSummary(run)?.stateCandidates"
                :key="`semantic-state-${entry.field}-${entry.source}-${entry.confidence}`"
                class="badge badge-sm"
                :class="semanticConfidenceBadgeClass(entry.confidence)"
                :title="entry.reason"
              >
                {{ formatSemanticCandidateEntry(entry) }}
              </span>
            </div>
          </div>

          <div v-if="getSemanticAbstractionSummary(run)?.notes.length" class="space-y-1">
            <div class="text-[11px] font-medium text-base-content/70">抽象备注</div>
            <ul class="list-disc list-inside text-[11px] text-base-content/70 space-y-1">
              <li v-for="note in getSemanticAbstractionSummary(run)?.notes" :key="note">
                {{ note }}
              </li>
            </ul>
          </div>
        </div>
        <pre v-if="run.output" class="mt-3 p-3 bg-base-200 rounded-lg text-xs overflow-auto">{{
          JSON.stringify(run.output, null, 2)
        }}</pre>
        <div
          v-else-if="run.errorMessage"
          class="mt-3 p-3 rounded-lg bg-error/10 text-error text-xs whitespace-pre-wrap"
        >
          {{ run.errorMessage }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { SystemAgentRunPayload } from '../systemAgentSettingsSupport'
import {
  formatContextExtractionEntry,
  formatSemanticActionEntry,
  formatSemanticCandidateEntry,
  getContextExtractionSummaryFromPayload,
  getSemanticAbstractionSummaryFromPayload,
  semanticConfidenceBadgeClass,
} from '../../system-agent/systemAgentTrafficSummaries'

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

function getContextExtractionSummary(run: SystemAgentRunPayload) {
  return getContextExtractionSummaryFromPayload(run.inputSummary)
}

function getSemanticAbstractionSummary(run: SystemAgentRunPayload) {
  return getSemanticAbstractionSummaryFromPayload(run.inputSummary)
}

const formatExtractionEntry = formatContextExtractionEntry

function formatSemanticSource(source?: string | null) {
  if (source === 'ai_augmented') return 'AI 增强'
  if (source === 'fallback') return '确定性回退'
  return source || 'unknown'
}
</script>
