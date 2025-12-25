<template>
  <div class="agent-view h-full flex bg-gradient-to-br from-base-100 to-base-200 overflow-hidden relative">
    <!-- Backdrop -->
    <div 
      v-if="showConversations || showToolConfig"
      class="conversation-backdrop absolute inset-0 bg-black/20 z-40 transition-opacity"
      @click="showConversations = false; showToolConfig = false"
    ></div>

    <!-- Conversation List Drawer -->
    <Transition name="slide-drawer">
      <div 
        v-if="showConversations"
        class="conversation-drawer absolute left-0 top-0 bottom-0 w-80 bg-base-100 shadow-2xl z-50 overflow-hidden"
      >
        <ConversationList 
          ref="conversationListRef"
          :current-conversation-id="conversationId"
          @select="handleSelectConversation"
          @create="handleCreateConversation"
          @close="showConversations = false"
        />
      </div>
    </Transition>

    <!-- Tool Config Panel -->
    <Transition name="slide-drawer-right">
      <div 
        v-if="showToolConfig"
        class="tool-config-drawer absolute right-0 top-0 bottom-0 w-96 bg-base-100 shadow-2xl z-50 overflow-hidden"
      >
        <ToolConfigPanel 
          :config="toolConfig"
          @update:config="handleToolConfigUpdate"
          @close="showToolConfig = false"
        />
      </div>
    </Transition>

    <!-- Main content area -->
    <div class="flex-1 flex flex-col overflow-hidden min-h-0">
      <!-- {{ t('agent.conversationHeader') }} -->
      <div class="conversation-header px-4 py-2 border-b border-base-300 flex items-center justify-between bg-base-100/50">
        <div class="flex items-center gap-2">
          <button 
            @click="showConversations = !showConversations"
            class="btn btn-sm btn-ghost"
            :title="t('agent.switchConversationList')"
          >
            <i class="fas fa-bars"></i>
          </button>
          <span class="text-sm font-medium text-base-content/70">
            {{ currentConversationTitle }}
          </span>
          
          <!-- RAG {{ t('agent.statusIndicator') }} -->
          <div v-if="ragEnabled" class="flex items-center gap-1 px-2 py-1 bg-info/10 rounded-md border border-info/30">
            <i class="fas fa-book text-info text-xs"></i>
            <span class="text-xs text-info font-medium">{{ t('agent.knowledgeBase') }}</span>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <!-- Vision History Button - shows when there's exploration history -->
          <button 
            v-if="visionEvents.hasHistory.value"
            @click="visionEvents.open()"
            class="btn btn-sm gap-1"
            :class="isVisionActive ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="isVisionActive ? t('agent.visionPanelOpen') : t('agent.viewVisionHistory')"
          >
            <i class="fas fa-eye"></i>
            <span>{{ t('agent.explore') }}</span>
            <span class="badge badge-xs badge-primary">{{ visionEvents.steps.value.length }}</span>
          </button>
          <button 
            @click="handleCreateConversation()"
            class="btn btn-sm btn-ghost gap-1"
            :title="t('agent.newConversation')"
          >
            <i class="fas fa-plus"></i>
            <span>{{ t('agent.newConversation') }}</span>
          </button>
        </div>
      </div>

      <!-- {{ t('agent.messagesAndTodos') }} -->
      <div class="flex flex-1 overflow-hidden min-h-0">
        <!-- Message flow -->
        <MessageFlow 
          ref="messageFlowRef"
          :messages="messages"
          :is-streaming="isStreaming"
          :streaming-content="streamingContent"
          :is-vision-active="isVisionActive"
          class="flex-1"
          @resend="handleResendMessage"
        />
        
        <!-- Side Panel (Vision or Todo) -->
        <div class="sidebar-container w-[420px] flex-shrink-0 border-l border-base-300 flex flex-col overflow-hidden bg-base-100" v-if="isVisionActive || hasTodos">
            <VisionExplorerPanel 
               v-if="isVisionActive"
               :steps="visionEvents.steps.value" 
               :coverage="visionEvents.coverage.value"
               :discovered-apis="visionEvents.discoveredApis.value"
               :is-active="isVisionActive"
               :current-url="visionEvents.currentUrl.value"
               :current-plan="visionEvents.currentPlan.value"
               :current-progress="visionEvents.currentProgress.value"
               :multi-agent="visionEvents.multiAgent.value"
               :is-multi-agent-mode="visionEvents.isMultiAgentMode.value"
               :activity="visionEvents.activity.value"
               :show-takeover-form="visionEvents.showTakeoverForm.value"
               :takeover-message="visionEvents.takeoverMessage.value"
               :takeover-fields="visionEvents.takeoverFields.value"
               :login-timeout-seconds="visionEvents.loginTimeoutSeconds.value"
               :execution-id="visionEvents.currentExecutionId.value"
               class="h-full border-0 rounded-none bg-transparent"
               @close="visionEvents.close()"
            />
            <TodoPanel 
              v-else-if="hasTodos" 
              :todos="todos" 
              class="h-full p-4 overflow-y-auto border-0 bg-transparent"
            />
        </div>
      </div>

      <!-- {{ t('agent.inputArea') }} -->
      <InputAreaComponent
        v-model:input-message="inputValue"
        :is-loading="isExecuting"
        :allow-takeover="true"
        :show-debug-info="false"
        :rag-enabled="ragEnabled"
        :tools-enabled="toolsEnabled"
        :web-search-enabled="webSearchEnabled"
        :pending-attachments="pendingAttachments"
        :referenced-traffic="referencedTraffic"
        @send-message="handleSubmit"
        @stop-execution="handleStop"
        @toggle-rag="handleToggleRAG"
        @toggle-tools="handleToggleTools"
        @toggle-web-search="handleToggleWebSearch"
        @add-attachments="handleAddAttachments"
        @remove-attachment="handleRemoveAttachment"
        @remove-traffic="handleRemoveTraffic"
        @clear-traffic="handleClearTraffic"
        @clear-conversation="handleClearConversation"
        @open-tool-config="showToolConfig = true"
      />

      <!-- {{ t('agent.errorDisplay') }} -->
      <div v-if="error" class="error-banner flex items-center gap-2 px-4 py-3 bg-error/10 border-t border-error text-error text-sm">
        <span class="error-icon flex-shrink-0">⚠️</span>
        <span class="error-message flex-1 overflow-hidden text-ellipsis whitespace-nowrap">{{ error }}</span>
        <button @click="clearError" class="error-close bg-transparent border-none text-error cursor-pointer text-xl leading-none px-1 hover:text-base-content">×</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import type { AgentMessage } from '@/types/agent'
