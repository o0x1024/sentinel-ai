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
        :class="['chat', message.role === 'user' ? 'chat-end' : 'chat-start', 'mb-4']"
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

        <div class="chat-header mb-2">
          <span class="font-medium text-sm text-base-content/80">
            {{ message.role === 'user' ? t('common.you', '您') : t('common.assistant', 'AI助手') }}
          </span>
          <time class="text-xs text-base-content/60 ml-2 px-2 py-0.5 bg-base-200 rounded-full">
            {{ formatTime(message.timestamp) }}
          </time>
        </div>

        <div
          :class="[
            'chat-bubble max-w-[85%] shadow-sm border transition-all duration-200',
            message.role === 'user'
              ? 'bg-base-100 text-primary-content border-primary/20'
              : 'bg-base-100 text-base-content border-base-300 hover:border-base-400',
          ]"
        >
          <!-- 简化的消息内容显示 - 统一使用 Markdown 渲染 -->
          <div 
            :class="[
              'prose prose-sm max-w-none leading-relaxed',
              message.role === 'user' ? 'prose-invert' : 'prose-neutral'
            ]"
            v-html="renderMarkdown(message.content)"
          />

          <!-- 计划 Markdown TodoList 展示 -->
          <div v-if="message.executionPlan && Array.isArray(message.executionPlan.steps) && message.executionPlan.steps.length" class="mt-3">
            <div
              class="prose prose-sm max-w-none leading-relaxed"
              v-html="renderMarkdown(planToMarkdown(message.executionPlan))"
            />
          </div>

          <!-- 流式指示器 -->
          <div v-if="message.isStreaming" class="flex items-center gap-2 mt-2 text-base-content/70">
            <span class="loading loading-dots loading-sm text-primary"></span>
            <span class="text-sm">{{ t('aiAssistant.generating', 'AI正在思考...') }}</span>
          </div>

          <!-- 工具执行折叠面板 -->
          <div v-if="message.toolExecutions && message.toolExecutions.length" class="mt-3 space-y-2">
            <div
              v-for="exec in message.toolExecutions"
              :key="exec.id || exec.stepName"
              class="collapse collapse-arrow border border-base-300 bg-base-100 rounded-box"
            >
              <input type="checkbox" />
              <div class="collapse-title text-sm font-medium flex items-center justify-between gap-3">
                <span class="truncate">{{ exec.stepName || t('aiAssistant.tool.unnamed', '未命名任务') }}</span>
                <span :class="statusBadgeClass(exec.status)">{{ statusText(exec.status) }}</span>
              </div>
              <div class="collapse-content">
                <div class="text-sm text-base-content/80">
                  <span class="font-semibold mr-2">{{ t('aiAssistant.tool.validInfo', '有效信息') }}:</span>
                  <span class="font-mono">{{ exec.extracted || exec.content || '-' }}</span>
                </div>
              </div>
            </div>
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
                :key="citation.id"
                class="group relative"
              >
                <button
                  @click="showCitationDetail(citation)"
                  class="btn btn-xs btn-outline gap-1 hover:btn-accent transition-all duration-200"
                  :title="citation.file_name"
                >
                  <i class="fas fa-file-alt text-xs"></i>
                  <span class="text-xs">[{{ index + 1 }}] {{ citation.file_name.split('/').pop() }}</span>
                </button>
                
                <!-- 悬浮预览 -->
                <div class="absolute bottom-full left-0 mb-2 hidden group-hover:block z-50 w-80 max-w-sm">
                  <div class="bg-base-100 border border-base-300 rounded-lg shadow-lg p-3 text-xs">
                    <div class="font-medium mb-1">{{ citation.file_name }}</div>
                    <div class="text-base-content/70 mb-2">
                      {{ citation.page_number ? `第${citation.page_number}页` : '' }}
                      {{ citation.section_title ? ` - ${citation.section_title}` : '' }}
                    </div>
                    <div class="text-base-content/80 line-clamp-3">
                      {{ citation.content_preview }}
                    </div>
                    <div class="mt-2 text-accent text-xs">
                      相似度: {{ (citation.score * 100).toFixed(1) }}%
                    </div>
                  </div>
                </div>
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
      </div>
    </div>

    <!-- Input Area -->
    <InputAreaComponent
      v-model:input-message="inputMessage"
      :is-loading="isLoading"
      :show-debug-info="showDebugInfo"
      @send-message="sendMessage"
      @stop-execution="stopExecution"
      @toggle-debug="showDebugInfo = !showDebugInfo"
      @create-new-conversation="handleCreateNewConversation"
      @clear-conversation="handleClearConversation"
      @toggle-task-mode="handleToggleTaskMode"
      @toggle-rag="handleToggleRAG"
    />

  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

// Composables
import { useConversation } from '../composables/useConversation'
import { useMessageUtils } from '../composables/useMessageUtils'
import { useOrderedMessages } from '../composables/useOrderedMessages'

// Components
import InputAreaComponent from './InputAreaComponent.vue'

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
}>()

const emit = defineEmits([
  'execution-started',
  'execution-progress',
  'execution-completed',
])

const { t } = useI18n()
const router = useRouter()

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

// Local state
const inputMessage = ref('B站今天有什么热门视频')
const ragEnabled = ref(false)
const isLoading = ref(false)
const messagesContainer = ref<HTMLElement | null>(null)
const currentExecutionId = ref<string | null>(null)
const streamStartTime = ref<number | null>(null)
const streamCharCount = ref(0)
const showDebugInfo = ref(false)
const loadingTimeoutId = ref<number | null>(null)
// Task mode state (controlled by toolbar button)
const isTaskMode = ref(false)

