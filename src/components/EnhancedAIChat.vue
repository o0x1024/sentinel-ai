<template>
  <div class="enhanced-ai-chat w-full h-full flex flex-col bg-gradient-to-br from-base-100 to-base-200 overflow-hidden">
    <!-- Messages Area -->
    <div ref="messagesContainer" class="flex-1 overflow-y-auto p-4 space-y-4 min-h-0 max-w-full">
      <!-- Welcome Message -->
      <div v-if="messages.length === 0" class="flex justify-center items-center h-full">
        <div class="text-center">
          <div class="avatar placeholder mb-4">
            <div class="bg-primary text-primary-content rounded-full w-16 flex items-center justify-center">
              <i class="fas fa-brain text-2xl"></i>
            </div>
          </div>
          <h3 class="text-lg font-semibold mb-2">{{ t('aiAssistant.welcome.title', 'AI智能助手') }}</h3>
          <p class="text-base-content/70 max-w-md">
            {{ t('aiAssistant.welcome.description', '我是您的AI安全助手，可以帮您执行安全扫描、漏洞分析等任务。请告诉我您需要什么帮助？') }}
          </p>
        </div>
      </div>

      <!-- Message List -->
      <div v-for="message in messages" :key="message.id" 
           :class="['chat', message.role === 'user' ? 'chat-end' : 'chat-start', 'mb-4']">
        <div class="chat-image">
          <div class="w-10 h-8 rounded-full shadow-lg border-2 border-base-300 bg-base-100 flex items-center justify-center">
            <svg v-if="message.role === 'user'" class="w-6 h-6 text-primary flex-shrink-0" fill="currentColor" viewBox="0 0 24 24">
              <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
            </svg>
            <svg v-else class="w-6 h-6 text-secondary flex-shrink-0" fill="currentColor" viewBox="0 0 24 24">
              <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.94-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"/>
            </svg>
          </div>
        </div>
        
        <div class="chat-header mb-2">
          <span class="font-medium text-sm text-base-content/80">
            {{ message.role === 'user' ? t('common.you', '您') : t('common.assistant', 'AI助手') }}
          </span>
          <!-- 显示选择的架构 -->
          <span v-if="message.selectedArchitecture && message.role === 'assistant'" 
                class="text-xs text-accent ml-2 px-2 py-0.5 bg-accent/10 rounded-full">
            <i class="fas fa-cogs mr-1"></i>
            {{ getArchitectureName(message.selectedArchitecture) }}
          </span>
          <time class="text-xs text-base-content/60 ml-2 px-2 py-0.5 bg-base-200 rounded-full">
            {{ formatTime(message.timestamp) }}
          </time>
        </div>
        
        <div :class="[
          'chat-bubble max-w-[85%] shadow-sm border transition-all duration-200',
          message.role === 'user' 
            ? 'bg-base-100 text-primary-content border-primary/20' 
            : 'bg-base-100 text-base-content border-base-300 hover:border-base-400'
        ]">
          <!-- Message Content Display -->
          <MessageContentDisplay 
            :message="message"
            :displayed-content="getDisplayedTypewriterContent(message.id)"
            :is-typing="isMessageTyping(message.id)"
            :stream-char-count="streamCharCount"
            :stream-speed="getStreamSpeed()"
            @skip-typewriter="skipTypewriter(message.id)"
          />
          
          <!-- Tool Executions Display (Real-time Progress) -->
          <ToolExecutionsDisplay 
            v-if="message.toolExecutions?.length || message.isStreaming"
            :tool-executions="message.toolExecutions || []"
            :is-streaming="message.isStreaming"
            :current-step="message.currentStep"
          />
          
          <!-- Execution Plan Display -->
          <ExecutionPlanDisplay 
            v-if="message.executionPlan"
            :execution-plan="message.executionPlan"
            :execution-progress="message.executionProgress"
            :current-step="message.currentStep"
            :is-streaming="message.isStreaming"
          />
          
          <!-- Execution Result Display -->
          <ExecutionResultDisplay 
            v-if="message.executionResult"
            :execution-result="message.executionResult"
            :message="message"
          />
          
          <!-- Error Actions -->
          <div v-if="message.hasError && message.role === 'assistant'" class="mt-3 flex gap-2 flex-wrap">
            <button @click="retryLastMessage" class="btn btn-sm btn-outline btn-primary">
              <i class="fas fa-redo"></i>
              重新发送
            </button>
            <button @click="clearErrorMessage(message)" class="btn btn-sm btn-outline btn-ghost">
              <i class="fas fa-times"></i>
              清除错误
            </button>
            <button 
              v-if="isConfigError(message.content)"
              @click="openAiSettings" 
              class="btn btn-sm btn-outline btn-warning"
            >
              <i class="fas fa-cog"></i>
              打开AI设置
            </button>
          </div>
          
        </div>
      </div>
    </div>

    <!-- Input Area -->
    <InputAreaComponent
      v-model:input-message="inputMessage"
      :is-loading="isLoading"
      :show-debug-info="showDebugInfo"
      :selected-architecture="selectedArchitecture"
      :available-architectures="getAvailableArchitectures()"
      :conversations="conversations"
      :current-conversation-id="currentConversationId"
      :is-loading-conversations="isLoadingConversations"
      :show-conversations-list="showConversationsList"
      :is-task-mode="isTaskMode"
      @send-message="sendMessage"
      @stop-execution="stopExecution"
      @toggle-debug="showDebugInfo = !showDebugInfo"
      @toggle-mode="isTaskMode = $event"
      @select-architecture="selectArchitecture"
      @create-conversation="createNewConversation"
      @load-conversations="loadConversations"
      @switch-conversation="switchToConversation"
      @delete-conversation="deleteConversation"
      @clear-conversation="clearCurrentConversation"
      @update:show-conversations-list="showConversationsList = $event"
    />

    <!-- Step Detail Dialog -->
    <dialog :class="['modal', { 'modal-open': stepDetailVisible }]">
      <div class="modal-box max-w-4xl max-h-[90vh] overflow-y-auto">
        <div class="flex justify-between items-center mb-4">
          <h3 class="font-bold text-lg">步骤详情</h3>
          <button class="btn btn-sm btn-circle btn-ghost" @click="closeStepDetail">✕</button>
        </div>
        
        <StepDetailDisplay 
          v-if="selectedStepDetail"
          :step="selectedStepDetail"
        />
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click="closeStepDetail">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