import { useAgentEvents } from '@/composables/useAgentEvents'
import { useVisionEvents } from '@/composables/useVisionEvents'
import { useTodos } from '@/composables/useTodos'
import MessageFlow from './MessageFlow.vue'
import TodoPanel from './TodoPanel.vue'
import VisionExplorerPanel from './VisionExplorerPanel.vue'
import InputAreaComponent from '@/components/InputAreaComponent.vue'
import ConversationList from './ConversationList.vue'
import ToolConfigPanel from './ToolConfigPanel.vue'

// Traffic reference type
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
  selectedRole?: any
}>(), {
  showTodos: true,
})

const emit = defineEmits<{
  (e: 'submit', task: string): void
  (e: 'complete', result: any): void
  (e: 'error', error: string): void
}>()

// i18n
const { t } = useI18n()

// Refs
const messageFlowRef = ref<InstanceType<typeof MessageFlow> | null>(null)
const conversationListRef = ref<InstanceType<typeof ConversationList> | null>(null)
const inputValue = ref('')
const localError = ref<string | null>(null)
const conversationId = ref<string | null>(null)
const showConversations = ref(false) // Default hidden
const showToolConfig = ref(false)
const currentConversationTitle = ref(t('agent.newConversationTitle'))

// Feature toggles
const ragEnabled = ref(false)
const toolsEnabled = ref(false)
const webSearchEnabled = ref(false)
const pendingAttachments = ref<any[]>([])
const referencedTraffic = ref<ReferencedTraffic[]>([])

