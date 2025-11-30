<template>
  <div
    class="enhanced-ai-chat w-full h-full flex flex-col bg-gradient-to-br from-base-100 to-base-200 overflow-hidden"
  >
    <!-- Messages Area -->
    <div ref="messagesContainer" @scroll="handleUserScroll" class="flex-1 overflow-y-auto p-4 space-y-4 min-h-0 max-w-full">
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
         

          <!-- Plan-and-Execute 步骤显示 -->
          <div v-if="isPlanAndExecuteMessageFn(message)" class="space-y-3">
            <PlanAndExecuteStepDisplay
              v-bind="parsePlanAndExecuteMessageData(message)"
            />
          </div>

          <!-- LLM Compiler 步骤显示 -->
          <div v-else-if="isLLMCompilerMessageFn(message)" class="space-y-3">
            <LLMCompilerStepDisplay
              v-bind="parseLLMCompilerMessageData(message)"
            />
            <!-- 显示最终响应：直接使用message.content中的纯文本部分 -->
            <div v-if="getLLMCompilerTextContent(message)" class="llm-compiler-final-response mt-4 p-4 bg-base-100 rounded-lg border border-base-300">
              <div class="prose prose-sm max-w-none" v-html="renderMarkdown(getLLMCompilerTextContent(message))"></div>
            </div>
          </div>

          <!-- ReWOO 步骤显示 -->
          <div v-else-if="isReWOOMessageFn(message)" class="space-y-3">
            <ReWOOStepDisplay
              v-bind="parseReWOOMessageData(message)"
            />
          </div>

          <!-- Travel 步骤显示 -->
          <div v-else-if="isTravelMessageFn(message)" class="space-y-3">
            <TravelStepDisplay
              :message="message"
              :stepData="parseTravelMessageData(message)"
            />
          </div>

          <!-- ReAct 消息：工具调用内联在流式内容中 -->
          <div v-else-if="isReActMessage(message)">
            <MessageContentDisplay
              :message="message"
              :is-typing="message.isStreaming"
              :stream-char-count="streamCharCount"
            />
          </div>

          <!-- 普通消息显示 - 使用统一组件渲染文本 + 附件图片 -->
          <MessageContentDisplay
            v-else
            :message="message"
            :is-typing="message.isStreaming"
            :stream-char-count="streamCharCount"
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

          <!-- Assistant Actions - Inside bubble bottom -->
          
        </div>

        <!-- User Message Actions - Outside the bubble -->
        <div
          v-if="message.role === 'user'"
          class="flex gap-2 justify-end mt-2 opacity-0 group-hover:opacity-100 transition-opacity duration-200"
        >
          <button 
            @click="copyMessage(message)" 
            :class="[
              'btn btn-xs gap-1 transition-colors',
              copiedMessageId === message.id 
                ? 'btn-success text-success'
                : 'btn-ghost text-base-content/60 hover:text-base-content'
            ]"
            :title="copiedMessageId === message.id ? '已复制' : '复制消息'"
          >
            <i v-if="copiedMessageId !== message.id" class="fas fa-copy text-xs"></i>
            <i v-else class="fas fa-check text-xs"></i>
            <span class="text-xs" v-if="copiedMessageId !== message.id">复制</span>
            <span class="text-xs" v-else>已复制</span>
          </button>
          <button 
            @click="resendMessage(message)" 
            class="btn btn-xs btn-ghost gap-1 text-base-content/60 hover:text-base-content"
            title="重新发送"
          >
            <i class="fas fa-redo text-xs"></i>
            <span class="text-xs">重发</span>
          </button>
        </div>

        <div
          v-if="message.role === 'assistant'"
          class="chat-footer mt-2 opacity-0 group-hover:opacity-100"
        >
          <div class="flex gap-2">
            <button 
              @click="copyMessage(message)" 
              :class="[
                'btn btn-xs gap-1 transition-colors',
                copiedMessageId === message.id 
                  ? 'btn-success text-success'
                  : 'btn-ghost text-base-content/60 hover:text-base-content'
              ]"
              :title="copiedMessageId === message.id ? '已复制' : '复制回复'"
            >
              <i v-if="copiedMessageId !== message.id" class="fas fa-copy text-xs"></i>
              <i v-else class="fas fa-check text-xs"></i>
              <span class="text-xs" v-if="copiedMessageId !== message.id">复制</span>
              <span class="text-xs" v-else>已复制</span>
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
      :rag-enabled="ragEnabled"
      :pending-attachments="pendingAttachments"
      @send-message="sendMessage"
      @stop-execution="stopExecution"
      @toggle-debug="showDebugInfo = !showDebugInfo"
      @create-new-conversation="handleCreateNewConversation"
      @clear-conversation="handleClearConversation"
      @toggle-task-mode="handleToggleTaskMode"
      @toggle-rag="handleToggleRAG"
      @add-attachments="handleAddAttachments"
      @remove-attachment="handleRemoveAttachment"
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
import { ReActMessageProcessor } from '../composables/processors/ReActMessageProcessor'
import { isReWOOMessage, parseReWOOMessage } from '../composables/useReWOOMessage'
import type { ReWOOMessageData } from '../composables/useReWOOMessage'
import { isLLMCompilerMessage, parseLLMCompilerMessage } from '../composables/useLLMCompilerMessage'
import type { LLMCompilerMessageData } from '../composables/useLLMCompilerMessage'
import { isPlanAndExecuteMessage, parsePlanAndExecuteMessage } from '../composables/usePlanAndExecuteMessage'
import type { PlanAndExecuteMessageData } from '../composables/usePlanAndExecuteMessage'