// Composables
import { useTypewriter } from '../composables/useTypewriter'
import { useConversation } from '../composables/useConversation'
import { useMessageUtils } from '../composables/useMessageUtils'
import { useEventListeners } from '../composables/useEventListeners'

// Components
import MessageContentDisplay from './MessageParts/MessageContentDisplay.vue'
import ExecutionPlanDisplay from './MessageParts/ExecutionPlanDisplay.vue'
import ToolExecutionsDisplay from './MessageParts/ToolExecutionsDisplay.vue'
import ExecutionResultDisplay from './MessageParts/ExecutionResultDisplay.vue'
import InputAreaComponent from './InputAreaComponent.vue'
import StepDetailDisplay from './MessageParts/StepDetailDisplay.vue'

// Types
interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: Date
  isStreaming?: boolean
  hasError?: boolean
  executionPlan?: any
  toolExecutions?: any[]
  executionResult?: any
  executionProgress?: number
  currentStep?: string
  totalSteps?: number
  completedSteps?: number
  selectedArchitecture?: string
}

interface DispatchResult {
  execution_id: string
  initial_response?: string
  execution_plan?: {
    name?: string
    steps?: any[]
  }
}

// Props and Emits
const props = defineProps<{
  selectedArchitecture: string
  selectedAgent?: any
  availableArchitectures?: any[]
}>()

const emit = defineEmits(['execution-started', 'execution-progress', 'execution-completed', 'architecture-changed'])

const { t } = useI18n()
const router = useRouter()