// Tool configuration
const toolConfig = ref({
  enabled: false,
  selection_strategy: 'Keyword',
  max_tools: 5,
  fixed_tools: ['local_time'],
  disabled_tools: [],
})

// Agent events
const agentEvents = useAgentEvents()
const messages = computed(() => agentEvents.messages.value)
const isExecuting = computed(() => agentEvents.isExecuting.value)
const isStreaming = computed(() => agentEvents.isExecuting.value && !!agentEvents.streamingContent.value)
const streamingContent = computed(() => agentEvents.streamingContent.value)

// Vision Events
// Important: pass through the nullable execution id ref so Vision Explorer can
// receive early events (start/plan/progress) and then bind itself to the session.
const visionEvents = useVisionEvents(agentEvents.currentExecutionId)
const isVisionActive = computed(() => visionEvents.isVisionActive.value)

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

// Handle Tools toggle
const handleToggleTools = (enabled: boolean) => {
  toolsEnabled.value = enabled
  toolConfig.value.enabled = enabled
  console.log('[AgentView] Tools:', enabled ? 'enabled' : 'disabled')
}

// Handle Tool Config update
const handleToolConfigUpdate = async (config: any) => {
  toolConfig.value = config
  toolsEnabled.value = config.enabled
  console.log('[AgentView] Tool config updated:', config)
  
  // Save tool config to database (global config, not bound to conversation)
  try {
    await invoke('save_tool_config', {
      toolConfig: config
    })
    console.log('[AgentView] Tool config saved globally')
  } catch (e) {
    console.error('[AgentView] Failed to save tool config:', e)
    localError.value = t('agent.failedToSaveToolConfig') + ': ' + e
  }
}

