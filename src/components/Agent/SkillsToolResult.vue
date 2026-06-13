<template>
  <div :class="['rounded-lg overflow-hidden border mb-2', containerClass]">
    <button
      type="button"
      class="flex w-full items-center gap-3 px-4 py-3 text-left transition-colors"
      :class="headerClass"
      :aria-expanded="isExpanded ? 'true' : 'false'"
      @click="toggleExpanded"
    >
      <i
        :class="[
          'fas text-xs transition-transform',
          isExpanded ? 'fa-chevron-down' : 'fa-chevron-right',
          accentTextClass,
        ]"
      ></i>
      <div
        :class="[
          'w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0 shadow-sm text-white',
          iconClass,
        ]"
      >
        <i :class="[iconName, 'text-sm']"></i>
      </div>
      <div class="flex-1 min-w-0">
        <div :class="['font-semibold text-sm', accentTextClass]">{{ title }}</div>
        <div class="text-xs text-base-content/70 truncate">
          {{ display.skillName }} ({{ display.skillId }})
        </div>
        <div v-if="display.description" class="text-xs text-base-content/60 mt-0.5 line-clamp-2">
          {{ display.description }}
        </div>
      </div>
      <span v-if="statusText" :class="['badge badge-sm whitespace-nowrap', statusClass]">
        {{ statusText }}
      </span>
    </button>

    <div v-show="isExpanded" class="px-4 py-3 space-y-3 border-t border-base-300/70 bg-base-100/60">
      <div v-if="status === 'running' || status === 'pending'" class="text-sm text-base-content/70">
        {{ runningText }}
      </div>

      <div
        v-if="displayStatus === 'failed'"
        class="rounded-lg border border-error/20 bg-error/10 px-3 py-2 text-sm text-error whitespace-pre-wrap break-words"
      >
        {{ errorText }}
      </div>

      <div v-if="referencedFiles.length > 0">
        <div class="text-xs font-medium text-base-content/70">
          {{ t('agent.skillHelperFilesLabel') }}
        </div>
        <ul class="mt-1 space-y-1">
          <li
            v-for="file in referencedFiles"
            :key="file"
            class="font-mono text-xs text-base-content/80 break-all"
          >
            {{ file }}
          </li>
        </ul>
      </div>

      <div v-if="warnings.length > 0">
        <div class="text-xs font-medium text-warning">
          {{ t('agent.skillHelperWarningsLabel') }}
        </div>
        <ul class="mt-1 space-y-1">
          <li
            v-for="(warning, index) in warnings"
            :key="`${index}-${warning}`"
            class="text-xs text-warning/90 break-words"
          >
            {{ warning }}
          </li>
        </ul>
      </div>

      <div v-if="resultSummary">
        <div class="text-xs font-medium text-base-content/70">
          {{ isFork ? t('agent.skillForkResultLabel') : t('agent.skillInvokeSummaryLabel') }}
        </div>
        <pre class="mt-1 whitespace-pre-wrap break-words font-sans text-sm text-base-content/80">{{ resultSummary }}</pre>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  compactSkillResultPreview,
  extractSkillsToolErrorMessage,
  isSkillsToolSuccess,
  parseSkillsToolArgs,
  parseSkillsToolResult,
  resolveSkillsDisplayFromMessage,
  type ParsedSkillsToolPayload,
} from './skillsToolSupport'

const props = withDefaults(
  defineProps<{
    args?: Record<string, unknown> | null
    result?: unknown
    error?: unknown
    status?: string
    referencedFiles?: string[]
    warnings?: string[]
  }>(),
  {
    args: null,
    result: undefined,
    error: undefined,
    status: 'pending',
    referencedFiles: () => [],
    warnings: () => [],
  },
)

const { t } = useI18n()
const isExpanded = ref(true)

const toggleExpanded = () => {
  isExpanded.value = !isExpanded.value
}

const parsedArgs = computed(() => parseSkillsToolArgs(props.args))
const parsedResult = computed(() => parseSkillsToolResult(props.result))

