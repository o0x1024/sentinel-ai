<template>
  <div
    class="tool-call-panel rounded-lg overflow-hidden bg-base-200 border-l-4"
    :class="toolPanelBorderClass"
  >
    <div
      @click="toggleToolPanel"
      @keydown.enter.prevent="toggleToolPanel"
      @keydown.space.prevent="toggleToolPanel"
      tabindex="0"
      role="button"
      :aria-expanded="isToolPanelExpanded ? 'true' : 'false'"
      class="tool-panel-header flex min-w-0 items-center gap-2 px-4 py-3 cursor-pointer hover:bg-base-300/50 transition-colors"
    >
      <i
        :class="[
          'fas shrink-0 transition-transform text-xs',
          isToolPanelExpanded ? 'fa-chevron-down' : 'fa-chevron-right',
        ]"
      ></i>

      <span class="shrink-0 font-mono text-sm font-semibold">{{ toolName || 'Tool' }}</span>

      <span
        v-if="toolHeaderDetail"
        class="min-w-0 flex-1 truncate text-xs text-base-content/45"
        :title="toolHeaderDetail"
      >
        {{ toolHeaderDetail }}
      </span>

      <span
        v-if="fileVerificationStatus"
        :class="['px-2 py-0.5 rounded-full text-xs font-medium', fileVerificationClass]"
      >
        {{ fileVerificationText }}
      </span>

      <span
        v-if="toolStatus"
        :class="[
          'status-badge px-2 py-0.5 rounded-full text-xs font-medium ml-auto',
          toolStatusClass,
        ]"
      >
        {{ toolStatusText }}
      </span>

      <span v-if="duration" class="text-xs text-base-content/60">{{ duration }}</span>
    </div>

    <div v-show="isToolPanelExpanded" class="tool-panel-content">
      <div v-if="hasToolArgs" class="border-t border-base-300">
        <div
          ref="argsBodyRef"
          @click="toggleArgs"
          @keydown.enter.prevent="toggleArgs"
          @keydown.space.prevent="toggleArgs"
          tabindex="0"
          role="button"
          :aria-expanded="isArgsExpanded ? 'true' : 'false'"
          :class="[
            'px-4 py-3 bg-base-100 cursor-pointer transition-all relative',
            isArgsExpanded ? 'max-h-96 overflow-y-auto' : 'max-h-24 overflow-hidden',
          ]"
        >
          <div class="text-xs text-base-content/50 mb-2">📥 {{ t('agent.inputParameters') }}</div>
          <pre class="text-xs font-mono text-base-content/70 whitespace-pre-wrap break-words overflow-x-auto">{{ formattedArgs }}</pre>

          <div
            v-if="!isArgsExpanded && argsHasOverflow"
            class="expand-hint absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-base-100 to-transparent flex items-end justify-center pb-1 pointer-events-none"
          >
            <span class="text-base-content/50 text-xs">点击展开</span>
          </div>
        </div>
      </div>

      <div v-if="hasToolResult" class="border-t border-base-300">
        <div
          ref="resultBodyRef"
          @click="toggleResult"
          @keydown.enter.prevent="toggleResult"
          @keydown.space.prevent="toggleResult"
          tabindex="0"
          role="button"
          :aria-expanded="isResultExpanded ? 'true' : 'false'"
          :class="[
            'px-4 py-3 bg-base-100 cursor-pointer transition-all relative',
            isResultExpanded ? 'max-h-96 overflow-y-auto' : 'max-h-24 overflow-hidden',
          ]"
        >
          <div class="text-xs text-base-content/50 mb-2">📤 {{ t('agent.executionResult') }}</div>
          <pre class="text-xs font-mono text-base-content/70 whitespace-pre-wrap break-words overflow-x-auto">{{ formattedToolResult }}</pre>

          <div
            v-if="!isResultExpanded && resultHasOverflow"
            class="expand-hint absolute bottom-0 left-0 right-0 h-8 bg-gradient-to-t from-base-100 to-transparent flex items-end justify-center pb-1 pointer-events-none"
          >
            <span class="text-base-content/50 text-xs">点击展开</span>
          </div>
        </div>
        <ToolRuntimeMeta :result="message.metadata?.tool_result" class="mx-4 mb-3" />
        <StoredArtifactPanel
          v-if="toolResultStoredArtifactViews.length > 0"
          :artifacts="toolResultStoredArtifactViews"
        />
      </div>

      <div
        v-if="message.metadata?.tool_call_id"
        class="px-4 py-2 border-t border-base-300 bg-base-100"
      >
        <span class="text-xs text-base-content/50">
          {{ t('agent.toolCallId') }}:
          <code class="font-mono">{{ message.metadata.tool_call_id }}</code>
        </span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AgentMessage } from '@/types/agent'
import { formatJsonValueIfPossible } from '@/utils/jsonFormatting'
import StoredArtifactPanel from './StoredArtifactPanel.vue'
import ToolRuntimeMeta from './ToolRuntimeMeta.vue'
import { buildStoredArtifactViews } from './storedArtifactSupport'
import { getToolHeaderDetail } from './toolRenderSupport'

const props = defineProps<{
  message: AgentMessage
}>()

const { t } = useI18n()

const isToolPanelExpanded = ref(false)
const isArgsExpanded = ref(false)
const isResultExpanded = ref(false)
const argsHasOverflow = ref(false)
const resultHasOverflow = ref(false)
const argsBodyRef = ref<HTMLElement | null>(null)
const resultBodyRef = ref<HTMLElement | null>(null)

