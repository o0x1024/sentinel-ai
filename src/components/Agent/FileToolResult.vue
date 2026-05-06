<template>
  <div
    class="rounded-lg overflow-hidden bg-base-200"
    :class="panelBorderClass"
  >
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
        <span
          v-if="fileVerificationText"
          :class="['rounded-full px-2 py-0.5 text-[10px] font-medium', fileVerificationClass]"
        >
          {{ fileVerificationText }}
        </span>
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
            <span class="font-medium">{{ t('agent.toolCardPath') }}:</span>
            <span class="ml-1 font-mono text-[10px] leading-4 text-base-content/70">{{ filePath }}</span>
          </span>
          <span v-if="rangeBadge" class="rounded-full bg-base-200 px-2 py-0.5">{{ rangeBadge }}</span>
          <span v-if="bytesBadge" class="rounded-full bg-base-200 px-2 py-0.5">{{ bytesBadge }}</span>
          <span v-if="replacementsBadge" class="rounded-full bg-base-200 px-2 py-0.5">{{ replacementsBadge }}</span>
          <span
            v-if="isTruncated"
            class="rounded-full bg-warning/15 px-2 py-0.5 text-warning"
          >
            {{ t('agent.toolCardTruncated') }}
          </span>
        </div>

        <div
          v-if="isFileRead && fileReadLines.length > 0"
          class="overflow-hidden rounded-lg border border-base-300 bg-base-200/40"
        >
          <div class="border-b border-base-300 px-3 py-2 text-xs font-medium text-base-content/70">
            {{ readSectionLabel }}
          </div>
          <div class="max-h-80 overflow-auto px-3 py-2 font-mono text-xs">
            <div
              v-for="line in fileReadLines"
              :key="line.number"
              class="grid grid-cols-[auto,1fr] gap-3 border-b border-base-300/60 py-1 last:border-b-0"
            >
              <span class="select-none text-right text-base-content/45">{{ line.number }}</span>
              <code class="whitespace-pre-wrap break-all text-base-content/80">{{ line.text || ' ' }}</code>
            </div>
          </div>
        </div>

        <div
          v-else-if="previewSections.length > 0"
          class="grid gap-3 lg:grid-cols-2"
        >
          <div
            v-for="section in previewSections"
            :key="section.label"
            class="overflow-hidden rounded-lg border border-base-300 bg-base-200/40"
          >
            <div class="border-b border-base-300 px-3 py-2 text-xs font-medium text-base-content/70">
              {{ section.label }}
            </div>
            <pre class="max-h-72 overflow-auto whitespace-pre-wrap break-all px-3 py-2 text-xs text-base-content/80">{{ section.content }}</pre>
          </div>
        </div>

        <div
          v-if="changeItems.length > 0"
          class="rounded-lg border border-base-300 bg-base-200/40 px-3 py-3"
        >
          <div class="mb-2 text-xs font-medium text-base-content/70">{{ t('agent.toolCardChanges') }}</div>
          <div class="flex flex-wrap gap-2 text-[11px] text-base-content/80">
            <span
              v-for="item in changeItems"
              :key="item.label"
              class="rounded-full bg-base-300/80 px-2 py-0.5"
            >
              <span class="font-medium">{{ item.label }}:</span>
              <span class="ml-1">{{ item.value }}</span>
            </span>
          </div>
        </div>

        <StoredArtifactPanel
          v-if="storedArtifactViews.length > 0"
          :artifacts="storedArtifactViews"
        />

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
import StoredArtifactPanel from './StoredArtifactPanel.vue'
import { buildStoredArtifactViews } from './storedArtifactSupport'
import ToolRuntimeMeta from './ToolRuntimeMeta.vue'
import { parseStructuredToolPayload } from './toolRenderSupport'

interface PreviewSection {
  label: string
  content: string
}

