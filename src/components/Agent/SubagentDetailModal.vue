<template>
  <Teleport to="body">
    <Transition name="modal-fade">
      <div 
        v-if="visible" 
        class="modal-backdrop fixed inset-0 bg-black/50 z-[100] flex items-center justify-center p-4"
        @click.self="emit('close')"
      >
        <div class="modal-content bg-base-100 rounded-xl shadow-2xl w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden">
          <!-- Header -->
          <div class="modal-header flex items-center justify-between px-5 py-4 border-b border-base-300">
            <div class="flex items-center gap-3">
              <div 
                class="w-10 h-10 rounded-lg flex items-center justify-center"
                :class="statusBgClass"
              >
                <i class="fas" :class="statusIconClass"></i>
              </div>
              <div>
                <h3 class="font-semibold text-lg">
                  {{ subagent?.role || t('agent.subagentRoles.generic') }}
                </h3>
                <span class="text-xs text-base-content/50 font-mono">
                  {{ subagent?.id || '' }}
                </span>
              </div>
            </div>
            <button 
              class="btn btn-sm btn-ghost btn-circle"
              @click="emit('close')"
            >
              <i class="fas fa-times"></i>
            </button>
          </div>

          <!-- Content -->
          <div class="modal-body flex-1 overflow-y-auto p-5 space-y-4">
            <!-- Status Section -->
            <div class="flex items-center gap-4">
              <div class="flex items-center gap-2">
                <span class="text-sm text-base-content/60">{{ t('agent.subagentDetail.status') }}:</span>
                <span 
                  class="badge"
                  :class="statusBadgeClass"
                >
                  {{ t(`agent.subagentStatus.${subagent?.status || 'queued'}`) }}
                </span>
              </div>
              <div v-if="subagent?.progress !== undefined" class="flex items-center gap-2 flex-1">
                <span class="text-sm text-base-content/60">{{ t('agent.subagentDetail.progress') }}:</span>
                <div class="flex-1 h-2 bg-base-200 rounded-full overflow-hidden max-w-[200px]">
                  <div 
                    class="h-full transition-all duration-300"
                    :class="subagent?.status === 'failed' ? 'bg-error' : 'bg-primary'"
                    :style="{ width: `${subagent?.progress || 0}%` }"
                  ></div>
                </div>
                <span class="text-sm font-medium">{{ subagent?.progress || 0 }}%</span>
              </div>
            </div>

            <div v-if="!hasExtraDetails" class="rounded-lg border border-base-200 bg-base-200/40 p-4 text-sm text-base-content/60">
              <i class="fas fa-info-circle mr-2 text-base-content/50"></i>
              {{ detailHint }}
            </div>

            <!-- Task Section -->
            <div v-if="subagent?.task" class="space-y-2">
              <h4 class="text-sm font-semibold text-base-content/70 flex items-center gap-2">
                <i class="fas fa-tasks text-primary"></i>
                {{ t('agent.subagentDetail.task') }}
              </h4>
              <div class="bg-base-200/50 rounded-lg p-3 text-sm whitespace-pre-wrap">
                {{ subagent.task }}
              </div>
            </div>

            <!-- Summary Section -->
            <div v-if="subagent?.summary" class="space-y-2">
              <h4 class="text-sm font-semibold text-base-content/70 flex items-center gap-2">
                <i class="fas fa-file-alt text-info"></i>
                {{ t('agent.subagentDetail.summary') }}
              </h4>
              <div class="bg-info/10 border border-info/20 rounded-lg p-3 text-sm whitespace-pre-wrap">
                {{ subagent.summary }}
              </div>
            </div>

            <!-- Error Section -->
            <div v-if="subagent?.status === 'failed' && subagent?.error" class="space-y-2">
              <h4 class="text-sm font-semibold text-error flex items-center gap-2">
                <i class="fas fa-exclamation-circle"></i>
                {{ t('agent.subagentDetail.error') }}
              </h4>
              <div class="bg-error/10 border border-error/20 rounded-lg p-3 text-sm text-error whitespace-pre-wrap">
                {{ subagent.error }}
              </div>
            </div>

            <!-- Tools Section -->
            <div v-if="subagent?.tools && subagent.tools.length > 0" class="space-y-2">
              <h4 class="text-sm font-semibold text-base-content/70 flex items-center gap-2">
                <i class="fas fa-wrench text-warning"></i>
                {{ t('agent.subagentDetail.toolsUsed') }} ({{ subagent.tools.length }})
              </h4>
              <div class="flex flex-wrap gap-2">
                <span 
                  v-for="tool in subagent.tools" 
                  :key="tool"
                  class="px-3 py-1.5 rounded-lg bg-base-200 text-sm font-mono"
                >
                  {{ tool }}
                </span>
              </div>
            </div>

            <!-- Timing Section -->
            <div class="flex items-center gap-6 text-sm text-base-content/60">
              <div v-if="subagent?.startedAt" class="flex items-center gap-2">
                <i class="fas fa-clock"></i>
                <span>{{ t('agent.subagentDetail.startedAt') }}:</span>
                <span class="font-medium text-base-content">{{ formatDateTime(subagent.startedAt) }}</span>
              </div>
              <div v-if="subagent?.duration" class="flex items-center gap-2">
                <i class="fas fa-stopwatch"></i>
                <span>{{ t('agent.subagentDetail.duration') }}:</span>
                <span class="font-medium text-base-content">{{ formatDuration(subagent.duration) }}</span>
              </div>
            </div>

            <!-- Parent Info -->
            <div v-if="subagent?.parentId" class="flex items-center gap-2 text-sm text-base-content/50">
              <i class="fas fa-link"></i>
              <span>{{ t('agent.parentExecution') }}:</span>
              <span class="font-mono text-xs">{{ subagent.parentId }}</span>
            </div>

            <!-- History Section -->
            <div class="space-y-2">
              <h4 class="text-sm font-semibold text-base-content/70 flex items-center gap-2">
                <i class="fas fa-history text-base-content/50"></i>
                {{ t('agent.subagentDetail.history') }}
              </h4>
              <div v-if="historyLoading" class="text-sm text-base-content/50">
                {{ t('agent.subagentDetail.loading') }}
              </div>
              <div v-else-if="historyRuns.length === 0" class="text-sm text-base-content/50">
                {{ t('agent.subagentDetail.noHistory') }}
              </div>
              <div v-else class="space-y-2">
                <div
                  v-for="run in historyRuns"
                  :key="run.id"
                  class="rounded-lg border border-base-200 bg-base-100 px-3 py-2 cursor-pointer transition-colors"
                  :class="selectedRunId === run.id ? 'border-primary/50 bg-primary/5' : 'hover:bg-base-200/60'"
                  @click="selectRun(run.id)"
                >
                  <div class="flex items-start justify-between gap-2">
                    <div class="min-w-0">
                      <div class="text-xs text-base-content/50 font-mono">{{ run.id }}</div>
                      <div class="text-sm font-medium text-base-content">
                        {{ run.role || t('agent.subagentRoles.generic') }}
                      </div>
                      <div class="text-xs text-base-content/50">
                        {{ formatRunTime(run.started_at, run.completed_at) }}
                      </div>
                    </div>
                    <span class="badge badge-sm" :class="statusBadgeClassForRun(run.status)">
                      {{ t(`agent.subagentStatus.${run.status}`) }}
                    </span>
                  </div>
                  <div v-if="run.task" class="mt-2 text-xs text-base-content/70 line-clamp-2">
                    <i class="fas fa-tasks mr-1 text-base-content/40"></i>
                    {{ run.task }}
                  </div>
                  <div v-if="run.output" class="mt-2 text-xs text-base-content/70 line-clamp-3 bg-base-200/50 rounded px-2 py-1">
                    {{ run.output }}
                  </div>
                  <div v-if="run.status === 'failed' && run.error" class="mt-2 text-xs text-error bg-error/10 rounded px-2 py-1">
                    <i class="fas fa-exclamation-circle mr-1"></i>
                    {{ run.error }}
                  </div>
                </div>
              </div>
            </div>

            <!-- Session Messages -->
            <div class="space-y-2">
              <h4 class="text-sm font-semibold text-base-content/70 flex items-center gap-2">
                <i class="fas fa-comments text-base-content/50"></i>
                {{ t('agent.subagentDetail.sessionMessages') }}
              </h4>
              <div v-if="!selectedRunId" class="text-sm text-base-content/50">
                {{ t('agent.subagentDetail.selectRun') }}
              </div>
              <div v-else-if="messagesLoading" class="text-sm text-base-content/50">
                {{ t('agent.subagentDetail.loadingMessages') }}
              </div>
              <div v-else-if="messages.length === 0" class="text-sm text-base-content/50">
                {{ t('agent.subagentDetail.noMessages') }}
              </div>
              <div v-else class="space-y-2">
                <div
                  v-for="msg in messages"
                  :key="msg.id"
                  class="rounded-lg border border-base-200 bg-base-100 px-3 py-2"
                >
                  <div class="flex items-center gap-2 text-xs text-base-content/50">
                    <span class="badge badge-xs">{{ msg.role }}</span>
                    <span>{{ formatMessageTime(msg.timestamp) }}</span>
                  </div>
                  <div class="mt-1 text-sm whitespace-pre-wrap">{{ msg.content }}</div>
                </div>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="modal-footer flex items-center justify-end gap-3 px-5 py-4 border-t border-base-300 bg-base-200/30">
            <button 
              class="btn btn-sm btn-ghost"
              @click="emit('close')"
            >
              {{ t('agent.close') }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'

type SubagentStatus = 'running' | 'queued' | 'completed' | 'failed'

interface SubagentItem {
  id: string
  role?: string
  status: SubagentStatus
  progress?: number
  tools?: string[]
  parentId: string
  summary?: string
  task?: string
  error?: string
  startedAt?: number
  duration?: number
}

interface SubagentRunRecord {
  id: string
  parent_execution_id: string
  role?: string | null
  task: string
  status: 'running' | 'queued' | 'completed' | 'failed'
  output?: string | null
  error?: string | null
  model_name?: string | null
  model_provider?: string | null
  started_at: string
  completed_at?: string | null
  created_at: string
  updated_at: string
}

interface SubagentMessageRecord {
  id: string
  subagent_run_id: string
  role: string
  content: string
  metadata?: string | null
  tool_calls?: string | null
  attachments?: string | null
  reasoning_content?: string | null
  timestamp: string
  structured_data?: string | null
}

const props = defineProps<{
  visible: boolean
  subagent: SubagentItem | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const { t } = useI18n()

const statusBadgeClass = computed(() => {
  const status = props.subagent?.status
  if (status === 'running') return 'badge-primary'
  if (status === 'queued') return 'badge-warning'
  if (status === 'failed') return 'badge-error'
  return 'badge-success'
})

const statusBgClass = computed(() => {
  const status = props.subagent?.status
  if (status === 'running') return 'bg-primary/20 text-primary'
  if (status === 'queued') return 'bg-warning/20 text-warning'
  if (status === 'failed') return 'bg-error/20 text-error'
  return 'bg-success/20 text-success'
})

const statusIconClass = computed(() => {
  const status = props.subagent?.status
  if (status === 'running') return 'fa-spinner fa-spin'
  if (status === 'queued') return 'fa-clock'
  if (status === 'failed') return 'fa-times'
  return 'fa-check'
})

const historyRuns = ref<SubagentRunRecord[]>([])
const historyLoading = ref(false)
const selectedRunId = ref<string | null>(null)
const messages = ref<SubagentMessageRecord[]>([])
const messagesLoading = ref(false)

const hasExtraDetails = computed(() => {
  const s = props.subagent
  if (!s) return false
  return Boolean(
    s.task ||
      s.summary ||
      (s.status === 'failed' && s.error) ||
      (s.tools && s.tools.length > 0) ||
      s.startedAt ||
      s.duration ||
      s.parentId ||
      s.progress !== undefined
  )
})

const detailHint = computed(() => {
  const status = props.subagent?.status
  if (status === 'running' || status === 'queued') {
    return t('agent.subagentDetail.waiting')
  }
  return t('agent.subagentDetail.noDetails')
})

const statusBadgeClassForRun = (status: SubagentRunRecord['status']) => {
  if (status === 'running') return 'badge-primary'
  if (status === 'queued') return 'badge-warning'
  if (status === 'failed') return 'badge-error'
  return 'badge-success'
}

const formatRunTime = (startedAt: string, completedAt?: string | null) => {
  const start = new Date(startedAt)
  const end = completedAt ? new Date(completedAt) : null
  if (Number.isNaN(start.getTime())) return ''
  if (!end || Number.isNaN(end.getTime())) {
    return `${start.toLocaleString()}`
  }
  const durationMs = end.getTime() - start.getTime()
  return `${start.toLocaleString()} · ${formatDuration(Math.max(0, durationMs))}`
}

const formatMessageTime = (timestamp: string) => {
  const date = new Date(timestamp)
  if (Number.isNaN(date.getTime())) return ''
  return date.toLocaleString()
}

const selectRun = (runId: string) => {
  if (selectedRunId.value === runId) return
  selectedRunId.value = runId
}

const loadMessages = async (runId: string) => {
  messagesLoading.value = true
  try {
    const result = await invoke<SubagentMessageRecord[]>('get_subagent_messages', {
      subagentRunId: runId,
    })
    messages.value = result || []
  } catch (e) {
    console.error('[SubagentDetailModal] Failed to load subagent messages:', e)
    messages.value = []
  } finally {
    messagesLoading.value = false
  }
}

const loadHistory = async () => {
  const parentId = props.subagent?.parentId
  if (!props.visible || !parentId) {
    historyRuns.value = []
    selectedRunId.value = null
    messages.value = []
    return
  }
  historyLoading.value = true
  try {
    const runs = await invoke<SubagentRunRecord[]>('get_subagent_runs', {
      parentExecutionId: parentId,
    })
    historyRuns.value = runs || []
    if (selectedRunId.value && historyRuns.value.some(r => r.id === selectedRunId.value)) {
      return
    }
    const preferredId = props.subagent?.id
    const initialId = preferredId && historyRuns.value.some(r => r.id === preferredId)
      ? preferredId
      : historyRuns.value[0]?.id || null
    selectedRunId.value = initialId
  } catch (e) {
    console.error('[SubagentDetailModal] Failed to load subagent runs:', e)
    historyRuns.value = []
    selectedRunId.value = null
    messages.value = []
  } finally {
    historyLoading.value = false
  }
}

watch(
  () => [props.visible, props.subagent?.parentId],
  () => {
    loadHistory()
  }
)

watch(
  () => selectedRunId.value,
  (runId) => {
    if (runId) {
      loadMessages(runId)
    } else {
      messages.value = []
    }
  }
)

const formatDateTime = (timestamp: number) => {
  return new Date(timestamp).toLocaleString()
}

const formatDuration = (ms: number) => {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
  const minutes = Math.floor(ms / 60000)
  const seconds = ((ms % 60000) / 1000).toFixed(0)
  return `${minutes}m ${seconds}s`
}
</script>

<style scoped>
.modal-fade-enter-active,
.modal-fade-leave-active {
  transition: opacity 0.2s ease;
}

.modal-fade-enter-active .modal-content,
.modal-fade-leave-active .modal-content {
  transition: transform 0.2s ease, opacity 0.2s ease;
}

.modal-fade-enter-from,
.modal-fade-leave-to {
  opacity: 0;
}

.modal-fade-enter-from .modal-content,
.modal-fade-leave-to .modal-content {
  transform: scale(0.95);
  opacity: 0;
}
</style>