// Use composables
const {
  enableTypewriter,
  typewriterSpeed,
  updateTypewriterContentIncremental,
  updateTypewriterContent,
  stopTypewriter,
  skipTypewriter,
  getDisplayedTypewriterContent,
  isMessageTyping,
  getFinalContentFromTypewriterState,
  getTypewriterMode,
  getTypewriterProgress,
  cleanupTypewriter
} = useTypewriter()

const {
  conversations,
  currentConversationId,
  isLoadingConversations,
  messages,
  createNewConversation,
  loadConversations,
  switchToConversation,
  deleteConversation,
  clearCurrentConversation,
  saveMessagesToConversation,
  getCurrentConversationTitle,
  restoreSessionState
} = useConversation()

const { formatTime } = useMessageUtils()

// Local state
const inputMessage = ref('')
const isLoading = ref(false)
const messagesContainer = ref<HTMLElement | null>(null)
const currentExecutionId = ref<string | null>(null)
const streamStartTime = ref<number | null>(null)
const streamCharCount = ref(0)
const showDebugInfo = ref(false)
const showConversationsList = ref(false)
const stepDetailVisible = ref(false)
const selectedStepDetail = ref<any>(null)
const loadingTimeoutId = ref<number | null>(null)
const isTaskMode = ref(false)

// Timeout mechanism to reset loading state
const resetLoadingWithTimeout = (timeoutMs = 30000) => { // 30 seconds timeout
  if (loadingTimeoutId.value) {
    clearTimeout(loadingTimeoutId.value)
  }
  
  loadingTimeoutId.value = window.setTimeout(() => {
    if (isLoading.value) {
      console.warn('Loading state timeout reached, forcing reset')
      isLoading.value = false
      streamStartTime.value = null
      streamCharCount.value = 0
      
      // Also stop any active typewriter
      const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
      if (lastAssistantMessage && lastAssistantMessage.isStreaming) {
        stopTypewriter(lastAssistantMessage.id)
        lastAssistantMessage.isStreaming = false
        lastAssistantMessage.content += '\n\n[响应超时]'
      }
    }
    loadingTimeoutId.value = null
  }, timeoutMs)
}

const clearLoadingTimeout = () => {
  if (loadingTimeoutId.value) {
    clearTimeout(loadingTimeoutId.value)
    loadingTimeoutId.value = null
  }
}

// Define scrollToBottom function before using in event listeners
const scrollToBottom = () => {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
    }
  })
}

// Event listeners setup with optimized stream handling
const eventListeners = useEventListeners(
  messages,
  currentExecutionId,
  currentConversationId,
  streamStartTime,
  streamCharCount,
  { updateTypewriterContentIncremental, updateTypewriterContent, stopTypewriter, getFinalContentFromTypewriterState },
  {
    'execution-started': emit,
    'execution-progress': emit,
    'execution-completed': emit,
    'architecture-changed': emit,
    'stream-completed': (data: any) => {
      console.log('Stream completed event received:', data)
      clearLoadingTimeout()
      isLoading.value = false
      streamStartTime.value = null
      streamCharCount.value = 0
      
      // Check for empty response and show helpful error message
      if (data.total_content_length === 0 || data.error) {
        const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
        if (lastAssistantMessage && (!lastAssistantMessage.content || lastAssistantMessage.content.trim().length === 0)) {
          lastAssistantMessage.hasError = true
          console.warn('Detected empty response in stream completion')
        }
      }
    },
    'stream-error': (data: any) => {
      console.log('Stream error event received:', data)
      clearLoadingTimeout()
      isLoading.value = false
      streamStartTime.value = null
      streamCharCount.value = 0
      
      // Find and mark the target message as having an error
      const targetMessage = data.messageId 
        ? messages.value.find(m => m.id === data.messageId)
        : messages.value.filter(m => m.role === 'assistant').pop()
      
      if (targetMessage) {
        targetMessage.hasError = true
        targetMessage.isStreaming = false
      }
    },
    'task-completed': (data: any) => {
      console.log('Task completed event received:', data)
      clearLoadingTimeout()
      isLoading.value = false
      streamStartTime.value = null
      streamCharCount.value = 0
      currentExecutionId.value = null
    },
    'task-error': (data: any) => {
      console.log('Task error event received:', data)
      clearLoadingTimeout()
      isLoading.value = false
      streamStartTime.value = null
      streamCharCount.value = 0
      currentExecutionId.value = null
    }
  },
  scrollToBottom,
  saveMessagesToConversation
)