// Web search global toggle & engine selection (controlled by InputArea popover)
const webSearchEnabled = ref(false)
const webSearchEngine = ref<'auto'|'google'|'bing'|'baidu'>('auto')

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
          
          const ragResponse = await invoke('assistant_rag_answer', {
            request: {
              query: userInput,
              collection_id: null, // 使用默认集合
              conversation_history: messages.value
                .filter(m => m.role === 'user' || m.role === 'assistant')
                .slice(-6) // 最近3轮对话
                .map(m => m.content),
              top_k: 5,
              use_mmr: true,
              mmr_lambda: 0.7,
              similarity_threshold: 0.7,
              reranking_enabled: false, // 暂时关闭重排序
              model_provider: 'moonshot',
              model_name: 'moonshot-v1-8k',
              max_tokens: 2000,
              temperature: 0.3,
            }
          }) as any

          // 更新助手消息内容和引用
          assistantMessage.content = ragResponse.answer || '抱歉，无法生成回答。'
          assistantMessage.citations = ragResponse.citations || []
          assistantMessage.isStreaming = false
          
          // 优雅的错误处理和降级提示
          if (ragResponse.fallback_reason) {
            console.warn('RAG降级原因:', ragResponse.fallback_reason)
            if (ragResponse.fallback_reason.includes('未找到相关上下文')) {
              assistantMessage.content += '\n\n💡 **提示**: 您可以尝试：\n• 重新表述问题\n• 添加更多相关文档到知识库\n• 关闭RAG模式使用普通聊天'
            } else if (ragResponse.fallback_reason.includes('RAG检索失败')) {
              assistantMessage.content += '\n\n⚠️ **系统提示**: 知识检索服务暂时不可用，已切换到普通聊天模式'
            }
          }
          
          assistantMessage.hasError = !ragResponse.answer
          
          console.log('RAG回答完成:', {
            citations: ragResponse.citations?.length || 0,
            tokens: ragResponse.total_tokens_used || 0,
            processingTime: ragResponse.processing_time_ms || 0
          })
          
          // RAG模式下重置loading状态
          clearLoadingTimeout()
          isLoading.value = false
          streamStartTime.value = null
          streamCharCount.value = 0
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
  if (currentExecutionId.value) {
    try {
      await invoke('stop_execution', {
        executionId: currentExecutionId.value,
      })
    } catch (error) {
      console.error('Failed to stop execution:', error)
    }
  }

  if (currentConversationId.value) {
    try {
      await invoke('cancel_ai_stream', {
        conversationId: currentConversationId.value,
      })
    } catch (error) {
      console.error('Failed to cancel stream:', error)
    }
  }

  const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
  if (lastAssistantMessage && lastAssistantMessage.isStreaming) {
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

const showCitationDetail = (citation: Citation) => {
  console.log('查看引用详情:', citation)
  // TODO: 可以打开一个模态框显示详细内容
  // 或者跳转到RAG管理页面查看原文档
  router.push(`/rag-management?file=${citation.source_id}`)
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

// 工具执行状态样式与文案
const statusBadgeClass = (status: string) => {
  switch (status) {
    case 'Completed':
      return 'badge badge-success'
    case 'Running':
      return 'badge badge-warning'
    case 'Failed':
      return 'badge badge-error'
    default:
      return 'badge'
  }
}

const statusText = (status: string) => {
  switch (status) {
    case 'Completed':
      return '已完成'
    case 'Running':
      return '执行中'
    case 'Failed':
      return '失败'
    default:
      return status || '未知'
  }
}

// 将执行计划转换为 Markdown TodoList
const planToMarkdown = (plan: any): string => {
  if (!plan || !Array.isArray(plan.steps)) return ''
  const lines: string[] = []
  lines.push('### 执行计划')
  plan.steps.forEach((s: any) => {
    const checked = s.status === 'Completed' ? 'x' : ' '
    const name = s?.name || '未命名步骤'
    const desc = s?.description ? ` - ${s.description}` : ''
    const tool = s?.tool?.name ? ` (tool: ${s.tool.name})` : ''
    lines.push(`- [${checked}] ${name}${tool}${desc}`)
  })
  return lines.join('\n')
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
}

const handleToggleRAG = (enabled: boolean) => {
  ragEnabled.value = enabled
  console.log('RAG模式:', enabled ? '开启' : '关闭')
}


// Lifecycle
onMounted(async () => {
  await loadConversations()
  if (conversations.value.length > 0 && !currentConversationId.value) {
    await switchToConversation(conversations.value[0].id)
  }
  await orderedMessages.setupEventListeners()

  // Listen to search state updates from InputAreaComponent
  window.addEventListener('sentinel-websearch-updated', (e: any) => {
    if (e?.detail) {
      webSearchEnabled.value = !!e.detail.enabled
      if (e.detail.engine) webSearchEngine.value = e.detail.engine
    }
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
      nextTick(() => scrollToBottom())
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

onUnmounted(() => {
  clearLoadingTimeout()
  orderedMessages.cleanup()
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

/* Tool result overflow handling */
.chat-bubble :deep(details.tool-result) {
  max-width: 100%;
}

.chat-bubble :deep(details.tool-result > summary) {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chat-bubble :deep(.tool-result-body) {
  max-width: 100%;
  overflow-x: auto;
}

.chat-bubble :deep(.tool-result-content) {
  max-width: 100%;
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
