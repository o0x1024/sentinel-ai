<template>
  <div
    class="enhanced-ai-chat w-full h-full flex flex-col bg-gradient-to-br from-base-100 to-base-200 overflow-hidden"
  >
    <!-- Messages Area -->
    <div ref="messagesContainer" class="flex-1 overflow-y-auto p-4 space-y-4 min-h-0 max-w-full">
      <!-- Welcome Message -->
      <div v-if="messages.length === 0" class="flex justify-center items-center h-full">
        <div class="text-center">
          <div class="avatar placeholder mb-4">
            <div
              class="bg-primary text-primary-content rounded-full w-16 flex items-center justify-center"
            >
              <i class="fas fa-brain text-2xl"></i>
            </div>
          </div>
          <h3 class="text-lg font-semibold mb-2">
            {{ t('aiAssistant.welcome.title', 'AI智能助手') }}
          </h3>
          <p class="text-base-content/70 max-w-md">
            {{
              t(
                'aiAssistant.welcome.description',
                '我是您的AI安全助手，可以帮您执行安全扫描、漏洞分析等任务。请告诉我您需要什么帮助？'
              )
            }}
          </p>
        </div>
      </div>

      <!-- Message List -->
      <div
        v-for="message in messages"
        :key="message.id"
        :class="['chat', message.role === 'user' ? 'chat-end' : 'chat-start', 'mb-4', 'group']"
      >
        <div class="chat-image">
          <div
            class="w-10 h-8 rounded-full shadow-lg border-2 border-base-300 bg-base-100 flex items-center justify-center"
          >
            <svg
              v-if="message.role === 'user'"
              class="w-6 h-6 text-primary flex-shrink-0"
              fill="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"
              />
            </svg>
            <svg
              v-else
              class="w-6 h-6 text-secondary flex-shrink-0"
              fill="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.94-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"
              />
            </svg>
          </div>
        </div>

        <div class="chat-header mb-2 flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="font-medium text-sm text-base-content/80">
              {{ message.role === 'user' ? t('common.you', '您') : t('common.assistant', 'AI助手') }}
            </span>
            <time class="text-xs text-base-content/60 px-2 py-0.5 bg-base-200 rounded-full">
              {{ formatTime(message.timestamp) }}
            </time>
          </div>
        </div>

        <div
          :class="[
            'chat-bubble max-w-[85%] shadow-sm border transition-all duration-200',
            message.role === 'user'
              ? 'bg-base-100 text-primary-content border-primary/20'
              : 'bg-base-100 text-base-content border-base-300 hover:border-base-400',
          ]"
        >
          <!-- ReAct 步骤显示 -->
          <div v-if="isReActMessage(message)" class="space-y-3">
            <ReActStepDisplay
              v-for="(step, index) in parseReActSteps(message.content, message.id)"
              :key="`react-step-${index}`"
              :step-data="step"
            />
          </div>

          <!-- 普通消息显示 - 统一使用 Markdown 渲染 -->
          <div v-else
            :class="[
              'prose prose-sm max-w-none leading-relaxed',
              message.role === 'user' ? 'prose-invert ' : 'prose-neutral'
            ]"
            v-html="renderMarkdown(message.content)"
          />

          <!-- 流式指示器 -->
          <div v-if="message.isStreaming" class="flex items-center gap-2 mt-2 text-base-content/70">
            <span class="loading loading-dots loading-sm text-primary"></span>
            <span class="text-sm">{{ t('aiAssistant.generating', 'AI正在思考...') }}</span>
          </div>

          <!-- Citations (引用来源) -->
          <div
            v-if="message.citations && message.citations.length && message.role === 'assistant'"
            class="mt-3 p-3 bg-base-200/50 rounded-lg border border-base-300/50"
          >
            <div class="flex items-center gap-2 mb-2">
              <i class="fas fa-quote-left text-xs text-accent"></i>
              <span class="text-xs font-medium text-base-content/80">参考来源 ({{ message.citations.length }})</span>
            </div>
            <div class="flex flex-wrap gap-2">
              <div
                v-for="(citation, index) in message.citations"
                :key="citation.id || citation.source_id || (citation.file_name + index)"
                class="group relative"
                :id="`source-${index + 1}`"
              >
                <button
                  @click="openCitationModal(citation)"
                  class="btn btn-xs btn-outline gap-1 hover:btn-accent transition-all duration-200"
                  :title="citation.file_name"
                >
                  <i class="fas fa-file-alt text-xs"></i>
                  <span class="text-xs">[{{ index + 1 }}] {{ (citation.file_name || '').split('/').pop() }}</span>
                </button>
              
              </div>
            </div>
          </div>

          <!-- Error Actions -->
          <div
            v-if="message.hasError && message.role === 'assistant'"
            class="mt-3 flex gap-2 flex-wrap"
          >
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

        <!-- User Message Actions - Outside the bubble -->
        <div
          v-if="message.role === 'user'"
          class="flex gap-2 justify-end mt-2 opacity-0 group-hover:opacity-100 transition-opacity duration-200"
        >
          <button 
            @click="copyMessage(message.content)" 
            class="btn btn-xs btn-ghost gap-1 text-base-content/60 hover:text-base-content"
            title="复制消息"
          >
            <i class="fas fa-copy text-xs"></i>
            <span class="text-xs">复制</span>
          </button>
          <button 
            @click="resendMessage(message.content)" 
            class="btn btn-xs btn-ghost gap-1 text-base-content/60 hover:text-base-content"
            title="重新发送"
          >
            <i class="fas fa-redo text-xs"></i>
            <span class="text-xs">重发</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Input Area -->
    <InputAreaComponent
      v-model:input-message="inputMessage"
      :is-loading="isLoading"
      :show-debug-info="showDebugInfo"
      :rag-enabled="ragEnabled"
      @send-message="sendMessage"
      @stop-execution="stopExecution"
        @toggle-debug="showDebugInfo = !showDebugInfo"
      @create-new-conversation="handleCreateNewConversation"
      @clear-conversation="handleClearConversation"
      @toggle-task-mode="handleToggleTaskMode"
      @toggle-rag="handleToggleRAG"
    />

    <!-- Citation Detail Modal -->
    <div v-if="citationModalOpen" class="modal modal-open">
      <div class="modal-box max-w-3xl">
        <h3 class="font-bold text-lg mb-2">参考来源详情</h3>
        <div v-if="citationDetail" class="space-y-2 text-sm">
          <div class="font-semibold">{{ citationDetail.file_name }}</div>
          <div class="text-base-content/70">
            源ID: {{ citationDetail.source_id }}
          </div>
          <div class="text-base-content/70">
            位置: {{ citationDetail.page_number ? `第${citationDetail.page_number}页` : '未知页' }}
            <span v-if="citationDetail.section_title"> · {{ citationDetail.section_title }}</span>
            <span> · {{ citationDetail.start_char }} - {{ citationDetail.end_char }}</span>
          </div>
          <div class="mt-2 p-3 bg-base-200/50 rounded border border-base-300/50 whitespace-pre-wrap break-words">
            {{ citationDetail.content_preview }}
          </div>
          <div class="text-xs text-base-content/60">相似度: {{ (citationDetail.score * 100).toFixed(1) }}%</div>
          <div class="mt-3 flex gap-2">
            <button class="btn btn-sm" @click="jumpToRagSource(citationDetail)">在知识库中查看</button>
            <button class="btn btn-sm btn-ghost" @click="citationModalOpen = false">关闭</button>
          </div>
        </div>
      </div>
      <div class="modal-backdrop" @click="citationModalOpen = false"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { getRagConfig, saveRagConfig } from '../services/rag_config'