// Handle Web Search toggle
const handleToggleWebSearch = (enabled: boolean) => {
  webSearchEnabled.value = enabled
  console.log('[AgentView] Web Search:', enabled ? 'enabled' : 'disabled')
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
const handleClearConversation = async () => {
  if (!conversationId.value) {
    console.log('[AgentView] No conversation to clear')
    return
  }

  try {
    console.log('[AgentView] Clearing conversation:', conversationId.value)
    
    // Call backend to clear conversation messages
    await invoke('clear_conversation_messages', {
      conversationId: conversationId.value
    })
    
    // Clear frontend messages
    agentEvents.clearMessages()
    
    // Clear attachments and references
    pendingAttachments.value = []
    referencedTraffic.value = []
    inputValue.value = ''
    
    // Refresh conversation list (update message count)
    conversationListRef.value?.loadConversations()
    
    console.log('[AgentView] Conversation cleared successfully')
  } catch (e) {
    console.error('[AgentView] Failed to clear conversation:', e)
    localError.value = t('agent.failedToClearConversation') + ': ' + e
  }
}

// Handle stop
const handleStop = async () => {
  console.log('[AgentView] Stop requested for conversation:', conversationId.value)
  
  if (!conversationId.value) {
    console.warn('[AgentView] No conversation ID to stop')
    return
  }
  
  try {
    // Call backend to cancel command
    await invoke('cancel_ai_stream', {
      conversationId: conversationId.value
    })
    
    console.log('[AgentView] Stop command sent successfully')
    
    // Notify useAgentEvents to stop execution status
    agentEvents.stopExecution()
    
  } catch (e) {
    console.error('[AgentView] Failed to stop execution:', e)
    localError.value = t('agent.failedToStopExecution') + ': ' + e
  }
}

// Handle resend message - resend user message, delete all messages after it
const handleResendMessage = async (message: AgentMessage) => {
  if (isExecuting.value) {
    console.log('[AgentView] Cannot resend while executing')
    return
  }

  console.log('[AgentView] Resending message:', message.id, message.content)
  
  // Find the position of the message in the list
  const messageIndex = messages.value.findIndex(m => m.id === message.id)
  if (messageIndex === -1) {
    console.error('[AgentView] Message not found')
    return
  }

  // Delete all messages after this message (keep the user message, delete LLM response and subsequent messages)
  const messagesToKeep = messages.value.slice(0, messageIndex)
  agentEvents.messages.value = messagesToKeep

  // Set user message content to input box
  inputValue.value = message.content

  // Auto trigger send
  await handleSubmit()
}

// Load conversation history
const loadConversationHistory = async (convId: string) => {
  console.log('[AgentView] Loading conversation history for:', convId)
  try {
    const messages = await invoke<any[]>('get_ai_messages_by_conversation', {
      conversationId: convId
    })
    
    console.log('[AgentView] Received messages:', messages)
    
    // Clear current messages
    agentEvents.clearMessages()

    if (messages && messages.length > 0) {
      // DB already returns messages ordered by timestamp ASC.
      // For tool-enabled runs, backend persists assistant segments and tool events as standalone ai_messages rows,
      // so we can just render them in order (with a stable tie-breaker).
      const timeline: Array<{ msg: AgentMessage; ts: number; tie: number }> = []
      let tie = 0

      const toMillis = (v: any) => {
        const ms = new Date(v).getTime()
        return Number.isFinite(ms) ? ms : Date.now()
      }

      messages.forEach((row: any) => {
        const parsedMetadata =
          row.metadata && typeof row.metadata === 'string' ? JSON.parse(row.metadata) : row.metadata
        const ts = toMillis(row.timestamp)

        if (row.role === 'tool') {
          // Persisted tool event message
          const kind = parsedMetadata?.kind
          const type = kind === 'tool_result' ? 'tool_result' : 'tool_call'
          timeline.push({
            msg: {
              id: row.id,
              type: type as any,
              content: row.content || '',
              timestamp: ts,
              metadata: parsedMetadata,
            },
            ts,
            tie: tie++,
          })
          return
        }

        const messageType = row.role === 'user' ? 'user' : 'final'
        timeline.push({
          msg: {
            id: row.id,
            type: messageType as any,
            content: row.content,
            timestamp: ts,
            metadata: parsedMetadata,
          },
          ts,
          tie: tie++,
        })

        // Legacy fallback: assistant rows may contain tool_calls (older data). Keep the old behavior but don't reorder.
        if (row.role === 'assistant' && row.tool_calls) {
          try {
            const toolCalls = typeof row.tool_calls === 'string' ? JSON.parse(row.tool_calls) : row.tool_calls
            if (Array.isArray(toolCalls) && toolCalls.length > 0) {
              toolCalls.forEach((tc: any, i: number) => {
                let parsedArgs: any = {}
                try {
                  parsedArgs = typeof tc.arguments === 'string' ? JSON.parse(tc.arguments) : (tc.arguments ?? {})
                } catch {
                  parsedArgs = { raw: tc.arguments }
                }
                timeline.push({
                  msg: {
                    id: `toolcall:${row.id}:${tc.id || i}`,
                    type: 'tool_call' as any,
                    content: `${t('agent.toolCallCompleted')}: ${tc.name || 'unknown'}`,
                    timestamp: ts + i + 1,
                    metadata: {
                      tool_name: tc.name,
                      tool_args: parsedArgs,
                      tool_result: tc.result,
                      tool_call_id: tc.id,
                      status: 'completed',
                      success: tc.success !== false,
                    },
                  },
                  ts: ts + i + 1,
                  tie: tie++,
                })
              })
            }
          } catch (e) {
            console.warn('[AgentView] Failed to parse legacy tool_calls:', e)
          }
        }
      })

      timeline.sort((a, b) => a.ts - b.ts || a.tie - b.tie)
      agentEvents.messages.value = timeline.map(x => x.msg)

      console.log('[AgentView] Loaded', messages.length, 'messages from conversation:', convId)
    } else {
      console.log('[AgentView] No messages found for conversation:', convId)
    }
  } catch (e) {
    console.error('[AgentView] Failed to load conversation history:', e)
  }
}

// Handle conversation selection
const handleSelectConversation = async (convId: string) => {
  conversationId.value = convId
  await loadConversationHistory(convId)
  
  // Update conversation title
  try {
    const conversations = await invoke<any[]>('get_ai_conversations')
    const conv = conversations.find(c => c.id === convId)
    if (conv) {
      currentConversationTitle.value = conv.title || t('agent.unnamedConversation')
    }
  } catch (e) {
    console.error('[AgentView] Failed to get conversation title:', e)
  }
  
  // Auto close drawer after selecting conversation
  showConversations.value = false
}

// Handle conversation creation
const handleCreateConversation = async (newConvId?: string) => {
  if (newConvId) {
    conversationId.value = newConvId
    currentConversationTitle.value = t('agent.newConversationTitle')
    agentEvents.clearMessages()
  } else {
    // Create new conversation
    try {
      const convId = await invoke<string>('create_ai_conversation', {
        request: {
          title: `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
          service_name: 'default'
        }
      })
      conversationId.value = convId
      currentConversationTitle.value = t('agent.newConversationTitle')
      agentEvents.clearMessages()
      
      // Refresh conversation list
      conversationListRef.value?.loadConversations()
    } catch (e) {
      console.error('[AgentView] Failed to create conversation:', e)
    }
  }
}

// Handle submit
const handleSubmit = async () => {
  const task = inputValue.value.trim()
  if (!task) return
  
  localError.value = null
  
  // Takeover: if currently executing, cancel previous stream first
  if (isExecuting.value && conversationId.value) {
    try {
      const partial = agentEvents.streamingContent.value?.trim()
      if (partial) {
        const partialMsgId = crypto.randomUUID()
        // First solidify the partial output of the old stream locally as the final assistant message
        agentEvents.messages.value.push({
          id: partialMsgId,
          type: 'final' as any,
          content: partial,
          timestamp: Date.now(),
        })
        // Then write to database to ensure conversation history consistency
        await invoke('save_ai_message', {
          request: {
            id: partialMsgId,
            conversation_id: conversationId.value,
            role: 'assistant',
            content: partial,
          },
        })
      }
      await handleStop()
    } catch (e) {
      console.warn('[AgentView] Takeover stop failed, continuing:', e)
    }
  }

  // Build full task with traffic context
  let fullTask = task
  let displayContent: string | undefined = undefined
  if (referencedTraffic.value.length > 0) {
    const trafficContext = buildTrafficContext(referencedTraffic.value)
    fullTask = `${trafficContext}\n\nUser task: ${task}`
    // Display content is just the user's original input
    displayContent = task
  }
  
  // Clear input and references
  inputValue.value = ''
  const usedAttachments = [...pendingAttachments.value]
  const usedTraffic = [...referencedTraffic.value]
  pendingAttachments.value = []
  referencedTraffic.value = []
  
  // Emit submit event
  emit('submit', fullTask)
  
  try {
    // Ensure conversation exists or create new one
    if (!conversationId.value) {
      console.log('[AgentView] No conversation ID, creating new conversation')
      const convId = await invoke<string>('create_ai_conversation', {
        request: {
          title: `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
          service_name: 'default'
        }
      })
      conversationId.value = convId
      currentConversationTitle.value = t('agent.newConversationTitle')
      console.log('[AgentView] Created new conversation:', convId)
      
      // Refresh conversation list
      conversationListRef.value?.loadConversations()
    }
    
    // Call agent_execute command (tool config passed directly from frontend to ensure latest config takes effect immediately)
    const result = await invoke('agent_execute', {
      task: fullTask,
      config: {
        max_iterations: 10,
        timeout_secs: 300,
        force_todos: props.showTodos,
        enable_rag: ragEnabled.value,
        enable_web_search: webSearchEnabled.value,
        conversation_id: conversationId.value,
        message_id: null,
        attachments: usedAttachments.length > 0 ? usedAttachments : undefined,
        tool_config: toolConfig.value,
        display_content: displayContent,
      }
    })
    
    emit('complete', result)
  } catch (e: any) {
    const errorMsg = e.toString()
    localError.value = errorMsg
    emit('error', errorMsg)
  }
  // isExecuting and isStreaming are automatically managed by useAgentEvents, no manual setting needed
}

// Load tool config from database
const loadToolConfig = async () => {
  try {
    const savedConfig = await invoke<any>('get_tool_config')
    if (savedConfig) {
      toolConfig.value = savedConfig
      toolsEnabled.value = savedConfig.enabled
      console.log('[AgentView] Loaded tool config from database:', savedConfig)
    } else {
      console.log('[AgentView] No saved tool config found, using defaults')
    }
  } catch (e) {
    console.error('[AgentView] Failed to load tool config:', e)
  }
}

// Load latest conversation on startup
const loadLatestConversation = async () => {
  try {
    const conversations = await invoke<any[]>('get_ai_conversations')
    if (conversations && conversations.length > 0) {
      // Sort by update time in reverse order, take the latest conversation
      const sorted = conversations.sort((a, b) => 
        new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
      )
      const latest = sorted[0]
      conversationId.value = latest.id
      currentConversationTitle.value = latest.title || t('agent.unnamedConversation')
      await loadConversationHistory(latest.id)
      console.log('[AgentView] Loaded latest conversation:', latest.id)
    }
  } catch (e) {
    console.error('[AgentView] Failed to load latest conversation:', e)
  }
}

// Initialize
onMounted(async () => {
  console.log('[AgentView] Mounted with executionId:', props.executionId)
  
  // Load saved tool configuration from database
  await loadToolConfig()
  
  // Load conversation history if executionId is provided
  if (props.executionId) {
    conversationId.value = props.executionId
    await loadConversationHistory(props.executionId)
  } else {
    // Default load the last conversation
    await loadLatestConversation()
  }
})

// Watch for conversation changes to update title
watch(conversationId, async (newId) => {
  if (newId) {
    try {
      const conversations = await invoke<any[]>('get_ai_conversations')
      const conv = conversations.find(c => c.id === newId)
      if (conv) {
        currentConversationTitle.value = conv.title || t('agent.unnamedConversation')
      }
    } catch (e) {
      console.error('[AgentView] Failed to update conversation title:', e)
    }
  }
})

// Expose methods
defineExpose({
  clearMessages: agentEvents.clearMessages,
  scrollToBottom: () => messageFlowRef.value?.scrollToBottom(),
  addReferencedTraffic,
  loadConversationHistory,
  conversationId,
})
</script>

<style scoped>
.agent-view {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

/* Conversation Drawer Styles */
.conversation-backdrop {
  animation: fadeIn 0.2s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

/* Drawer Slide Animation */
.slide-drawer-enter-active,
.slide-drawer-leave-active {
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-drawer-enter-from {
  transform: translateX(-100%);
}

.slide-drawer-leave-to {
  transform: translateX(-100%);
}

/* Drawer Slide Animation Right */
.slide-drawer-right-enter-active,
.slide-drawer-right-leave-active {
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-drawer-right-enter-from,
.slide-drawer-right-leave-to {
  transform: translateX(100%);
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

  .conversation-drawer {
    width: 85vw !important;
    max-width: 320px;
  }
}
</style>