interface ChangeItem {
  label: string
  value: string
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

const isFileRead = computed(() => toolName.value === 'file_read')
const isFileWrite = computed(() => toolName.value === 'file_write')
const isFileEdit = computed(() => toolName.value === 'file_edit')

const filePath = computed(() => {
  const path = resultRecord.value?.file_path ?? parsedArgs.value.file_path
  return typeof path === 'string' && path.trim().length > 0 ? path.trim() : 'unknown'
})

const title = computed(() => {
  if (isFileRead.value) return `${t('agent.toolCardFileReadTitle')} ${filePath.value}`
  if (isFileWrite.value) return `${t('agent.toolCardFileWriteTitle')} ${filePath.value}`
  return `${t('agent.toolCardFileEditTitle')} ${filePath.value}`
})

const subtitle = computed(() => {
  if (isFileRead.value) {
    const start = Number(resultRecord.value?.start_line || parsedArgs.value.offset || 1)
    const end = Number(resultRecord.value?.end_line || 0)
    const total = Number(resultRecord.value?.total_lines || 0)
    if (start > 0 && end > 0 && total > 0) {
      return `${t('agent.toolCardLineRange')} ${start}-${end} / ${total}`
    }
    const limit = Number(parsedArgs.value.limit || 0)
    return limit > 0
      ? `${t('agent.toolCardLineRange')} ${start}-${start + Math.max(limit - 1, 0)}`
      : ''
  }

  if (isFileWrite.value) {
    const operation = String(resultRecord.value?.operation || '').trim()
    const actionLabel =
      operation === 'create'
        ? t('agent.toolCardOperationCreate')
        : operation === 'overwrite'
          ? t('agent.toolCardOperationOverwrite')
          : ''
    const preview = String(resultRecord.value?.content_preview || '').trim()
    if (preview && actionLabel) return `${actionLabel} · ${preview}`
    return preview || actionLabel
  }

  const preview = String(resultRecord.value?.after_preview || '').trim()
  const replacements = Number(resultRecord.value?.replacements || 0)
  if (preview) {
    return replacements > 0 ? `${replacements} ${t('agent.toolCardReplacements')} · ${preview}` : preview
  }
  return replacements > 0 ? `${replacements} ${t('agent.toolCardReplacements')}` : ''
})

const summaryBadges = computed(() => {
  const badges: string[] = []
  if (isFileWrite.value) {
    const operation = String(resultRecord.value?.operation || '').trim()
    if (operation === 'create') badges.push(t('agent.toolCardOperationCreate'))
    if (operation === 'overwrite') badges.push(t('agent.toolCardOperationOverwrite'))
  }
  const changed = Number(resultRecord.value?.change_summary?.changed_line_count || 0)
  if (changed > 0) {
    badges.push(`${changed} ${t('agent.toolCardChangedLines')}`)
  }
  const total = Number(resultRecord.value?.total_lines || 0)
  if (isFileRead.value && total > 0) {
    badges.push(`${total} ${t('agent.toolCardTotalLines')}`)
  }
  return badges
})

const rangeBadge = computed(() => {
  if (isFileRead.value) {
    const start = Number(resultRecord.value?.start_line || 0)
    const end = Number(resultRecord.value?.end_line || 0)
    if (start > 0 && end > 0) {
      return `${t('agent.toolCardLineRange')} ${start}-${end}`
    }
  }
  return ''
})

const bytesBadge = computed(() => {
  const bytes = Number(resultRecord.value?.bytes_written || 0)
  return bytes > 0 ? `${t('agent.toolCardBytes')} ${bytes}` : ''
})

const replacementsBadge = computed(() => {
  const replacements = Number(resultRecord.value?.replacements || 0)
  return replacements > 0 ? `${t('agent.toolCardReplacements')} ${replacements}` : ''
})

const isTruncated = computed(() => resultRecord.value?.truncated === true)

const readSectionLabel = computed(() => {
  const start = Number(resultRecord.value?.start_line || 0)
  const end = Number(resultRecord.value?.end_line || 0)
  const total = Number(resultRecord.value?.total_lines || 0)
  if (start > 0 && end > 0 && total > 0) {
    return `${t('agent.toolCardLineRange')} ${start}-${end} / ${total}`
  }
  return t('agent.executionResult')
})

const fileReadLines = computed(() => {
  if (!isFileRead.value) return []
  const content = typeof resultRecord.value?.content === 'string' ? resultRecord.value.content : ''
  if (!content) return []
  const start = Number(resultRecord.value?.start_line || parsedArgs.value.offset || 1)
  return content.split('\n').map((text, index) => ({
    number: start + index,
    text,
  }))
})

const previewSections = computed<PreviewSection[]>(() => {
  const sections: PreviewSection[] = []
  const beforePreview = String(
    resultRecord.value?.before_preview ??
    resultRecord.value?.previous_preview ??
    resultRecord.value?.change_summary?.before_preview ??
    '',
  ).trim()
  const afterPreview = String(
    resultRecord.value?.after_preview ??
    resultRecord.value?.content_preview ??
    resultRecord.value?.change_summary?.after_preview ??
    '',
  ).trim()

  if (beforePreview) {
    sections.push({ label: t('agent.toolCardBefore'), content: beforePreview })
  }
  if (afterPreview) {
    sections.push({ label: t('agent.toolCardAfter'), content: afterPreview })
  }
  return sections
})

const changeItems = computed<ChangeItem[]>(() => {
  const summary = resultRecord.value?.change_summary
  if (!summary || typeof summary !== 'object') return []

  const items: ChangeItem[] = []
  const firstChangedLine = Number(summary.first_changed_line || 0)
  const changedLineCount = Number(summary.changed_line_count || 0)
  const addedLineCount = Number(summary.added_line_count || 0)
  const removedLineCount = Number(summary.removed_line_count || 0)

  if (firstChangedLine > 0) {
    items.push({ label: t('agent.toolCardFirstChangedLine'), value: String(firstChangedLine) })
  }
  if (changedLineCount > 0) {
    items.push({ label: t('agent.toolCardChangedLines'), value: String(changedLineCount) })
  }
  if (addedLineCount > 0) {
    items.push({ label: t('agent.toolCardAddedLines'), value: String(addedLineCount) })
  }
  if (removedLineCount > 0) {
    items.push({ label: t('agent.toolCardRemovedLines'), value: String(removedLineCount) })
  }

  return items
})

const storedArtifactViews = computed(() =>
  buildStoredArtifactViews(metadata.value?.tool_result, metadata.value?.tracked_artifacts)
)

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

const iconClass = computed(() => {
  if (status.value === 'failed') return 'text-error'
  if (status.value === 'running') return 'text-warning'
  return 'text-primary/80'
})

const iconName = computed(() => {
  if (isFileRead.value) return 'fas fa-file-lines'
  return 'fas fa-file-pen'
})

const panelBorderClass = computed(() => {
  if (status.value === 'failed') return 'border-error/50'
  if (isFileRead.value) return 'border-info/40'
  if (isFileWrite.value) return 'border-success/40'
  return 'border-warning/40'
})

const durationText = computed(() => {
  const ms = Number(metadata.value?.duration_ms || 0)
  return ms > 0 ? `${(ms / 1000).toFixed(1)}s` : ''
})

const fileVerificationStatus = computed(() => metadata.value?.file_verification_status || '')
const fileVerificationText = computed(() => {
  switch (fileVerificationStatus.value) {
    case 'verified':
      return t('agent.toolCardReadbackVerified')
    case 'pending':
      return t('agent.toolCardReadbackPending')
    case 'failed':
      return t('agent.toolCardWriteFailed')
    default:
      return ''
  }
})

const fileVerificationClass = computed(() => {
  switch (fileVerificationStatus.value) {
    case 'verified':
      return 'bg-success/15 text-success'
    case 'pending':
      return 'bg-warning/15 text-warning'
    case 'failed':
      return 'bg-error/15 text-error'
    default:
      return 'bg-base-300 text-base-content/65'
  }
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