// Components
import InputAreaComponent from './InputAreaComponent.vue'
import ReActStepDisplay from './MessageParts/ReActStepDisplay.vue'
import ReWOOStepDisplay from './MessageParts/ReWOOStepDisplay.vue'
import LLMCompilerStepDisplay from './MessageParts/LLMCompilerStepDisplay.vue'
import PlanAndExecuteStepDisplay from './MessageParts/PlanAndExecuteStepDisplay.vue'
import TravelStepDisplay from './MessageParts/TravelStepDisplay.vue'
import MessageContentDisplay from './MessageParts/MessageContentDisplay.vue'
import { isTravelMessage, parseTravelMessage } from '../composables/useTravelMessage'
import type { TravelMessageData } from '../composables/useTravelMessage'
import OrchestratorStepDisplay from './MessageParts/OrchestratorStepDisplay.vue'

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

// 待发送的附件（上传完成后存为后端返回的Attachment JSON）
const pendingAttachments = ref<any[]>([])

const { formatTime, renderMarkdown } = useMessageUtils()

// 处理来自输入区的附件选择（默认按 Tauri 环境处理）
const handleAddAttachments = async (filePaths: string[]) => {
  if (!filePaths || filePaths.length === 0) return

  console.log('[AIChat] 接收到附件路径:', filePaths)
  try {
    const attachments = await invoke<any[]>('upload_multiple_images', { filePaths })
    if (attachments && attachments.length > 0) {
      pendingAttachments.value.push(...attachments)
      console.log('[AIChat] 成功上传', attachments.length, '个图片附件:', attachments)
    }
  } catch (error) {
    console.error('[AIChat] 批量上传图片附件失败:', error)
  }
}

// 移除待发附件
const handleRemoveAttachment = (index: number) => {
  if (index >= 0 && index < pendingAttachments.value.length) {
    pendingAttachments.value.splice(index, 1)
    console.log('[AIChat] 已移除附件，剩余:', pendingAttachments.value.length)
  }
}

// 新增：从架构元数据判断架构类型
const getMessageArchitecture = (message: ChatMessage): string => {
  // 优先使用message对象中的architectureType
  if (message.architectureType) {
    return message.architectureType
  }
  
  // 回退到processor（仅用于streaming消息）
  if (message.isStreaming) {
    const archInfo = orderedMessages.processor.getArchitectureInfo?.(message.id)
    if (archInfo?.type) {
      return archInfo.type
    }
  }
  
  return 'Unknown'
}