// Composables
import { useConversation } from '../composables/useConversation'
import { useMessageUtils } from '../composables/useMessageUtils'
import { useOrderedMessages } from '../composables/useOrderedMessages'

// Components
import InputAreaComponent from './InputAreaComponent.vue'
import ReActStepDisplay from './MessageParts/ReActStepDisplay.vue'

// Types
import type { ChatMessage, Citation } from '../types/chat'
import { createUserMessage, createAssistantMessage } from '../composables/useOrderedMessages'

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
  selectedAgent?: any
  selectedRole?: any
}>()

const emit = defineEmits([
  'execution-started',
  'execution-progress',
  'execution-completed',
])

const { t } = useI18n()
const router = useRouter()

// 注意：角色管理现在在后端处理，不需要前端传递system_prompt

// Use composables
const {
  conversations,
  currentConversationId,
  isLoadingConversations,
    messages: conversationMessages,
  createNewConversation,
  loadConversations,
  switchToConversation,
  deleteConversation,
  clearCurrentConversation,
  saveMessagesToConversation,
  getCurrentConversationTitle,
} = useConversation()

// 使用简化的消息状态
const messages = ref<ChatMessage[]>([])

const { formatTime, renderMarkdown } = useMessageUtils()

// ReAct 消息解析函数
const isReActMessage = (message: ChatMessage) => {
  if (message.role !== 'assistant') return false
  const content = message.content || ''
  
  // 检测 ReAct 特征：Thought:, Action:, Observation:
  return /(?:Thought:|Action:|Observation:|Final Answer:)/i.test(content)
}

