<template>
  <div class="rounded-lg overflow-hidden bg-base-200">
    <button
      type="button"
      class="flex w-full items-start gap-2.5 px-4 py-2.5 text-left transition-colors hover:bg-base-300/50"
      :aria-expanded="isExpanded ? 'true' : 'false'"
      @click="isExpanded = !isExpanded"
    >
      <i
        :class="[iconName, iconClass]"
        class="mt-0.5 w-4 flex-shrink-0 text-sm transition-colors"
      ></i>
      <div class="min-w-0 flex-1">
        <div class="truncate text-sm font-medium text-base-content/85">{{ title }}</div>
        <div v-if="subtitle" class="mt-0.5 truncate text-[11px] text-base-content/60">{{ subtitle }}</div>
        <div v-if="subtitle && summaryBadges.length > 0" class="mt-1.5 flex flex-wrap gap-1.5">
          <span
            v-for="badge in summaryBadges"
            :key="badge"
            class="rounded-full bg-base-200 px-2 py-0.5 text-[10px] text-base-content/70"
          >
            {{ badge }}
          </span>
        </div>
      </div>
      <div class="ml-auto flex flex-shrink-0 items-center gap-2 pl-2">
        <span :class="['rounded-full px-2 py-0.5 text-[10px] font-medium', statusClass]">
          {{ statusText }}
        </span>
        <span v-if="durationText" class="text-[10px] text-base-content/50">{{ durationText }}</span>
      </div>
    </button>

    <div v-show="isExpanded" class="border-t border-base-300 bg-base-200 px-4 py-3">
      <div class="space-y-4">
        <div class="flex flex-wrap gap-2 text-[10px] text-base-content/70">
          <span class="rounded-full bg-base-200 px-2 py-0.5">
            <span class="font-medium">{{ t('agent.toolCardPattern') }}:</span>
            <span class="ml-1 font-mono text-[10px] leading-4 text-base-content/70">{{ patternText }}</span>
          </span>
          <span v-if="basePathText" class="rounded-full bg-base-200 px-2 py-0.5">
            <span class="font-medium">{{ t('agent.toolCardBasePath') }}:</span>
            <span class="ml-1 font-mono text-[10px] leading-4 text-base-content/70">{{ basePathText }}</span>
          </span>
          <span
            v-if="isTruncated"
            class="rounded-full bg-warning/15 px-2 py-0.5 text-warning"
          >
            {{ t('agent.toolCardTruncated') }}
          </span>
        </div>

        <div
          v-if="matchRows.length > 0"
          class="overflow-hidden rounded-lg border border-base-300 bg-base-200/40"
        >
          <div class="border-b border-base-300 px-3 py-2 text-xs font-medium text-base-content/70">
            {{ t('agent.toolCardMatches') }}
          </div>
          <div class="max-h-80 overflow-auto px-3 py-2 text-xs">
            <div
              v-for="match in matchRows"
              :key="`${match.filePath}:${match.lineNumber}:${match.line}`"
              class="border-b border-base-300/60 py-2 last:border-b-0"
            >
              <div class="flex flex-wrap items-center gap-2 text-[10px] text-base-content/65">
                <span class="font-mono text-[10px] leading-4 text-base-content/70">{{ match.filePath }}</span>
                <span class="rounded-full bg-base-300/80 px-2 py-0.5">{{ t('agent.toolCardLineNumber') }} {{ match.lineNumber }}</span>
              </div>
              <code class="mt-2 block whitespace-pre-wrap break-all rounded bg-base-300/80 px-2 py-2 text-base-content/80">{{ match.line }}</code>
            </div>
          </div>
        </div>

        <div
          v-else-if="fileRows.length > 0"
          class="overflow-hidden rounded-lg border border-base-300 bg-base-200/40"
        >
          <div class="border-b border-base-300 px-3 py-2 text-xs font-medium text-base-content/70">
            {{ t('agent.toolCardFiles') }}
          </div>
          <div class="max-h-80 overflow-auto px-3 py-2 text-xs">
            <div
              v-for="file in fileRows"
              :key="file"
              class="border-b border-base-300/60 py-1.5 font-mono text-base-content/80 last:border-b-0"
            >
              {{ file }}
            </div>
          </div>
        </div>

        <div
          v-else
          class="rounded-lg border border-base-300 bg-base-200/40 px-3 py-3 text-xs text-base-content/65"
        >
          {{ t('agent.toolCardNoStructuredOutput') }}
        </div>

        <ToolRuntimeMeta :result="parsedResult" />

        <details v-if="showRawPayload" class="rounded-lg border border-base-300 bg-base-200/40">
          <summary class="cursor-pointer px-3 py-2 text-xs font-medium text-base-content/70">
            {{ t('agent.toolCardViewDetails') }}
          </summary>
          <div class="grid gap-3 border-t border-base-300 px-3 py-3 lg:grid-cols-2">
            <div>
              <div class="mb-2 text-xs font-medium text-base-content/65">{{ t('agent.inputParameters') }}</div>
              <pre class="max-h-72 overflow-auto whitespace-pre-wrap break-all rounded bg-base-300/80 px-3 py-2 text-xs text-base-content/80">{{ formattedArgs }}</pre>
            </div>
            <div>
              <div class="mb-2 text-xs font-medium text-base-content/65">{{ t('agent.toolCardRawPayload') }}</div>
              <pre class="max-h-72 overflow-auto whitespace-pre-wrap break-all rounded bg-base-300/80 px-3 py-2 text-xs text-base-content/80">{{ formattedRawResult }}</pre>
            </div>
          </div>
        </details>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import type { AgentMessage, MessageMetadata } from '@/types/agent'