// Methods
const sendMessage = async () => {
  if (!inputMessage.value.trim() || isLoading.value) return

  const userMessage: ChatMessage = {
    id: Date.now().toString(),
    role: 'user',
    content: inputMessage.value,
    timestamp: new Date()
  }
  
  messages.value.push(userMessage)
  const userInput = inputMessage.value
  inputMessage.value = ''
  isLoading.value = true
  
  // Start timeout mechanism
  resetLoadingWithTimeout()

  const assistantMessage: ChatMessage = {
    id: `assistant_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
    role: 'assistant',
    content: '',
    timestamp: new Date(),
    isStreaming: true,
    executionPlan: null,
    toolExecutions: [],
    executionResult: null,
    executionProgress: 0,
    currentStep: undefined,
    totalSteps: 0,
    completedSteps: 0
  }
  messages.value.push(assistantMessage)
  
  await nextTick()
  scrollToBottom()

  try {
    // Ensure current conversation exists
    if (!currentConversationId.value) {
      await createNewConversation()
    }
    
    // Handle based on user-selected mode
    if (isTaskMode.value) {
      // Task mode - execute tasks with agent execution
      if (!currentConversationId.value) {
        await createNewConversation()
      }
      
      // Generate unique execution ID
      const executionId = `exec_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
      currentExecutionId.value = executionId
      
      // Start task execution streaming
      assistantMessage.content = '正在生成执行计划...'
      assistantMessage.executionPlan = null
      assistantMessage.currentStep = '计划生成'
      
      try {
        await invoke('dispatch_intelligent_query', {
          request: {
            query: userInput,
            architecture: 'auto', // 让系统自动选择架构
            agent_id: null,
            options: {
              conversation_id: currentConversationId.value,
              message_id: assistantMessage.id,
              execution_id: executionId,
              task_mode: true
            }
          }
        })
        
        emit('execution-started', {
          id: executionId,
          name: '智能任务执行',
          description: userInput,
          progress: 0,
          status: 'running'
        })
        
        // Note: Don't reset isLoading here - let the task events handle it
      } catch (taskError) {
        console.error('Failed to start intelligent task execution:', taskError)
        assistantMessage.content = `智能任务调度失败: ${taskError}`
        assistantMessage.hasError = true
        assistantMessage.isStreaming = false
        clearLoadingTimeout()
        isLoading.value = false
        streamStartTime.value = null
        streamCharCount.value = 0
      }
    } else {
      // Chat mode - normal conversation
      streamStartTime.value = Date.now()
      streamCharCount.value = 0
      
      try {
        await invoke('send_ai_stream_message', {
          request: {
            conversation_id: currentConversationId.value,
            message: userInput,
            service_name: 'default',
            message_id: assistantMessage.id
          }
        })
        // Note: Don't reset isLoading here - let the stream events handle it
      } catch (streamError) {
        console.error('Failed to start streaming:', streamError)
        assistantMessage.content = `启动流式响应失败: ${streamError}`
        assistantMessage.hasError = true
        assistantMessage.isStreaming = false
        clearLoadingTimeout()
        isLoading.value = false
        streamStartTime.value = null
        streamCharCount.value = 0
      }
    }

  } catch (error) {
    console.error('Failed to send message:', error)
    assistantMessage.content = `${t('aiAssistant.error', '错误')}: ${error}`
    assistantMessage.isStreaming = false
    assistantMessage.hasError = true
    clearLoadingTimeout()
    isLoading.value = false
    streamStartTime.value = null
    streamCharCount.value = 0
  }
}

