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
      <!-- Conversation Header -->
      <div class="conversation-header px-4 py-2 border-b border-base-300 flex items-center justify-between bg-base-100/50">
        <div class="flex items-center gap-2">
          <button 
            @click="showConversations = !showConversations"
            class="btn btn-sm btn-ghost"
            title="切换会话列表"
          >
            <i class="fas fa-bars"></i>
          </button>
          <span class="text-sm font-medium text-base-content/70">
            {{ currentConversationTitle }}
          </span>
          
          <!-- RAG状态指示器 -->
          <div v-if="ragEnabled" class="flex items-center gap-1 px-2 py-1 bg-info/10 rounded-md border border-info/30">
            <i class="fas fa-book text-info text-xs"></i>
            <span class="text-xs text-info font-medium">知识库</span>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <button 
            @click="handleCreateConversation()"
            class="btn btn-sm btn-ghost gap-1"
            title="新建会话"
          >
            <i class="fas fa-plus"></i>
            新建
          </button>
        </div>
      </div>

      <!-- Messages and Todos -->
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
        <div class="sidebar-container w-80 flex-shrink-0 border-l border-base-300 flex flex-col overflow-hidden bg-base-100" v-if="isVisionActive || hasTodos">
            <VisionExplorerPanel 
               v-if="isVisionActive"
               :steps="visionEvents.steps.value" 
               :coverage="visionEvents.coverage.value"
               :discovered-apis="visionEvents.discoveredApis.value"
               :is-active="isVisionActive"
               :current-url="visionEvents.currentUrl.value"
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

      <!-- Input area - using InputAreaComponent for full features -->
      <InputAreaComponent
        v-model:input-message="inputValue"
        :is-loading="isExecuting"
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

      <!-- Error display -->
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
import { invoke } from '@tauri-apps/api/core'
import type { AgentMessage } from '@/types/agent'
import type { Todo } from '@/types/todo'
import { useAgentEvents } from '@/composables/useAgentEvents'
import { useVisionEvents } from '@/composables/useVisionEvents'
import { useTodos } from '@/composables/useTodos'
import MessageFlow from './MessageFlow.vue'
import TodoPanel from './TodoPanel.vue'
import VisionExplorerPanel from './VisionExplorerPanel.vue'
import InputAreaComponent from '@/components/InputAreaComponent.vue'
import ConversationList from './ConversationList.vue'
import ToolConfigPanel from './ToolConfigPanel.vue'

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
  selectedRole?: any
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
const conversationListRef = ref<InstanceType<typeof ConversationList> | null>(null)
const inputValue = ref('')
const localError = ref<string | null>(null)
const conversationId = ref<string | null>(null)
const showConversations = ref(false) // 默认隐藏
const showToolConfig = ref(false)
const currentConversationTitle = ref('新会话')

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
const visionEvents = useVisionEvents(computed(() => agentEvents.currentExecutionId.value || ''))
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
  
  // 保存工具配置到数据库（全局配置，不绑定到会话）
  try {
    await invoke('save_tool_config', {
      toolConfig: config
    })
    console.log('[AgentView] Tool config saved globally')
  } catch (e) {
    console.error('[AgentView] Failed to save tool config:', e)
    localError.value = '保存工具配置失败: ' + e
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
    
    // 调用后端清空会话消息
    await invoke('clear_conversation_messages', {
      conversationId: conversationId.value
    })
    
    // 清空前端消息
    agentEvents.clearMessages()
    
    // 清空附件和引用
    pendingAttachments.value = []
    referencedTraffic.value = []
    inputValue.value = ''
    
    // 刷新会话列表（更新消息计数）
    conversationListRef.value?.loadConversations()
    
    console.log('[AgentView] Conversation cleared successfully')
  } catch (e) {
    console.error('[AgentView] Failed to clear conversation:', e)
    localError.value = '清空会话失败: ' + e
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
    // 调用后端取消命令
    await invoke('cancel_ai_stream', {
      conversationId: conversationId.value
    })
    
    console.log('[AgentView] Stop command sent successfully')
    
    // 通知 useAgentEvents 停止执行状态
    agentEvents.stopExecution()
    
  } catch (e) {
    console.error('[AgentView] Failed to stop execution:', e)
    localError.value = '停止执行失败: ' + e
  }
}