import { formatJsonValueIfPossible } from '@/utils/jsonFormatting'
import ToolRuntimeMeta from './ToolRuntimeMeta.vue'
import { parseStructuredToolPayload } from './toolRenderSupport'

interface GrepMatchRow {
  filePath: string
  lineNumber: number
  line: string
}

const props = defineProps<{
  message: AgentMessage
}>()

const { t } = useI18n()
const isExpanded = ref(false)

const metadata = computed(() => props.message.metadata as MessageMetadata | undefined)
const toolName = computed(() => String(metadata.value?.tool_name || '').trim().toLowerCase())
const status = computed(() => metadata.value?.status || 'completed')
const parsedArgs = computed<Record<string, any>>(() => {
  const parsed = parseStructuredToolPayload(metadata.value?.tool_args)
  return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
    ? parsed as Record<string, any>
    : {}
})
const parsedResult = computed(() => parseStructuredToolPayload(metadata.value?.tool_result))
const resultRecord = computed<Record<string, any> | null>(() => {
  const parsed = parsedResult.value
  return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
    ? parsed as Record<string, any>
    : null
})

const isGlob = computed(() => toolName.value === 'glob')
const iconName = computed(() => (isGlob.value ? 'fas fa-folder-tree' : 'fas fa-magnifying-glass'))
const iconClass = computed(() => {
  if (status.value === 'failed') return 'text-error'
  if (status.value === 'running') return 'text-warning'
  return 'text-primary/80'
})

const patternText = computed(() => {
  const pattern = resultRecord.value?.pattern ?? parsedArgs.value.pattern
  return typeof pattern === 'string' && pattern.trim().length > 0 ? pattern.trim() : '*'
})

const basePathText = computed(() => {
  const base = resultRecord.value?.base_path ?? parsedArgs.value.path
  return typeof base === 'string' ? base : ''
})

const matchRows = computed<GrepMatchRow[]>(() => {
  if (toolName.value !== 'grep') return []
  const rows = Array.isArray(resultRecord.value?.content) ? resultRecord.value?.content : []
  return rows
    .map((item) => {
      if (!item || typeof item !== 'object') return null
      const record = item as Record<string, unknown>
      const filePath = typeof record.file_path === 'string' ? record.file_path : ''
      const lineNumber = Number(record.line_number || 0)
      const line = typeof record.line === 'string' ? record.line : ''
      if (!filePath || lineNumber <= 0) return null
      return { filePath, lineNumber, line }
    })
    .filter((item): item is GrepMatchRow => item !== null)
})

const fileRows = computed<string[]>(() => {
  const rows = Array.isArray(resultRecord.value?.filenames) ? resultRecord.value?.filenames : []
  return rows.filter((item): item is string => typeof item === 'string' && item.trim().length > 0)
})

const matchCount = computed(() => Number(resultRecord.value?.num_matches || 0))
const fileCount = computed(() => Number(resultRecord.value?.num_files || fileRows.value.length || 0))

const title = computed(() => {
  return isGlob.value ? t('agent.toolCardGlobTitle') : t('agent.toolCardGrepTitle')
})

const subtitle = computed(() => {
  if (isGlob.value) {
    if (fileCount.value > 0) {
      return `${t('agent.toolCardFiles')} ${fileCount.value}`
    }
    return ''
  }

  const outputMode = String(resultRecord.value?.output_mode || '').trim()
  if (outputMode === 'files_with_matches') {
    return `${t('agent.toolCardMatchedFiles')} ${fileRows.value.length}`
  }
  if (outputMode === 'count') {
    return `${t('agent.toolCardMatches')} ${matchCount.value}`
  }
  if (matchCount.value > 0) {
    return `${t('agent.toolCardMatches')} ${matchCount.value}`
  }
  return ''
})

const summaryBadges = computed(() => {
  const badges: string[] = []
  if (!isGlob.value) {
    const appliedLimit = Number(resultRecord.value?.applied_limit || 0)
    if (appliedLimit > 0) {
      badges.push(`${t('agent.toolCardLimit')} ${appliedLimit}`)
    }
    const globPattern = String(parsedArgs.value.glob || '').trim()
    if (globPattern) {
      badges.push(`glob ${globPattern}`)
    }
  }
  return badges
})

const isTruncated = computed(() => resultRecord.value?.truncated === true)

const statusText = computed(() => {
  switch (status.value) {
    case 'running':
      return t('agent.statusRunning')
    case 'failed':
      return t('agent.statusFailed')
    case 'pending':
      return t('agent.statusPending')
    default:
      return t('agent.statusCompleted')
  }
})

const statusClass = computed(() => {
  switch (status.value) {
    case 'running':
      return 'bg-warning/15 text-warning'
    case 'failed':
      return 'bg-error/15 text-error'
    case 'pending':
      return 'bg-base-300 text-base-content/65'
    default:
      return 'bg-success/15 text-success'
  }
})

const durationText = computed(() => {
  const ms = Number(metadata.value?.duration_ms || 0)
  return ms > 0 ? `${(ms / 1000).toFixed(1)}s` : ''
})

const formattedArgs = computed(() => {
  return formatJsonValueIfPossible(parsedArgs.value) || '{}'
})

const formattedRawResult = computed(() => {
  if (typeof parsedResult.value === 'string') return parsedResult.value || '{}'
  return formatJsonValueIfPossible(parsedResult.value) || '{}'
})

const showRawPayload = computed(() => {
  return Object.keys(parsedArgs.value).length > 0 || !!formattedRawResult.value.trim()
})
</script>