interface ReActStepData {
  thought?: string
  action?: any
  observation?: any
  error?: string
  finalAnswer?: string
}

// 修改版：优先使用消息中存储的 reactSteps，否则从 content 和 chunks 解析
const parseReActSteps = (content: string, messageId?: string): ReActStepData[] => {
  // 优先使用消息对象中已经解析并存储的 reactSteps
  const message = messages.value.find(m => m.id === messageId)
  if (message && (message as any).reactSteps) {
    console.log('[parseReActSteps] Using pre-parsed reactSteps from message:', messageId)
    return (message as any).reactSteps
  }
  
  
  const steps: ReActStepData[] = []
  
  // 尝试从 processor 获取原始 chunks (包含 ToolResult)
  const chunks = messageId ? (orderedMessages.processor.chunks.get(messageId) || []) : []
  const toolResultChunks = chunks.filter(c => c.chunk_type === 'ToolResult')
  
  
  // 分割内容为多个步骤（每个步骤以 Thought: 开始或独立的 Action: 开始）
  const lines = content.split('\n')
  let currentStep: ReActStepData = {}
  let inObservation = false
  let observationLines: string[] = []
  
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim()
    
    // 检测 Thought
    if (line.startsWith('Thought:')) {
      // 保存之前的步骤
      if (Object.keys(currentStep).length > 0) {
        if (observationLines.length > 0) {
          currentStep.observation = observationLines.join('\n')
          observationLines = []
          inObservation = false
        }
        steps.push(currentStep)
      }
      currentStep = {}
      currentStep.thought = line.substring('Thought:'.length).trim()
    }
    // 检测 Action
    else if (line.startsWith('Action:')) {
      if (inObservation && observationLines.length > 0) {
        currentStep.observation = observationLines.join('\n')
        observationLines = []
        inObservation = false
      }
      
      const actionContent = line.substring('Action:'.length).trim()
      
      // 检查下一行是否有 Action Input
      let actionInput = null
      if (i + 1 < lines.length && lines[i + 1].trim().startsWith('Action Input:')) {
        i++
        const inputLine = lines[i].substring(lines[i].indexOf('Action Input:') + 'Action Input:'.length).trim()
        try {
          actionInput = JSON.parse(inputLine)
        } catch {
          actionInput = inputLine
        }
      }
      
      currentStep.action = {
        tool: actionContent,
        args: actionInput,
        status: 'completed'
      }
      
      // 🔧 新增：尝试从 ToolResult chunks 中查找对应的 Observation
      const matchingToolResult = toolResultChunks.find(chunk => 
        chunk.tool_name === actionContent
      )
      
      if (matchingToolResult) {
        console.log('[parseReActSteps] Found ToolResult for tool:', actionContent)
        try {
          const obsData = JSON.parse(matchingToolResult.content.toString())
          currentStep.observation = obsData
          
          // 检查执行状态
          if (obsData.success === false || obsData.error) {
            currentStep.action.status = 'failed'
          }
        } catch (e) {
          // 如果不是 JSON，直接使用原始内容
          currentStep.observation = matchingToolResult.content.toString()
        }
      }
    }
    // 检测 Observation (保留旧逻辑作为后备)
    else if (line.startsWith('Observation:')) {
      inObservation = true
      const obsContent = line.substring('Observation:'.length).trim()
      if (obsContent) {
        observationLines.push(obsContent)
      }
      
      // 检查是否包含错误信息，如果有则更新 action 状态
      if (currentStep.action && obsContent) {
        try {
          const obsJson = JSON.parse(obsContent)
          if (obsJson.success === false || obsJson.error) {
            currentStep.action.status = 'failed'
            if (obsJson.error) {
              currentStep.error = obsJson.error
            }
          }
        } catch {
          // 如果不是 JSON，检查文本中是否包含错误关键字
          if (obsContent.toLowerCase().includes('error') || 
              obsContent.toLowerCase().includes('failed') ||
              obsContent.toLowerCase().includes('失败')) {
            currentStep.action.status = 'failed'
          }
        }
      }
    }
    // 检测 Final Answer
    else if (line.match(/^Final\s+Answer:/i)) {
      if (inObservation && observationLines.length > 0) {
        currentStep.observation = observationLines.join('\n')
        observationLines = []
        inObservation = false
      }
      
      const finalContent = line.substring(line.indexOf(':') + 1).trim()
      currentStep.finalAnswer = finalContent
      
      // 收集后续所有行作为 Final Answer 的一部分，直到消息结束
      // 不再检查 Thought/Action，因为 Final Answer 应该是最后一部分
      for (let j = i + 1; j < lines.length; j++) {
        const nextLine = lines[j]
        // 保留原始格式，包括空行
        if (currentStep.finalAnswer) {
          currentStep.finalAnswer += '\n' + nextLine
        } else if (nextLine.trim()) {
          currentStep.finalAnswer = nextLine
        }
      }
      // 已经收集完所有后续行，可以跳出循环
      break
    }
    // 继续收集 observation 内容
    else if (inObservation && line) {
      observationLines.push(line)
      
      // 持续检查后续行是否包含错误信息
      if (currentStep.action) {
        const combinedObs = observationLines.join('\n')
        try {
          const obsJson = JSON.parse(combinedObs)
          if (obsJson.success === false || obsJson.error) {
            currentStep.action.status = 'failed'
            if (obsJson.error && !currentStep.error) {
              currentStep.error = obsJson.error
            }
          }
        } catch {
          // 检查文本中的错误关键字
          if (combinedObs.toLowerCase().includes('error') || 
              combinedObs.toLowerCase().includes('failed') ||
              combinedObs.toLowerCase().includes('失败')) {
            currentStep.action.status = 'failed'
          }
        }
      }
    }
    // 继续收集 thought 内容（多行 thought）
    else if (!inObservation && line && !currentStep.action && currentStep.thought) {
      currentStep.thought += '\n' + line
    }
  }
  
  // 保存最后一个步骤
  if (Object.keys(currentStep).length > 0) {
    if (observationLines.length > 0) {
      currentStep.observation = observationLines.join('\n')
    }
    steps.push(currentStep)
  }
  
  return steps
}