// Handle resend message - 重新发送用户消息，删除该消息之后的所有消息
const handleResendMessage = async (message: AgentMessage) => {
  if (isExecuting.value) {
    console.log('[AgentView] Cannot resend while executing')
    return
  }

  console.log('[AgentView] Resending message:', message.id, message.content)
  
  // 找到该消息在列表中的位置
  const messageIndex = messages.value.findIndex(m => m.id === message.id)
  if (messageIndex === -1) {
    console.error('[AgentView] Message not found')
    return
  }

  // 删除该消息之后的所有消息（保留该用户消息，删除 LLM 响应和后续消息）
  const messagesToKeep = messages.value.slice(0, messageIndex)
  agentEvents.messages.value = messagesToKeep

  // 将用户消息内容设置到输入框
  inputValue.value = message.content

  // 自动触发发送
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
      // Convert database messages to AgentMessage format
      messages.forEach((msg: any) => {
        const parsedMetadata = msg.metadata && typeof msg.metadata === 'string' 
          ? JSON.parse(msg.metadata) 
          : msg.metadata
        
        // 解析 tool_calls 字段（如果存在）
        let toolCalls: any[] = []
        if (msg.tool_calls) {
          try {
            toolCalls = typeof msg.tool_calls === 'string' 
              ? JSON.parse(msg.tool_calls) 
              : msg.tool_calls
          } catch (e) {
            console.warn('[AgentView] Failed to parse tool_calls:', e)
          }
        }
        
        // 如果有工具调用，先添加工具调用消息
        if (toolCalls && toolCalls.length > 0) {
          toolCalls.forEach((tc: any) => {
            // 解析参数 JSON
            let parsedArgs: any = {}
            try {
              parsedArgs = typeof tc.arguments === 'string' 
                ? JSON.parse(tc.arguments) 
                : tc.arguments
            } catch (e) {
              parsedArgs = { raw: tc.arguments }
            }
            
            // 创建工具调用消息（带合并的结果）
            agentEvents.messages.value.push({
              id: tc.id || crypto.randomUUID(),
              type: 'tool_call' as any,
              content: `工具调用完成: ${tc.name}`,
              timestamp: new Date(msg.timestamp).getTime(),
              metadata: {
                tool_name: tc.name,
                tool_args: parsedArgs,
                tool_result: tc.result,
                tool_call_id: tc.id,
                status: 'completed',
                success: tc.success !== false,
              },
            })
          })
        }
        
        // 添加主消息（用户或助手）
        const messageType = msg.role === 'user' ? 'user' : 'final'
        agentEvents.messages.value.push({
          id: msg.id,
          type: messageType as any,
          content: msg.content,
          timestamp: new Date(msg.timestamp).getTime(),
          metadata: parsedMetadata,
        })
      })
      console.log('[AgentView] Loaded', messages.length, 'messages with tool calls from conversation:', convId)
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
      currentConversationTitle.value = conv.title || '未命名会话'
    }
  } catch (e) {
    console.error('[AgentView] Failed to get conversation title:', e)
  }
  
  // 选择会话后自动关闭抽屉
  showConversations.value = false
}

// Handle conversation creation
const handleCreateConversation = async (newConvId?: string) => {
  if (newConvId) {
    conversationId.value = newConvId
    currentConversationTitle.value = '新会话'
    agentEvents.clearMessages()
  } else {
    // Create new conversation
    try {
      const convId = await invoke<string>('create_ai_conversation', {
        request: {
          title: `新会话 ${new Date().toLocaleString()}`,
          service_name: 'default'
        }
      })
      conversationId.value = convId
      currentConversationTitle.value = '新会话'
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
  if (isExecuting.value || !task) return
  
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
  
  // Emit submit event
  emit('submit', fullTask)
  
  try {
    // Ensure conversation exists or create new one
    if (!conversationId.value) {
      console.log('[AgentView] No conversation ID, creating new conversation')
      const convId = await invoke<string>('create_ai_conversation', {
        request: {
          title: `新会话 ${new Date().toLocaleString()}`,
          service_name: 'default'
        }
      })
      conversationId.value = convId
      currentConversationTitle.value = '新会话'
      console.log('[AgentView] Created new conversation:', convId)
      
      // Refresh conversation list
      conversationListRef.value?.loadConversations()
    }
    
    // Call agent_execute command (工具配置直接从前端传递，确保最新配置立即生效)
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
      }
    })
    
    emit('complete', result)
  } catch (e: any) {
    const errorMsg = e.toString()
    localError.value = errorMsg
    emit('error', errorMsg)
  }
  // isExecuting 和 isStreaming 由 useAgentEvents 自动管理，不需要手动设置
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
      // 按更新时间倒序，取最新的会话
      const sorted = conversations.sort((a, b) => 
        new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
      )
      const latest = sorted[0]
      conversationId.value = latest.id
      currentConversationTitle.value = latest.title || '未命名会话'
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
    // 默认加载最后一次会话
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
        currentConversationTitle.value = conv.title || '未命名会话'
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