const PREVIEW_MAX_DEPTH = 6
const PREVIEW_MAX_ITEMS = 1000
const PREVIEW_MAX_STRING = 4000
const PREVIEW_MAX_CHARS_COLLAPSED = 8000
const PREVIEW_MAX_CHARS_EXPANDED = 120000

const toggleToolPanel = () => {
  isToolPanelExpanded.value = !isToolPanelExpanded.value
}

const toggleArgs = () => {
  isArgsExpanded.value = !isArgsExpanded.value
}

const toggleResult = () => {
  isResultExpanded.value = !isResultExpanded.value
}

const checkOverflow = () => {
  nextTick(() => {
    if (argsBodyRef.value) {
      argsHasOverflow.value = argsBodyRef.value.scrollHeight > argsBodyRef.value.clientHeight
    }
    if (resultBodyRef.value) {
      resultHasOverflow.value = resultBodyRef.value.scrollHeight > resultBodyRef.value.clientHeight
    }
  })
}

onMounted(checkOverflow)
watch(() => [props.message.metadata?.tool_args, props.message.metadata?.tool_result], checkOverflow, { deep: true })

const truncatePreviewText = (text: string, maxChars: number) => {
  if (text.length <= maxChars) return text
  return `${text.slice(0, maxChars)}... [truncated ${text.length - maxChars} chars]`
}

const normalizeForPreview = (value: any, depth = 0, budget = { nodes: 0 }): any => {
  if (value === null || value === undefined) return value
  if (budget.nodes >= PREVIEW_MAX_ITEMS) return '[Truncated: too many nodes]'
  budget.nodes += 1

  if (typeof value === 'string') {
    if (value.length > PREVIEW_MAX_STRING) {
      return `${value.slice(0, PREVIEW_MAX_STRING)}... [truncated ${value.length - PREVIEW_MAX_STRING} chars]`
    }
    return value
  }

  if (typeof value !== 'object') return value
  if (depth >= PREVIEW_MAX_DEPTH) return '[Truncated: max depth reached]'

  if (Array.isArray(value)) {
    return value.map((item) => normalizeForPreview(item, depth + 1, budget))
  }

  const out: Record<string, any> = {}
  for (const key of Object.keys(value)) {
    out[key] = normalizeForPreview(value[key], depth + 1, budget)
  }
  return out
}

const stringifyPreview = (value: any, maxChars: number) => {
  if (value === null || value === undefined) return ''
  if (typeof value === 'string') {
    return truncatePreviewText(value, maxChars)
  }

  try {
    const normalized = normalizeForPreview(value)
    const text = JSON.stringify(normalized, null, 2)
    if (!text) return ''
    return truncatePreviewText(text, maxChars)
  } catch {
    return String(value)
  }
}

const stringifyToolResultPreview = (value: any, maxChars: number) => {
  const formattedJson = formatJsonValueIfPossible(value)
  if (formattedJson) {
    return truncatePreviewText(formattedJson, maxChars)
  }

  return stringifyPreview(value, maxChars)
}

const toolName = computed(() => props.message.metadata?.tool_name)
const toolHeaderDetail = computed(() =>
  getToolHeaderDetail({
    toolName: props.message.metadata?.tool_name,
    args: props.message.metadata?.tool_args,
  })
)
const toolStatus = computed(() => props.message.metadata?.status)
const toolPanelBorderClass = computed(() => 'border-l-warning')
const hasToolArgs = computed(() => (
  !!props.message.metadata?.tool_args && Object.keys(props.message.metadata.tool_args).length > 0
))
const hasToolResult = computed(() => !!props.message.metadata?.tool_result)

const formattedArgs = computed(() => {
  const maxChars = isArgsExpanded.value ? PREVIEW_MAX_CHARS_EXPANDED : PREVIEW_MAX_CHARS_COLLAPSED
  return stringifyPreview(props.message.metadata?.tool_args, maxChars)
})

const formattedToolResult = computed(() => {
  const maxChars = isResultExpanded.value ? PREVIEW_MAX_CHARS_EXPANDED : PREVIEW_MAX_CHARS_COLLAPSED
  return stringifyToolResultPreview(props.message.metadata?.tool_result, maxChars)
})

const toolResultStoredArtifactViews = computed(() =>
  buildStoredArtifactViews(
    props.message.metadata?.tool_result,
    props.message.metadata?.tracked_artifacts,
  )
)

const toolStatusClass = computed(() => {
  switch (toolStatus.value) {
    case 'running':
      return 'bg-warning/20 text-warning'
    case 'completed':
      return 'bg-success/20 text-success'
    case 'failed':
      return 'bg-error/20 text-error'
    case 'pending':
      return 'bg-base-300 text-base-content/60'
    default:
      return ''
  }
})

const toolStatusText = computed(() => {
  switch (toolStatus.value) {
    case 'running':
      return `⏳ ${t('agent.statusRunning')}`
    case 'completed':
      return `✓ ${t('agent.statusCompleted')}`
    case 'failed':
      return `✗ ${t('agent.statusFailed')}`
    case 'pending':
      return t('agent.statusPending')
    default:
      return ''
  }
})

const duration = computed(() => {
  const ms = props.message.metadata?.duration_ms
  if (ms) {
    return `${(ms / 1000).toFixed(1)}s`
  }
  return null
})

const fileVerificationStatus = computed(() => {
  const raw = props.message.metadata?.file_verification_status
  return raw === 'verified' || raw === 'pending' || raw === 'failed' ? raw : null
})

const fileVerificationText = computed(() => {
  switch (fileVerificationStatus.value) {
    case 'verified':
      return 'Artifact verified'
    case 'pending':
      return 'Verification pending'
    case 'failed':
      return 'Write failed'
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
      return 'bg-base-300 text-base-content/60'
  }
})
</script>
