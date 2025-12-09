<template>
  <div class="agent-view h-full flex flex-col bg-gradient-to-br from-base-100 to-base-200 overflow-hidden">
    <!-- Main content area -->
    <div class="agent-main flex flex-1 overflow-hidden min-h-0">
      <!-- Message flow -->
      <MessageFlow 
        ref="messageFlowRef"
        :messages="messages"
        :is-streaming="isStreaming"
        :streaming-content="streamingContent"
        class="flex-1"
      />
      
      <!-- Todo panel (when available) -->
      <TodoPanel 
        v-if="hasTodos" 
        :todos="todos" 
        class="todo-sidebar w-72 flex-shrink-0 border-l border-base-300 p-4 overflow-y-auto"
      />
    </div>
    
    <!-- Input area - using InputAreaComponent for full features -->
    <InputAreaComponent
      v-model:input-message="inputValue"
      :is-loading="isExecuting"
      :show-debug-info="false"
      :rag-enabled="ragEnabled"
      :pending-attachments="pendingAttachments"
      :referenced-traffic="referencedTraffic"
      @send-message="handleSubmit"
      @stop-execution="handleStop"
      @toggle-rag="handleToggleRAG"
      @add-attachments="handleAddAttachments"
      @remove-attachment="handleRemoveAttachment"
      @remove-traffic="handleRemoveTraffic"
      @clear-traffic="handleClearTraffic"
      @clear-conversation="handleClearConversation"
    />
    
    <!-- Error display -->
    <div v-if="error" class="error-banner flex items-center gap-2 px-4 py-3 bg-error/10 border-t border-error text-error text-sm">
      <span class="error-icon flex-shrink-0">⚠️</span>
      <span class="error-message flex-1 overflow-hidden text-ellipsis whitespace-nowrap">{{ error }}</span>
      <button @click="clearError" class="error-close bg-transparent border-none text-error cursor-pointer text-xl leading-none px-1 hover:text-base-content">×</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AgentMessage } from '@/types/agent'
import type { Todo } from '@/types/todo'
import { useAgentEvents } from '@/composables/useAgentEvents'
import { useTodos } from '@/composables/useTodos'
import MessageFlow from './MessageFlow.vue'
import TodoPanel from './TodoPanel.vue'
import InputAreaComponent from '@/components/InputAreaComponent.vue'

// 流量引用类型
type TrafficSendType = 'request' | 'response' | 'both'
interface ReferencedTraffic {
  id: number
  url: string
  method: string
  host: string
  status_code: number
  request_headers?: string
  request_body?: string
  response_headers?: string
  response_body?: string
  sendType?: TrafficSendType
}

const props = withDefaults(defineProps<{
  executionId?: string
  showTodos?: boolean
}>(), {
  showTodos: true,
})

const emit = defineEmits<{
  (e: 'submit', task: string): void
  (e: 'complete', result: any): void
  (e: 'error', error: string): void
}>()

// Refs
const messageFlowRef = ref<InstanceType<typeof MessageFlow> | null>(null)
const inputValue = ref('')
const isExecuting = ref(false)
const localError = ref<string | null>(null)
const isStreaming = ref(false)
const streamingContent = ref('')

// Feature toggles
const ragEnabled = ref(false)
const pendingAttachments = ref<any[]>([])
const referencedTraffic = ref<ReferencedTraffic[]>([])

// Agent events
const agentEvents = useAgentEvents()
const messages = computed(() => agentEvents.messages.value)

// Todos
const todosComposable = useTodos()
const todos = computed(() => todosComposable.todos.value)
const hasTodos = computed(() => props.showTodos && todosComposable.hasTodos.value)

// Combined error
const error = computed(() => localError.value || agentEvents.error.value)

// Clear error
const clearError = () => {
  localError.value = null
}

// Handle RAG toggle
const handleToggleRAG = (enabled: boolean) => {
  ragEnabled.value = enabled
  console.log('[AgentView] RAG:', enabled ? 'enabled' : 'disabled')
}

// Handle attachments
const handleAddAttachments = async (filePaths: string[]) => {
  if (!filePaths || filePaths.length === 0) return
  
  try {
    const attachments = await invoke<any[]>('upload_multiple_images', { filePaths })
    if (attachments && attachments.length > 0) {
      pendingAttachments.value.push(...attachments)
      console.log('[AgentView] Uploaded', attachments.length, 'attachments')
    }
  } catch (error) {
    console.error('[AgentView] Upload failed:', error)
  }
}

const handleRemoveAttachment = (index: number) => {
  if (index >= 0 && index < pendingAttachments.value.length) {
    pendingAttachments.value.splice(index, 1)
  }
}

// Handle traffic references
const handleRemoveTraffic = (index: number) => {
  if (index >= 0 && index < referencedTraffic.value.length) {
    referencedTraffic.value.splice(index, 1)
  }
}

const handleClearTraffic = () => {
  referencedTraffic.value = []
}

