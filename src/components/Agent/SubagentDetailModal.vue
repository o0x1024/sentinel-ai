<template>
  <Teleport to="body">
    <Transition name="modal-fade">
      <div 
        v-if="visible" 
        class="modal-backdrop fixed inset-0 bg-black/50 z-[100] flex items-center justify-center p-4"
        @click.self="emit('close')"
      >
        <div class="modal-content bg-base-100 rounded-xl shadow-2xl w-full max-w-5xl h-[90vh] flex flex-col overflow-hidden">
          <!-- Header -->
          <div class="modal-header flex items-center justify-between px-5 py-3 border-b border-base-300 bg-base-100">
            <div class="flex items-center gap-3">
              <div 
                class="w-10 h-10 rounded-lg flex items-center justify-center"
                :class="statusBgClass"
              >
                <i class="fas" :class="statusIconClass"></i>
              </div>
              <div>
                <h3 class="font-semibold text-lg flex items-center gap-2 text-base-content">
                  {{ subagent?.role || t('agent.subagentRoles.generic') }}
                  <span 
                    class="badge badge-sm"
                    :class="statusBadgeClass"
                  >
                    {{ t(`agent.subagentStatus.${subagent?.status || 'queued'}`) }}
                  </span>
                </h3>
                <div class="flex items-center gap-3 text-xs text-base-content/50">
                  <span class="font-mono">{{ subagent?.id || '' }}</span>
                  <span v-if="subagent?.startedAt">
                    <i class="fas fa-clock mr-1"></i>
                    {{ formatDateTime(subagent.startedAt) }}
                  </span>
                  <span v-if="subagent?.duration">
                    <i class="fas fa-stopwatch mr-1"></i>
                    {{ formatDuration(subagent.duration) }}
                  </span>
                </div>
              </div>
            </div>
            <div class="flex items-center gap-2">
              <!-- Tasks toggle button -->
              <button 
                v-if="hasTasks"
                @click="showTasks = !showTasks"
                class="btn btn-sm gap-1"
                :class="showTasks ? 'btn-primary' : 'btn-ghost text-primary'"
                :title="t('agent.tasks')"
              >
                <i class="fas fa-tasks"></i>
                <span>{{ t('agent.tasks') }}</span>
                <span class="badge badge-xs badge-primary">{{ taskItems.length }}</span>
              </button>
              <button 
                class="btn btn-sm btn-ghost btn-circle"
                @click="emit('close')"
              >
                <i class="fas fa-times"></i>
              </button>
            </div>
          </div>

          <!-- Progress bar for running status -->
          <div v-if="subagent?.status === 'running'" class="px-5 py-2 bg-base-200/50 border-b border-base-300">
            <div class="flex items-center justify-between text-xs text-base-content/60 mb-1">
              <span>{{ t('agent.subagentDetail.progress') }}</span>
              <span>{{ subagent?.progress || 0 }}%</span>
            </div>
            <div class="h-1.5 bg-base-300 rounded-full overflow-hidden">
              <div 
                class="h-full bg-primary transition-all duration-300"
                :style="{ width: `${subagent?.progress || 0}%` }"
              ></div>
            </div>
          </div>

          <!-- Task info bar -->
          <div v-if="displayTask" class="px-5 py-2 bg-base-200/30 border-b border-base-300">
            <div class="flex items-start gap-2">
              <i class="fas fa-tasks text-primary mt-0.5"></i>
              <div class="text-sm text-base-content/80 line-clamp-2">
                {{ displayTask }}
              </div>
            </div>
          </div>

          <!-- Main content area -->
          <div class="flex-1 flex overflow-hidden">
            <!-- Message Flow -->
            <div class="flex-1 flex flex-col overflow-hidden">
              <SimpleMessageFlow
                ref="messageFlowRef"
                :messages="displayMessages"
                :is-loading="messagesLoading"
                :is-streaming="subagent?.status === 'running' && !streamingContent && !streamingReasoningContent"
                class="flex-1 p-4"
              />
            </div>

            <!-- Tasks Panel -->
            <div 
              v-if="showTasks && hasTasks"
              class="w-80 border-l border-base-300 flex flex-col overflow-hidden bg-base-100"
            >
              <TaskPanel
                :tasks="panelTasks"
                :is-active="showTasks"
                class="h-full"
                @close="showTasks = false"
              />
            </div>
          </div>

          <!-- Error display -->
          <div v-if="subagent?.status === 'failed' && subagent?.error" class="px-5 py-3 bg-error/10 border-t border-error/20">
            <div class="flex items-start gap-2 text-error">
              <i class="fas fa-exclamation-circle mt-0.5"></i>
              <div class="text-sm">{{ subagent.error }}</div>
            </div>
          </div>

          <!-- Footer -->
          <div class="modal-footer flex items-center justify-between px-5 py-3 border-t border-base-300 bg-base-200/30">
            <div class="flex items-center gap-2 text-xs text-base-content/50">
              <span v-if="subagent?.parentId">
                <i class="fas fa-link mr-1"></i>
                {{ t('agent.parentExecution') }}: {{ subagent.parentId.slice(0, 8) }}
              </span>
            </div>
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
import { computed, ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import SimpleMessageFlow, { type SimpleMessage } from './SimpleMessageFlow.vue'
import TaskPanel from './TaskPanel.vue'
import { mapTaskRuntimeItemsToAgentTasks } from '@/types/agentTask'
import type { SubagentMessageState } from './useSubagentMessageStore'

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

const props = defineProps<{
  visible: boolean
  subagent: SubagentItem | null
  messageState: SubagentMessageState | null
  loadMessages: (subagentId: string) => Promise<void>
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const { t } = useI18n()

const messageFlowRef = ref<InstanceType<typeof SimpleMessageFlow> | null>(null)
const showTasks = ref(false)
const messages = computed(() => props.messageState?.messages || [])
const messagesLoading = computed(() => props.messageState?.messagesLoading || false)
const taskItems = computed(() => props.messageState?.taskItems || [])
const streamingContent = computed(() => props.messageState?.streamingContent || '')
const streamingReasoningContent = computed(
  () => props.messageState?.streamingReasoningContent || ''
)
const panelTasks = computed(() => mapTaskRuntimeItemsToAgentTasks(taskItems.value, props.subagent?.id || undefined))

// Status styling
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

// Has tasks
const hasTasks = computed(() => taskItems.value.length > 0)

// Convert messages to SimpleMessage format, including streaming content
const displayMessages = computed<SimpleMessage[]>(() => {
  const result: SimpleMessage[] = messages.value.map(msg => ({
    id: msg.id,
    role: msg.role as 'user' | 'assistant' | 'tool' | 'system',
    content: msg.content,
    reasoning_content: msg.reasoning_content,
    tool_calls: msg.tool_calls,
    timestamp: msg.timestamp,
    metadata: msg.metadata ? tryParseJson(msg.metadata) : undefined,
  }))
  
  // If there's streaming content and subagent is running, add a temporary streaming message
  if (props.subagent?.status === 'running' && (streamingContent.value || streamingReasoningContent.value)) {
    result.push({
      id: 'streaming-' + Date.now(),
      role: 'assistant',
      content: streamingContent.value || null,
      reasoning_content: streamingReasoningContent.value || null,
      tool_calls: null,
      timestamp: new Date().toISOString(),
      metadata: undefined,
    })
  }
  
  return result
})

// Try parse JSON safely
const tryParseJson = (str: string): any => {
  try {
    return JSON.parse(str)
  } catch {
    return undefined
  }
}

// Display task
const displayTask = computed(() => {
  const task = props.subagent?.task
  if (!task) return ''
  const match = task.match(/Subagent task:\s*(.+)/is)
  return match ? match[1].trim() : task.trim()
})

// Format functions
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

// Watch for visibility and subagent changes
watch(
  () => [props.visible, props.subagent?.id],
  (nextValue, previousValue) => {
    const [visible, subagentId] = nextValue
    const previousSubagentId = previousValue?.[1]
    if (visible && subagentId) {
      void props.loadMessages(String(subagentId)).then(() => {
        nextTick(() => {
          messageFlowRef.value?.scrollToBottom()
        })
      })
      if (subagentId !== previousSubagentId) {
        showTasks.value = false
      }
    }
  },
  { immediate: true }
)

watch(
  () => [displayMessages.value.length, streamingContent.value, streamingReasoningContent.value],
  () => {
    nextTick(() => {
      messageFlowRef.value?.scrollToBottom()
    })
  }
)

watch(
  () => taskItems.value.length,
  (count, previousCount) => {
    if (props.visible && count > 0 && previousCount === 0) {
      showTasks.value = true
    }
  }
)
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