const stopExecution = async () => {
  if (currentExecutionId.value) {
    try {
      await invoke('stop_execution', {
        execution_id: currentExecutionId.value
      })
    } catch (error) {
      console.error('Failed to stop execution:', error)
    }
  }
  
  if (currentConversationId.value) {
    try {
      await invoke('cancel_ai_stream', {
        conversationId: currentConversationId.value
      })
    } catch (error) {
      console.error('Failed to cancel stream:', error)
    }
  }
  
  const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
  if (lastAssistantMessage && lastAssistantMessage.isStreaming) {
    stopTypewriter(lastAssistantMessage.id)
    lastAssistantMessage.isStreaming = false
    lastAssistantMessage.content += '\n\n[用户中断了响应]'
  }
  
  // Always reset loading state when stopping
  clearLoadingTimeout()
  isLoading.value = false
  streamStartTime.value = null
  streamCharCount.value = 0
}

const retryLastMessage = () => {
  const userMessages = messages.value.filter(m => m.role === 'user')
  if (userMessages.length > 0) {
    const lastUserMessage = userMessages[userMessages.length - 1]
    inputMessage.value = lastUserMessage.content
    sendMessage()
  }
}

const clearErrorMessage = (message: ChatMessage) => {
  message.hasError = false
  message.content = '[已清除错误消息]'
}

const openAiSettings = () => {
  router.push('/settings?tab=ai')
}

const getStreamSpeed = () => {
  if (!streamStartTime.value || streamCharCount.value === 0) return 0
  const elapsed = (Date.now() - streamStartTime.value) / 1000
  return Math.round(streamCharCount.value / elapsed)
}

const selectArchitecture = (architecture: any) => {
  emit('architecture-changed', architecture)
}

const getAvailableArchitectures = () => {
  if (props.availableArchitectures && props.availableArchitectures.length > 0) {
    return props.availableArchitectures
  }
  
  return [
    {
      id: 'plan-execute',
      name: 'Plan-and-Execute',
      description: '计划执行架构：先制定计划，再逐步执行',
      status: 'stable'
    },
    {
      id: 'rewoo',
      name: 'ReWOO',
      description: '推理无观察架构：减少工具调用的推理方法',
      status: 'beta'
    },
    {
      id: 'llm-compiler',
      name: 'LLMCompiler',
      description: 'LLM编译器：并行执行任务的先进架构',
      status: 'experimental'
    },
    {
      id: 'intelligent-dispatcher',
      name: 'Intelligent Dispatcher',
      description: '智能调度器：AI驱动的智能任务分发',
      status: 'ai-powered'
    }
  ]
}
const mapArchitectureToId = (architectureName: string) => {
  const mapping: Record<string, string> = {
    'Plan-and-Execute': 'plan-execute',
    'ReWOO': 'rewoo',
    'LLMCompiler': 'llm-compiler',
    'Intelligent Dispatcher': 'intelligent-dispatcher'
  }
  return mapping[architectureName] || 'plan-execute'
}

const isConfigError = (content: string) => {
  return content.includes('配置') || 
         content.includes('API') || 
         content.includes('provider') ||
         content.includes('not configured') ||
         content.includes('空响应') ||
         content.includes('configuration')
}

const getArchitectureName = (architecture: string) => {
  const architectureNames: Record<string, string> = {
    'intelligent-dispatcher': 'Intelligent Dispatcher',
    'plan-execute': 'Plan-and-Execute',
    'rewoo': 'ReWOO',
    'llm-compiler': 'LLM Compiler'
  }
  return architectureNames[architecture] || architecture
}

// Step detail methods
const closeStepDetail = () => {
  stepDetailVisible.value = false
  selectedStepDetail.value = null
}

// Debug methods
const testIncrementalTypewriter = (messageId: string) => {
  console.log('Testing incremental typewriter for message:', messageId)
  
  const testChunks = [
    'Hello ',
    'this ',
    'is ',
    'a ',
    'test ',
    'of ',
    'incremental ',
    'typewriter ',
    'functionality!'
  ]
  
  const message = messages.value.find(m => m.id === messageId)
  if (message) {
    message.content = ''
    message.isStreaming = true
    
    stopTypewriter(messageId)
    
    testChunks.forEach((chunk, index) => {
      setTimeout(() => {
        updateTypewriterContentIncremental(messageId, chunk)
        
        if (index === testChunks.length - 1) {
          setTimeout(() => {
            message.isStreaming = false
            console.log('Test completed')
          }, 500)
        }
      }, index * 200)
    })
  }
}