// 持久化状态的key
const AI_CHAT_STATE_KEY = 'ai-chat-state'

// 从localStorage恢复状态的辅助函数
const restoreState = () => {
  try {
    const saved = localStorage.getItem(AI_CHAT_STATE_KEY)
    if (saved) {
      return JSON.parse(saved)
    }
  } catch (error) {
    console.warn('Failed to restore AI chat state:', error)
  }
  return {}
}

// 防抖保存状态到localStorage的辅助函数
let saveStateTimer: number | null = null
const saveState = () => {
  if (saveStateTimer) {
    clearTimeout(saveStateTimer)
  }
  
  saveStateTimer = window.setTimeout(() => {
    try {
      const state = {
        inputMessage: inputMessage.value,
        ragEnabled: ragEnabled.value,
        showDebugInfo: showDebugInfo.value,
        isTaskMode: isTaskMode.value,
        webSearchEnabled: webSearchEnabled.value,
        webSearchEngine: webSearchEngine.value,
      }
      localStorage.setItem(AI_CHAT_STATE_KEY, JSON.stringify(state))
    } catch (error) {
      console.warn('Failed to save AI chat state:', error)
    }
    saveStateTimer = null
  }, 300) // 300ms防抖
}

// 恢复保存的状态
const savedState = restoreState()