// ReAct 消息解析函数（增强版：优先使用架构元数据）
const isReActMessage = (message: ChatMessage) => {
  if (message.role !== 'assistant') return false
  
  // 优先检查架构元数据
  const archType = getMessageArchitecture(message)
  if (archType === 'ReAct') return true
  
  // 回退到内容匹配（向后兼容）
  const content = message.content || ''
  return /(?:Thought:|Action:|Observation:|Final Answer:)/i.test(content)
}

// Plan-and-Execute 消息检测函数（增强版：优先使用架构元数据）
const isPlanAndExecuteMessageFn = (message: ChatMessage) => {
  if (message.role !== 'assistant') return false

  // 优先检查架构元数据
  const archType = getMessageArchitecture(message)
  if (archType === 'PlanAndExecute') return true
  // 如果已经是其他明确的架构类型，直接返回false
  if (archType && archType !== 'Unknown') return false

  // 回退到内容匹配（向后兼容，仅用于Unknown架构）
  const content = message.content || ''
  const chunks = orderedMessages.processor.chunks.get(message.id) || []
  return isPlanAndExecuteMessage(content, chunks)
}

// Plan-and-Execute 消息解析函数
const parsePlanAndExecuteMessageData = (message: ChatMessage): PlanAndExecuteMessageData => {
  // 优先使用预解析的数据
  if ((message as any).planAndExecuteData) {
    return (message as any).planAndExecuteData
  }
  const content = message.content || ''
  const chunks = orderedMessages.processor.chunks.get(message.id) || []
  return parsePlanAndExecuteMessage(content, chunks)
}

// LLM Compiler 消息检测函数（增强版：优先使用架构元数据）
const isLLMCompilerMessageFn = (message: ChatMessage) => {
  if (message.role !== 'assistant') return false

  // 优先检查架构元数据
  const archType = getMessageArchitecture(message)
  if (archType === 'LLMCompiler') return true
  // 如果已经是其他明确的架构类型，直接返回false
  if (archType && archType !== 'Unknown') return false

  // 回退到内容匹配（向后兼容，仅用于Unknown架构）
  const content = message.content || ''
  const chunks = orderedMessages.processor.chunks.get(message.id) || []
  return isLLMCompilerMessage(content, chunks)
}

// LLM Compiler 消息解析函数
const parseLLMCompilerMessageData = (message: ChatMessage): LLMCompilerMessageData => {
  // 优先使用预解析的数据
  if ((message as any).llmCompilerData) {
    return (message as any).llmCompilerData
  }
  const content = message.content || ''
  const chunks = orderedMessages.processor.chunks.get(message.id) || []
  return parseLLMCompilerMessage(content, chunks)
}