const debugDisplayState = (messageId: string) => {
  const message = messages.value.find(m => m.id === messageId)
  console.log('=== Display State Debug ===', {
    messageId,
    messageExists: !!message,
    messageContent: message?.content || 'N/A',
    messageIsStreaming: message?.isStreaming,
    displayedContent: getDisplayedTypewriterContent(messageId),
    isMessageTyping: isMessageTyping(messageId)
  })
}

const debugTypewriterState = (messageId: string) => {
  console.log('=== Typewriter Debug Info ===')
  console.log('Message ID:', messageId)
  console.log('Typewriter Mode:', getTypewriterMode(messageId))
  console.log('Typewriter Progress:', getTypewriterProgress(messageId))
  console.log('EnableTypewriter:', enableTypewriter.value)
  console.log('TypewriterSpeed:', typewriterSpeed.value)
}

// Lifecycle
onMounted(async () => {
  restoreSessionState()
  await loadConversations()
  await eventListeners.setupEventListeners()
})

onUnmounted(() => {
  clearLoadingTimeout()
  cleanupTypewriter()
  eventListeners.cleanup()
})
</script>

<style scoped>
.enhanced-ai-chat {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  position: relative;
  overflow: hidden;
  max-width: 100vw;
  max-height: 100vh;
  box-sizing: border-box;
}

.chat {
  animation: fadeInUp 0.3s ease-out;
}

@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.streaming-content {
  position: relative;
}

.streaming-content::after {
  content: '|';
  animation: typewriter-cursor 1s infinite;
  color: hsl(var(--p));
  font-weight: bold;
  margin-left: 2px;
  display: inline-block;
}

@keyframes typewriter-cursor {
  0%, 50% {
    opacity: 1;
  }
  51%, 100% {
    opacity: 0;
  }
}

.typewriter-text {
  animation: typewriter-reveal 0.05s ease-out;
  transition: all 0.1s ease;
}

.typewriter-text:hover {
  background-color: hsl(var(--b3) / 0.3);
  border-radius: 0.375rem;
  padding: 0.125rem 0.25rem;
  margin: -0.125rem -0.25rem;
}

.typewriter-text:hover::after {
  content: ' ✋ 点击跳过';
  font-size: 0.75rem;
  opacity: 0.7;
  color: hsl(var(--bc) / 0.6);
  background: hsl(var(--b1));
  padding: 0.125rem 0.375rem;
  border-radius: 0.25rem;
  margin-left: 0.5rem;
  animation: fadeIn 0.2s ease-in;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 0.7; }
}

@keyframes typewriter-reveal {
  from {
    opacity: 0.7;
  }
  to {
    opacity: 1;
  }
}

.chat-bubble {
  transition: all 0.2s ease;
}

.chat:hover .chat-bubble {
  transform: translateY(-1px);
  box-shadow: 0 8px 25px rgba(0, 0, 0, 0.1);
}

.btn {
  transition: all 0.2s ease;
}

.btn:hover:not(.btn-disabled) {
  transform: translateY(-1px);
}

.enhanced-ai-chat {
  background: linear-gradient(135deg, hsl(var(--b1)) 0%, hsl(var(--b2)) 100%);
  background-size: 200% 200%;
  animation: gradientShift 20s ease infinite;
}

@keyframes gradientShift {
  0% {
    background-position: 0% 50%;
  }
  50% {
    background-position: 100% 50%;
  }
  100% {
    background-position: 0% 50%;
  }
}

@media (max-width: 768px) {
  .enhanced-ai-chat .chat-bubble {
    max-width: calc(100vw - 8rem);
    word-wrap: break-word;
    overflow-wrap: break-word;
  }
}
</style>