const display = computed<ParsedSkillsToolPayload>(() => {
  return (
    resolveSkillsDisplayFromMessage({
      toolArgs: props.args,
      toolResult: props.result,
    }) || {
      action: 'unknown',
      skillId: parsedArgs.value.skill || 'unknown',
      skillName: parsedArgs.value.skill || 'unknown',
      referencedFiles: [],
      warnings: [],
    }
  )
})

const isFork = computed(() => parsedResult.value?.action === 'fork')

const loadedDespiteFailure = computed(() => {
  const status = String(props.status || '').toLowerCase()
  return status === 'failed' && isSkillsToolSuccess(props.result)
})

const displayStatus = computed(() => {
  if (loadedDespiteFailure.value) {
    return 'completed'
  }
  return String(props.status || '').toLowerCase()
})

const referencedFiles = computed(() => {
  const fromResult = parsedResult.value?.referencedFiles || display.value.referencedFiles || []
  const fromMetadata = (props.referencedFiles || [])
    .filter((file) => typeof file === 'string' && file.trim().length > 0)
  return fromMetadata.length > 0 ? fromMetadata : fromResult
})

const warnings = computed(() => {
  const fromResult = parsedResult.value?.warnings || display.value.warnings || []
  const fromMetadata = (props.warnings || [])
    .filter((item) => typeof item === 'string' && item.trim().length > 0)
  return fromMetadata.length > 0 ? fromMetadata : fromResult
})

const title = computed(() => {
  if (displayStatus.value === 'failed') return t('agent.skillInvokeFailedTitle')
  if (isFork.value) return t('agent.skillForkedTitle')
  return t('agent.skillInvokedTitle')
})

const runningText = computed(() => {
  if (isFork.value) return t('agent.skillForkRunningText')
  return t('agent.skillInvokeRunningText', { skill: display.value.skillName })
})

const resultSummary = computed(() => {
  const content = parsedResult.value?.content
  if (!content || displayStatus.value === 'running' || displayStatus.value === 'pending') {
    return ''
  }
  return compactSkillResultPreview(content)
})

const errorText = computed(() => {
  if (loadedDespiteFailure.value) {
    return ''
  }
  const extracted = extractSkillsToolErrorMessage(props.error, props.result)
  if (extracted) return extracted
  if (typeof props.error === 'string') return props.error
  if (props.error) {
    try {
      return JSON.stringify(props.error)
    } catch {
      return String(props.error)
    }
  }
  return t('agent.skillInvokeFailedTitle')
})

const statusText = computed(() => {
  const status = displayStatus.value
  if (status === 'running' || status === 'pending') return t('agent.skillStatusRunning')
  if (status === 'failed') return t('agent.skillStatusFailed')
  if (status === 'completed' || status === 'success') return t('agent.skillStatusCompleted')
  return ''
})

const statusClass = computed(() => {
  const status = displayStatus.value
  if (status === 'failed') return 'badge-error'
  if (status === 'running' || status === 'pending') return 'badge-warning'
  return 'badge-success'
})

const containerClass = computed(() => {
  if (displayStatus.value === 'failed') return 'border-error/30 bg-error/5'
  if (isFork.value) return 'border-secondary/30 bg-secondary/5'
  return 'border-success/30 bg-success/5'
})

const headerClass = computed(() => {
  if (displayStatus.value === 'failed') return 'bg-error/10 hover:bg-error/15'
  if (isFork.value) return 'bg-secondary/10 hover:bg-secondary/15'
  return 'bg-success/10 hover:bg-success/15'
})

const accentTextClass = computed(() => {
  if (displayStatus.value === 'failed') return 'text-error'
  if (isFork.value) return 'text-secondary'
  return 'text-success'
})

const iconClass = computed(() => {
  if (displayStatus.value === 'failed') return 'bg-error'
  if (isFork.value) return 'bg-secondary'
  return 'bg-success'
})

const iconName = computed(() => {
  if (displayStatus.value === 'failed') return 'fas fa-exclamation'
  if (isFork.value) return 'fas fa-code-branch'
  return 'fas fa-book-open'
})
</script>