// LLM Compiler 获取纯文本内容
const getLLMCompilerTextContent = (message: ChatMessage): string => {
  // 1. 首先检查已保存的最终响应
  if ((message as any).llmCompilerFinalResponse) {
    return (message as any).llmCompilerFinalResponse
  }

  // 2. 从chunks获取Content类型的文本（流式过程中）
  const chunks = orderedMessages.processor.chunks.get(message.id) || []
  const contentChunks = chunks.filter(c =>
    c.chunk_type === 'Content' && c.architecture === 'LLMCompiler'
  )
  if (contentChunks.length > 0) {
    return contentChunks.map(c => c.content?.toString() || '').join('')
  }

  // 3. 从message.content中提取[DECISION]部分的response（历史消息fallback）
  const content = message.content || ''
  if (content.includes('[DECISION]')) {
    // 尝试从[DECISION]后的JSON中提取response字段
    const decisionIdx = content.indexOf('[DECISION]')
    const afterDecision = content.substring(decisionIdx + 10)
    
    // 查找JSON代码块
    const jsonMatch = afterDecision.match(/```json\s*([\s\S]*?)```/)
    if (jsonMatch) {
      try {
        const json = JSON.parse(jsonMatch[1])
        if (json.response) {
          return json.response
        }
      } catch (e) {
        // JSON解析失败，尝试正则提取
      }
    }
    
    // 正则提取response字段
    const responseMatch = afterDecision.match(/"response"\s*:\s*"([\s\S]*?)(?:"\s*,|\"\s*\})/i)
    if (responseMatch && responseMatch[1]) {
      return responseMatch[1].replace(/\\n/g, '\n').replace(/\\"/g, '"')
    }
  }

  return ''
}

// ReWOO 消息检测函数（增强版：优先使用架构元数据）
const isReWOOMessageFn = (message: ChatMessage) => {
  if (message.role !== 'assistant') return false

  // 优先检查架构元数据
  const archType = getMessageArchitecture(message)
  if (archType === 'ReWOO') return true
  // 如果已经是其他明确的架构类型，直接返回false
  if (archType && archType !== 'Unknown') return false

  // 回退到内容匹配（向后兼容，仅用于Unknown架构）
  const content = message.content || ''
  const chunks = orderedMessages.processor.chunks.get(message.id) || []
  return isReWOOMessage(content, chunks)
}

// ReWOO 消息解析函数
const parseReWOOMessageData = (message: ChatMessage): ReWOOMessageData => {
  // 优先使用预解析的数据
  if ((message as any).rewooData) {
    return (message as any).rewooData
  }
  const content = message.content || ''
  const chunks = orderedMessages.processor.chunks.get(message.id) || []
  return parseReWOOMessage(content, chunks)
}

// Travel 消息检测函数（增强版：优先使用架构元数据）
const isTravelMessageFn = (message: ChatMessage) => {
  if (message.role !== 'assistant') return false

  // 优先检查架构元数据
  const archType = getMessageArchitecture(message)
  if (archType === 'Travel') return true
  // 如果已经是其他明确的架构类型，直接返回false
  if (archType && archType !== 'Unknown') return false

  // 回退到内容匹配（向后兼容，仅用于Unknown架构）
  const content = message.content || ''
  const chunks = orderedMessages.processor.chunks.get(message.id) || []
  return isTravelMessage(content, chunks)
}

// Travel 消息解析函数
const parseTravelMessageData = (message: ChatMessage): TravelMessageData => {
  // 优先使用预解析的数据
  if ((message as any).travelData) {
    return (message as any).travelData
  }
  const content = message.content || ''
  const chunks = orderedMessages.processor.chunks.get(message.id) || []
  return parseTravelMessage(content, chunks)
}

// Orchestrator 消息检测函数（增强版：优先使用架构元数据）
const isOrchestratorMessageFn = (message: ChatMessage) => {
  if (message.role !== 'assistant') return false
  
  // 优先检查架构元数据
  const archType = getMessageArchitecture(message)
  if (archType === 'Travel') return false // Travel now handled by isTravelMessageFn
  
  // 回退到内容匹配（向后兼容）
  const content = message.content || ''
  // 1) 优先尝试直接解析消息内容（兼容早期单条JSON场景）
  try {
    const parsed = JSON.parse(content)
    if (
      parsed?.type === 'orchestrator_session' ||
      parsed?.type === 'orchestrator_step' ||
      parsed?.type === 'orchestrator_bundle'
    ) {
      return true
    }
  } catch {
    // ignore
  }
  // 2) 回退：从分片队列中查找 Orchestrator 的 Meta 事件
  const chunks = orderedMessages.processor.chunks.get(message.id) || []
  return chunks.some(c => {
    if (c.chunk_type !== 'Meta' || !c.content) return false
    try {
      const obj = JSON.parse(c.content.toString())
      return obj?.type === 'orchestrator_session' || obj?.type === 'orchestrator_step'
    } catch {
      return false
    }
  })
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
      
      // 默认认为工具调用仍在运行中，等待 ToolResult 或 Observation 更新状态
      currentStep.action = {
        tool: actionContent,
        args: actionInput,
        status: 'running'
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
          
          // 根据 ToolResult 中的 success / error 字段更新状态
          if (obsData.success === false || obsData.error) {
            currentStep.action.status = 'failed'
          } else {
            currentStep.action.status = 'success'
          }
        } catch (e) {
          // 如果不是 JSON，直接使用原始内容，但仍认为调用已结束
          currentStep.observation = matchingToolResult.content.toString()
          currentStep.action.status = 'success'
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

// 判断指定消息中是否存在仍在运行中的工具调用
const hasRunningTool = (message: ChatMessage): boolean => {
  if (!isReActMessage(message)) return false
  const steps = ReActMessageProcessor.buildReActStepsFromMessage(message)
  return steps.some(s => s.action && (s.action.status === 'running' || s.action.status === 'pending'))
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

// 检查用户是否在底部（允许一定误差）
const isUserAtBottom = () => {
  if (!messagesContainer.value) return true
  const { scrollTop, scrollHeight, clientHeight } = messagesContainer.value
  const threshold = 100 // 距离底部100px以内认为是在底部
  return scrollHeight - scrollTop - clientHeight < threshold
}

// 用户滚动状态跟踪
const userIsScrolling = ref(false)
const scrollTimeout = ref<number | null>(null)

// 监听用户滚动行为
const handleUserScroll = () => {
  userIsScrolling.value = true
  
  // 清除之前的定时器
  if (scrollTimeout.value) {
    clearTimeout(scrollTimeout.value)
  }
  
  // 500ms后重置滚动状态
  scrollTimeout.value = window.setTimeout(() => {
    userIsScrolling.value = false
  }, 500)
}

// 智能滚动到底部：只在用户已经在底部时才滚动
const scrollToBottom = (force = false) => {
  nextTick(() => {
    if (messagesContainer.value) {
      // 强制滚动或用户在底部时才滚动
      if (force || isUserAtBottom()) {
        messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
      }
    }
  })
}

// 使用简化的有序消息处理
// 仅由有序消息处理完成时触发一次保存（避免与其它路径重复）
// 使用简化的有序消息处理
// 仅由有序消息处理完成时触发一次保存（避免与其它路径重复）
const orderedMessages = useOrderedMessages(messages)
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
    ;(userMessage as any).is_task_mode = isTaskMode.value
    if (pendingAttachments.value.length > 0) {
      (userMessage as any).attachments = [...pendingAttachments.value]
    }
    messages.value.push(userMessage)

    // 创建助手消息
    const assistantMessage = createAssistantMessage(
      `assistant_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      new Date()
    )
    messages.value.push(assistantMessage)

    await nextTick()
    scrollToBottom(true) // 发送新消息时强制滚动到底部

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

          // 调用后端 RAG 回答接口（支持多集合与自动持久化）
          try {
            const resp = await invoke('assistant_rag_answer', {
              request: { ...baseReq, collection_id: null }, // collection_id in request is ignored if collection_ids is provided
              collection_ids: activeIds.length > 0 ? activeIds : null,
              conversation_id: currentConversationId.value,
              message_id: assistantMessage.id,
              user_message_id: userMessage.id,
            }) as any

            combinedAnswer = resp?.answer || ''
            combinedCitations = resp?.citations || []
            fallbackReason = resp?.fallback_reason
          } catch (e) {
            console.warn('RAG回答生成失败', e)
            fallbackReason = 'RAG服务调用失败'
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
            } else if (fallbackReason.includes('RAG检索失败') || fallbackReason.includes('RAG服务调用失败')) {
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

          // 消息已由后端持久化，前端无需再次保存
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
              attachments: pendingAttachments.value.length ? pendingAttachments.value : undefined,
            } : {
              conversation_id: currentConversationId.value,
              message: userInput,
              service_name: 'default',
              message_id: assistantMessage.id,
              attachments: pendingAttachments.value.length ? pendingAttachments.value : undefined,
            },
            }) as string
          // Align local ids with server-acknowledged id to ensure consistency
          if (returnedMessageId && typeof returnedMessageId === 'string') {
            assistantMessage.id = returnedMessageId
          }
          // 清空待发送附件；本轮已提交给后端
          pendingAttachments.value = []
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
  console.log('[AIChat] ========== 停止执行被调用 ==========')
  console.log('[AIChat] 当前执行ID:', currentExecutionId.value)
  console.log('[AIChat] 当前会话ID:', currentConversationId.value)
  console.log('[AIChat] isLoading状态:', isLoading.value)
  
  // 必须有 execution_id 才能停止
  if (!currentExecutionId.value) {
    console.warn('[AIChat] ⚠️ 没有执行ID，无法停止')
    // 如果没有执行ID，尝试使用会话ID
    if (currentConversationId.value) {
      console.log('[AIChat] 尝试使用会话ID停止:', currentConversationId.value)
      try {
      await invoke('stop_execution', {
          execution_id: currentConversationId.value,
        })
        console.log('[AIChat] ✅ 使用会话ID停止成功')
      } catch (error) {
        console.error('[AIChat] ❌ 使用会话ID停止失败:', error)
      }
    }
  } else {
    // 使用 execution_id 停止
    try {
      console.log('[AIChat] 🛑 正在停止执行，execution_id:', currentExecutionId.value)
      const result = await invoke('stop_execution', {
        execution_id: currentExecutionId.value,
      })
      console.log('[AIChat] ✅ stop_execution 命令成功，返回:', result)
  } catch (error) {
      console.error('[AIChat] ❌ stop_execution 失败:', error)
    }
  }

  // 额外调用取消流命令作为备用（使用当前会话ID）
  if (currentConversationId.value) {
    try {
      console.log('[AIChat] 📡 调用 cancel_ai_stream，会话ID:', currentConversationId.value)
      await invoke('cancel_ai_stream', {
        conversationId: currentConversationId.value,
      })
      console.log('[AIChat] ✅ cancel_ai_stream 成功')
    } catch (error) {
      console.error('[AIChat] ❌ cancel_ai_stream 失败:', error)
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

const copiedMessageId = ref<string | null>(null)
let copyTimer: number | null = null
const copyMessage = async (msg: ChatMessage) => {
  const content = msg.content
  try {
    await navigator.clipboard.writeText(content)
    copiedMessageId.value = msg.id
    if (copyTimer) { clearTimeout(copyTimer) }
    copyTimer = window.setTimeout(() => { copiedMessageId.value = null }, 1500)
  } catch (error) {
    const textArea = document.createElement('textarea')
    textArea.value = content
    document.body.appendChild(textArea)
    textArea.select()
    document.execCommand('copy')
    document.body.removeChild(textArea)
    copiedMessageId.value = msg.id
    if (copyTimer) { clearTimeout(copyTimer) }
    copyTimer = window.setTimeout(() => { copiedMessageId.value = null }, 1500)
  }
}

const resendMessage = async (userMessage: ChatMessage) => {
  if (isLoading.value) return

  // Remove previous assistant response if exists
  try {
    const idx = messages.value.findIndex(m => m.id === userMessage.id)
    if (idx !== -1) {
      const next = messages.value[idx + 1]
      if (next && next.role === 'assistant') {
        const assistantId = next.id
        messages.value.splice(idx + 1, 1)
        try { orderedMessages.processor.cleanup(assistantId) } catch {}
        try { await invoke('delete_ai_message', { message_id: assistantId }) } catch {}
      }
      const userId = userMessage.id
      messages.value.splice(idx, 1)
      try { orderedMessages.processor.cleanup(userId) } catch {}
      try { await invoke('delete_ai_message', { message_id: userId }) } catch {}
    }
  } catch {}

  inputMessage.value = userMessage.content
  await sendMessage()
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

  // 首次打开时强制滚动到底部
  nextTick(() => {
    scrollToBottom(true)
  })
})

// 同步全局会话消息到本地列表（不再转换类型）
watch(
  () => ({
    msgs: conversationMessages.value,
    cid: currentConversationId.value,
  }),
  ({ msgs, cid }, oldVal) => {
    if (Array.isArray(msgs)) {
      messages.value = msgs as ChatMessage[]
      // 如果是切换会话，强制滚动到底部；否则智能滚动
      const isConversationSwitch = oldVal && cid !== oldVal.cid
      nextTick(() => scrollToBottom(isConversationSwitch))
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
  
  // 清理滚动定时器
  if (scrollTimeout.value) {
    clearTimeout(scrollTimeout.value)
    scrollTimeout.value = null
  }
  
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
