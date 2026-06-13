<template>
  <div class="rounded-lg overflow-hidden border border-info/30 bg-info/5 mb-2">
    <button
      type="button"
      class="flex w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-info/10"
      :aria-expanded="isExpanded ? 'true' : 'false'"
      @click="toggleExpanded"
    >
      <i
        :class="[
          'fas text-xs text-info transition-transform flex-shrink-0',
          isExpanded ? 'fa-chevron-down' : 'fa-chevron-right',
        ]"
      ></i>
      <div
        class="w-8 h-8 rounded-full bg-info/80 flex items-center justify-center flex-shrink-0 shadow-sm"
      >
        <i class="fas fa-file-alt text-white text-sm"></i>
      </div>
      <div class="flex-1 min-w-0">
        <div class="font-semibold text-sm text-info">{{ title }}</div>
        <div class="text-xs text-base-content/70 font-mono truncate mt-0.5">
          {{ subtitle }}
        </div>
      </div>
      <span v-if="statusText" :class="['badge badge-sm whitespace-nowrap', statusBadgeClass]">
        {{ statusText }}
      </span>
    </button>

    <div
      v-show="isExpanded"
      class="px-4 py-3 border-t border-info/20 bg-base-100/60 space-y-3"
    >
      <div
        v-if="displayStatus === 'failed'"
        class="rounded-lg border border-error/20 bg-error/10 px-3 py-2 text-sm text-error whitespace-pre-wrap break-words"
      >
        {{ errorText }}
      </div>

      <div v-else-if="displayStatus === 'running' || displayStatus === 'pending'" class="text-sm text-base-content/70">
        {{ t('agent.skillStatusRunning') }}
      </div>

      <SkillFileMarkdownSections v-else-if="fileSections.length > 0" :files="fileSections" />

      <div v-else-if="fallbackMarkdown" class="max-h-[480px] overflow-y-auto">
        <MarkdownRenderer :content="fallbackMarkdown" />
      </div>

      <div v-else class="text-sm text-base-content/60">
        {{ t('agent.skillHelperFileEmpty') }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import MarkdownRenderer from './MarkdownRenderer.vue'
import SkillFileMarkdownSections from './SkillFileMarkdownSections.vue'
import {
  extractSkillsToolErrorMessage,
  isSkillsToolSuccess,
  parseSkillsToolResult,
  resolveSkillFileSections,
} from './skillsToolSupport'

const props = withDefaults(
  defineProps<{
    title?: string
    args?: Record<string, unknown> | null
    result?: unknown
    error?: unknown
    status?: string
  }>(),
  {
    title: '',
    args: null,
    result: undefined,
    error: undefined,
    status: 'pending',
  },
)

const { t } = useI18n()
const isExpanded = ref(false)

const toggleExpanded = () => {
  isExpanded.value = !isExpanded.value
}

const parsedResult = computed(() => parseSkillsToolResult(props.result))

const subtitle = computed(() => {
  const fromArgs = String(props.args?.file || '').trim()
  if (fromArgs) return fromArgs
  return parsedResult.value?.referencedFiles?.[0] || '—'
})

const panelTitle = computed(() => props.title || t('agent.skillReadFileTitle'))

const title = computed(() => panelTitle.value)

const loadedDespiteFailure = computed(() => {
  const status = String(props.status || '').toLowerCase()
  return status === 'failed' && isSkillsToolSuccess(props.result)
})

const displayStatus = computed(() => {
  if (loadedDespiteFailure.value) return 'completed'
  return String(props.status || '').toLowerCase()
})

const fileSections = computed(() =>
  resolveSkillFileSections(parsedResult.value?.content, parsedResult.value?.referencedFiles),
)

const fallbackMarkdown = computed(() => {
  const content = parsedResult.value?.content?.trim()
  if (!content || fileSections.value.length > 0) return ''
  return content
})

const errorText = computed(() => {
  const extracted = extractSkillsToolErrorMessage(props.error, props.result)
  if (extracted) return extracted
  if (typeof props.error === 'string' && props.error.trim()) return props.error.trim()
  return t('agent.skillInvokeFailedTitle')
})

const statusText = computed(() => {
  const status = displayStatus.value
  if (status === 'running' || status === 'pending') return t('agent.skillStatusRunning')
  if (status === 'failed') return t('agent.skillStatusFailed')
  if (status === 'completed' || status === 'success') return t('agent.skillStatusCompleted')
  return ''
})

const statusBadgeClass = computed(() => {
  const status = displayStatus.value
  if (status === 'failed') return 'badge-error'
  if (status === 'running' || status === 'pending') return 'badge-warning'
  return 'badge-info'
})
</script>