// Local state - 从保存的状态恢复或使用默认值
const inputMessage = ref(savedState.inputMessage || '')
const ragEnabled = ref(savedState.ragEnabled ?? false)
const isLoading = ref(false)
const messagesContainer = ref<HTMLElement | null>(null)
const currentExecutionId = ref<string | null>(null)
const streamStartTime = ref<number | null>(null)
const streamCharCount = ref(0)
const showDebugInfo = ref(savedState.showDebugInfo ?? false)
const loadingTimeoutId = ref<number | null>(null)
// Task mode state (controlled by toolbar button)
const isTaskMode = ref(savedState.isTaskMode ?? false)
// RAG reranking toggle from backend config
const rerankingEnabled = ref(false)

// Web search global toggle & engine selection (controlled by InputArea popover)
const webSearchEnabled = ref(savedState.webSearchEnabled ?? false)
const webSearchEngine = ref<'auto'|'google'|'bing'|'baidu'>(savedState.webSearchEngine || 'auto')

// Timeout mechanism to reset loading state
const resetLoadingWithTimeout = (timeoutMs = 300000) => {
  // 30 seconds timeout
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

// 使用简化的有序消息处理
// 仅由有序消息处理完成时触发一次保存（避免与其它路径重复）
const orderedMessages = useOrderedMessages(messages, async (msgs) => {
  try {
    await saveMessagesToConversation(msgs as any)
  } catch (error) {
    console.error('保存消息失败:', error)
  }
})
const sendMessage = async () => {
  if (!inputMessage.value.trim() || isLoading.value) return

  const rawInput = inputMessage.value
  const trimmed = rawInput.trim()
  const userInput = rawInput
  inputMessage.value = ''
  isLoading.value = true

  // Start timeout mechanism
  resetLoadingWithTimeout()

  try {
    // Ensure current conversation exists BEFORE adding messages
    if (!currentConversationId.value) {
      await createNewConversation()
    }

    // 创建用户消息
    const userMessage = createUserMessage(
      Date.now().toString(),
      userInput,
      new Date()
    )
    messages.value.push(userMessage)

    // 创建助手消息
    const assistantMessage = createAssistantMessage(
      `assistant_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      new Date()
    )
    messages.value.push(assistantMessage)

    await nextTick()
    scrollToBottom()

    // Handle based on input prefix
    if (isTaskMode.value) {

      // Generate unique execution ID
      const executionId = `exec_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
      currentExecutionId.value = executionId

      // Start task execution streaming
      assistantMessage.content = '正在生成执行计划...'

      try {
        const agentId = props.selectedAgent?.id
        
        await invoke('dispatch_scenario_task', {
          request: {
            agent_id: agentId,
            query: userInput,
            options: {
              conversation_id: currentConversationId.value,
              message_id: assistantMessage.id,
              execution_id: executionId,
              task_mode: true,
            },
          },
        })

        emit('execution-started', {
          id: executionId,
          name: '智能任务执行',
          description: userInput,
          progress: 0,
          status: 'running',
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
      // Chat mode - normal conversation with optional RAG or web search
      streamStartTime.value = Date.now()
      streamCharCount.value = 0

      try {
        if (ragEnabled.value) {
          // RAG模式：使用知识检索增强
          console.log('使用RAG模式回答问题')
          
          // 首先确保默认集合存在
          try {
            await invoke('ensure_default_rag_collection')
            console.log('默认RAG集合检查完成')
          } catch (collectionError) {
            console.warn('确保默认集合失败:', collectionError)
            // 继续执行，让RAG服务自己处理
          }
          // 加载已激活集合（若无则回退到默认集合）
          let activeIds: string[] = []
          try {
            activeIds = await invoke('get_active_rag_collections') as string[]
          } catch {
            activeIds = []
          }

          // 构造通用请求体
          const baseReq = {
            query: userInput,
            conversation_history: messages.value
              .filter(m => m.role === 'user' || m.role === 'assistant')
              .slice(-6)
              .map(m => m.content),
            top_k: 5,
            use_mmr: true,
            mmr_lambda: 0.7,
            similarity_threshold: 0.7,
            reranking_enabled: rerankingEnabled.value,
            model_provider: 'moonshot',
            model_name: 'moonshot-v1-8k',
            max_tokens: 2000,
            temperature: 0.3,
          }

          let combinedAnswer = ''
          let combinedCitations: any[] = []
          let fallbackReason: string | undefined

          if (activeIds.length > 0) {
            // 针对每个激活集合检索并合并
            for (const cid of activeIds) {
              try {
                const resp = await invoke('assistant_rag_answer', {
                  request: { ...baseReq, collection_id: cid }
                }) as any
                if (resp?.answer) {
                  combinedAnswer += (combinedAnswer ? '\n\n' : '') + resp.answer
                }
                if (Array.isArray(resp?.citations)) {
                  combinedCitations.push(...resp.citations)
                }
              } catch (e) {
                console.warn('集合检索失败', cid, e)
                fallbackReason = '部分集合检索失败'
              }
            }
          } else {
            // 无激活集合：使用默认集合
            const resp = await invoke('assistant_rag_answer', {
              request: { ...baseReq, collection_id: null }
            }) as any
            combinedAnswer = resp?.answer || ''
            combinedCitations = resp?.citations || []
            fallbackReason = resp?.fallback_reason
          }

          // 更新助手消息内容和引用
          assistantMessage.content = combinedAnswer || '抱歉，无法生成回答。'
          assistantMessage.citations = combinedCitations
          assistantMessage.isStreaming = false
          
          // 优雅的错误处理和降级提示
          if (fallbackReason) {
            console.warn('RAG降级原因:', fallbackReason)
            if (fallbackReason.includes('未找到相关上下文')) {
              assistantMessage.content += '\n\n💡 **提示**: 您可以尝试：\n• 重新表述问题\n• 添加更多相关文档到知识库\n• 关闭RAG模式使用普通聊天'
            } else if (fallbackReason.includes('RAG检索失败')) {
              assistantMessage.content += '\n\n⚠️ **系统提示**: 知识检索服务暂时不可用，已切换到普通聊天模式'
            }
          }
          
          assistantMessage.hasError = !combinedAnswer
          
          console.log('RAG回答完成:', {
            citations: combinedCitations?.length || 0,
            tokens: undefined,
            processingTime: undefined
          })
          
          // RAG模式下重置loading状态
          clearLoadingTimeout()
          isLoading.value = false
          streamStartTime.value = null
          streamCharCount.value = 0

          // 非流式路径下：只保存本次新增的用户消息和助手消息
          try {
            await saveMessagesToConversation([userMessage, assistantMessage] as any)
          } catch (e) {
            console.error('保存消息失败:', e)
          }
        } else {
          // 传统模式：流式聊天或网页搜索
          const useSearch = webSearchEnabled.value
          
          const returnedMessageId = await invoke(useSearch ? 'send_ai_stream_with_search' : 'send_ai_stream_message', {
            request: useSearch ? {
              conversation_id: currentConversationId.value,
              message: userInput,
              service_name: 'default',
              engine: webSearchEngine.value,
              auto: webSearchEngine.value === 'auto',
              limit: 5,
              message_id: assistantMessage.id,
            } : {
              conversation_id: currentConversationId.value,
              message: userInput,
              service_name: 'default',
              message_id: assistantMessage.id,
            },
            }) as string
          // Align local ids with server-acknowledged id to ensure consistency
          if (returnedMessageId && typeof returnedMessageId === 'string') {
            assistantMessage.id = returnedMessageId
          }
          // Note: Don't reset isLoading here - let the stream events handle it
        }
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
    // Find the assistant message to update its state
    const assistantMessage = messages.value[messages.value.length - 1];
    if(assistantMessage && assistantMessage.role === 'assistant') {
        assistantMessage.content = `${t('aiAssistant.error', '错误')}: ${error}`
        assistantMessage.isStreaming = false
        assistantMessage.hasError = true
    }
    clearLoadingTimeout()
    isLoading.value = false
    streamStartTime.value = null
    streamCharCount.value = 0
  }
}

const stopExecution = async () => {
  console.log('停止执行 - 当前执行ID:', currentExecutionId.value, '会话ID:', currentConversationId.value)
  
  // 优先调用统一的停止命令
  try {
    if (currentConversationId.value) {
      await invoke('stop_execution', {
        executionId: currentExecutionId.value || currentConversationId.value,
      })
      console.log('成功调用 stop_execution 命令')
    }
  } catch (error) {
    console.error('停止执行失败:', error)
  }

  // 额外调用取消流命令作为备用
  if (currentConversationId.value) {
    try {
      await invoke('cancel_ai_stream', {
        conversationId: currentConversationId.value,
      })
      console.log('成功调用 cancel_ai_stream 命令')
    } catch (error) {
      console.error('取消流失败:', error)
    }
  }

  // 更新UI状态
  const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
  if (lastAssistantMessage && lastAssistantMessage.isStreaming) {
    lastAssistantMessage.isStreaming = false
    if (!lastAssistantMessage.content.includes('[用户中断了响应]')) {
      lastAssistantMessage.content += '\n\n[用户中断了响应]'
    }
  }

  // 清理执行ID
  currentExecutionId.value = null
  
  // 重置加载状态
  clearLoadingTimeout()
  isLoading.value = false
  streamStartTime.value = null
  streamCharCount.value = 0
  
  console.log('停止执行完成，已重置所有状态')
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

const citationModalOpen = ref(false)
const citationDetail = ref<Citation | null>(null)
const openCitationModal = (citation: Citation) => {
  citationDetail.value = citation
  citationModalOpen.value = true
}

const jumpToRagSource = (citation: Citation) => {
  const file = encodeURIComponent((citation.source_id || citation.file_name || '').toString())
  const start = citation.start_char
  const end = citation.end_char
  router.push(`/rag-management?file=${file}&start=${start}&end=${end}`)
}

const copyMessage = async (content: string) => {
  try {
    await navigator.clipboard.writeText(content)
    console.log('Message copied to clipboard')
    // TODO: 可以添加一个toast提示
  } catch (error) {
    console.error('Failed to copy message:', error)
    // 降级方案：使用传统的复制方法
    const textArea = document.createElement('textarea')
    textArea.value = content
    document.body.appendChild(textArea)
    textArea.select()
    document.execCommand('copy')
    document.body.removeChild(textArea)
  }
}

const resendMessage = (content: string) => {
  if (isLoading.value) {
    console.warn('Cannot resend message while loading')
    return
  }
  inputMessage.value = content
  sendMessage()
}

const getStreamSpeed = () => {
  if (!streamStartTime.value || streamCharCount.value === 0) return 0
  const elapsed = (Date.now() - streamStartTime.value) / 1000
  return Math.round(streamCharCount.value / elapsed)
}

// no-op: architecture selection removed


const isConfigError = (content: string) => {
  return (
    content.includes('配置') ||
    content.includes('API') ||
    content.includes('provider') ||
    content.includes('not configured') ||
    content.includes('空响应') ||
    content.includes('configuration')
  )
}

 




// Conversation management methods
const handleCreateNewConversation = async () => {
  try {
    await createNewConversation()
    console.log('New conversation created successfully')
  } catch (error) {
    console.error('Failed to create new conversation:', error)
  }
}

const handleClearConversation = async () => {
  if (!currentConversationId.value) {
    console.warn('No active conversation to clear')
    return
  }
  try {
    await clearCurrentConversation()
    await createNewConversation()
    console.log('Conversation cleared and new one created')
  } catch (error) {
    console.error('Failed to clear conversation:', error)
  }
}

const handleToggleTaskMode = (enabled: boolean) => {
  isTaskMode.value = enabled
  console.log(`Task mode ${enabled ? 'enabled' : 'disabled'}`)
  // 保存状态到本地存储
  saveState()
}

const handleToggleRAG = (enabled: boolean) => {
  ragEnabled.value = enabled
  console.log('RAG模式:', enabled ? '开启' : '关闭')
  // 持久化到后端全局配置（仅更新该字段）
  saveRagConfig({ augmentation_enabled: enabled }).catch(err => {
    console.error('保存RAG配置失败:', err)
  })
  // 同时保存到本地状态
  saveState()
}


// Lifecycle
onMounted(async () => {
  await loadConversations()
  if (conversations.value.length > 0 && !currentConversationId.value) {
    await switchToConversation(conversations.value[0].id)
  }
  await orderedMessages.setupEventListeners()

  // 初始化：从后端读取配置，设置本地 RAG 开关（优先级高于localStorage）
  try {
    const cfg = await getRagConfig()
    ragEnabled.value = !!cfg.augmentation_enabled
    rerankingEnabled.value = !!cfg.reranking_enabled
    // 同步更新本地状态
    saveState()
  } catch (e) {
    console.warn('获取RAG配置失败，使用本地保存的状态或默认关闭:', e)
    // 如果后端配置获取失败，保持从localStorage恢复的状态
  }

  // Listen to search state updates from InputAreaComponent
  window.addEventListener('sentinel-websearch-updated', (e: any) => {
    if (e?.detail) {
      webSearchEnabled.value = !!e.detail.enabled
      if (e.detail.engine) webSearchEngine.value = e.detail.engine
    }
  })

  // 首次打开时滚动到底部
  nextTick(() => {
    scrollToBottom()
  })
})

// 同步全局会话消息到本地列表（不再转换类型）
watch(
  () => ({
    msgs: conversationMessages.value,
    cid: currentConversationId.value,
  }),
  ({ msgs }) => {
    if (Array.isArray(msgs)) {
      messages.value = msgs as ChatMessage[]
      // nextTick(() => scrollToBottom())
    }
  },
  { deep: true, immediate: true }
)

// 跟随消息流状态自动同步 isLoading，用于切换发送/停止按钮
watch(
  () => messages.value.some(m => m.role === 'assistant' && m.isStreaming),
  streaming => {
    if (streaming) {
      isLoading.value = true
    } else {
      isLoading.value = false
      streamStartTime.value = null
      streamCharCount.value = 0
      clearLoadingTimeout()
    }
  },
  { immediate: true }
)

// 监听状态变化并自动保存
watch(
  [inputMessage, showDebugInfo, webSearchEnabled, webSearchEngine],
  () => {
    saveState()
  },
  { deep: true }
)

onUnmounted(() => {
  clearLoadingTimeout()
  orderedMessages.cleanup()
  
  // 清理保存状态的定时器并立即保存
  if (saveStateTimer) {
    clearTimeout(saveStateTimer)
    saveStateTimer = null
  }
  
  // 确保在组件卸载时立即保存状态
  try {
    const state = {
      inputMessage: inputMessage.value,
      ragEnabled: ragEnabled.value,
      showDebugInfo: showDebugInfo.value,
      isTaskMode: isTaskMode.value,
      webSearchEnabled: webSearchEnabled.value,
      webSearchEngine: webSearchEngine.value,
    }
    localStorage.setItem(AI_CHAT_STATE_KEY, JSON.stringify(state))
  } catch (error) {
    console.warn('Failed to save AI chat state on unmount:', error)
  }
})

// Expose conversation controls/state for parent (AIAssistant)
defineExpose({
  conversations,
  currentConversationId,
  isLoadingConversations,
  createNewConversation,
  loadConversations,
  switchToConversation,
  deleteConversation,
  clearCurrentConversation,
  getCurrentConversationTitle,
})
</script>

<style scoped>
.enhanced-ai-chat {
  font-family:
    'Inter',
    -apple-system,
    BlinkMacSystemFont,
    'Segoe UI',
    Roboto,
    sans-serif;
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
  0%,
  50% {
    opacity: 1;
  }
  51%,
  100% {
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
  from {
    opacity: 0;
  }
  to {
    opacity: 0.7;
  }
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

.chat-bubble :deep(pre),
.chat-bubble :deep(code) {
  white-space: pre-wrap;
  word-break: break-word;
}

.chat-bubble :deep(pre) {
  max-width: 100%;
  overflow: auto;
}
</style>