// Add traffic references (for external use)
const addReferencedTraffic = (traffic: ReferencedTraffic[], type: TrafficSendType = 'both') => {
  const existingIds = new Set(referencedTraffic.value.map(t => t.id))
  const newTraffic = traffic
    .filter(t => !existingIds.has(t.id))
    .map(t => ({ ...t, sendType: type }))
  referencedTraffic.value.push(...newTraffic)
}

// Build traffic context for prompt
const buildTrafficContext = (traffic: ReferencedTraffic[]): string => {
  const parts: string[] = ['Referenced HTTP traffic:\n']
  
  traffic.forEach((t, index) => {
    const sendType = t.sendType || 'both'
    const typeLabel = sendType === 'request' ? 'Request' : sendType === 'response' ? 'Response' : 'Traffic'
    parts.push(`\n--- ${typeLabel} #${index + 1} ---`)
    parts.push(`URL: ${t.url}`)
    parts.push(`Method: ${t.method}`)
    parts.push(`Host: ${t.host}`)
    
    const showRequest = sendType === 'request' || sendType === 'both'
    const showResponse = sendType === 'response' || sendType === 'both'
    
    if (showResponse) {
      parts.push(`Status: ${t.status_code || 'N/A'}`)
    }
    
    if (showRequest && t.request_headers) {
      try {
        const headers = JSON.parse(t.request_headers)
        const headerStr = Object.entries(headers).map(([k, v]) => `  ${k}: ${v}`).join('\n')
        parts.push(`\nRequest Headers:\n${headerStr}`)
      } catch {
        parts.push(`\nRequest Headers: ${t.request_headers}`)
      }
    }
    
    if (showRequest && t.request_body) {
      const body = t.request_body.length > 2000 
        ? t.request_body.substring(0, 2000) + '... [truncated]'
        : t.request_body
      parts.push(`\nRequest Body:\n${body}`)
    }
    
    if (showResponse && t.response_headers) {
      try {
        const headers = JSON.parse(t.response_headers)
        const headerStr = Object.entries(headers).map(([k, v]) => `  ${k}: ${v}`).join('\n')
        parts.push(`\nResponse Headers:\n${headerStr}`)
      } catch {
        parts.push(`\nResponse Headers: ${t.response_headers}`)
      }
    }
    
    if (showResponse && t.response_body) {
      const body = t.response_body.length > 3000 
        ? t.response_body.substring(0, 3000) + '... [truncated]'
        : t.response_body
      parts.push(`\nResponse Body:\n${body}`)
    }
  })
  
  return parts.join('\n')
}

// Handle clear conversation
const handleClearConversation = () => {
  agentEvents.clearMessages()
  pendingAttachments.value = []
  referencedTraffic.value = []
  inputValue.value = ''
}

// Handle stop
const handleStop = () => {
  console.log('[AgentView] Stop requested')
  // TODO: implement stop logic
  isExecuting.value = false
  isStreaming.value = false
}

// Handle submit
const handleSubmit = async () => {
  const task = inputValue.value.trim()
  if (isExecuting.value || !task) return
  
  isExecuting.value = true
  isStreaming.value = true
  localError.value = null
  
  // Build full task with traffic context
  let fullTask = task
  if (referencedTraffic.value.length > 0) {
    const trafficContext = buildTrafficContext(referencedTraffic.value)
    fullTask = `${trafficContext}\n\nUser task: ${task}`
  }
  
  // Clear input and references
  inputValue.value = ''
  const usedAttachments = [...pendingAttachments.value]
  const usedTraffic = [...referencedTraffic.value]
  pendingAttachments.value = []
  referencedTraffic.value = []
  
  // Clear previous messages
  agentEvents.clearMessages()
  
  // Emit submit event
  emit('submit', fullTask)
  
  try {
    // Call agent_execute command
    const result = await invoke('agent_execute', {
      task: fullTask,
      config: {
        max_iterations: 10,
        timeout_secs: 300,
        force_todos: props.showTodos,
        enable_rag: ragEnabled.value,
        conversation_id: props.executionId || null,
        message_id: null,
        attachments: usedAttachments.length > 0 ? usedAttachments : undefined,
      }
    })
    
    emit('complete', result)
  } catch (e: any) {
    const errorMsg = e.toString()
    localError.value = errorMsg
    emit('error', errorMsg)
  } finally {
    isExecuting.value = false
    isStreaming.value = false
  }
}

// Initialize
onMounted(() => {
  // Focus input on mount
})

// Expose methods
defineExpose({
  clearMessages: agentEvents.clearMessages,
  scrollToBottom: () => messageFlowRef.value?.scrollToBottom(),
  addReferencedTraffic,
})
</script>

<style scoped>
.agent-view {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

/* Responsive */
@media (max-width: 768px) {
  .agent-main {
    flex-direction: column;
  }
  
  .todo-sidebar {
    width: 100%;
    border-left: none;
    border-top: 1px solid hsl(var(--b3));
    max-height: 200px;
  }
}
</style>
