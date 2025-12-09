/**
 * Agent 事件监听 Composable
 * 监听后端 Agent 执行事件 (message_chunk)
 */

import { ref, onMounted, onUnmounted, type Ref, type ComputedRef, computed } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AgentMessage, MessageType } from '@/types/agent'

// 后端发送的 OrderedMessageChunk 结构
interface OrderedMessageChunk {
  execution_id: string
  message_id: string
  conversation_id?: string
  sequence: number
  chunk_type: string  // 'Content' | 'Thinking' | 'ToolResult' | 'PlanInfo' | 'Error' | 'Meta' | 'StreamComplete'
  content: string
  timestamp: { secs_since_epoch: number; nanos_since_epoch: number }
  is_final: boolean
  stage?: string
  tool_name?: string
  architecture?: string
  structured_data?: any
}

export interface UseAgentEventsReturn {
  // 状态
  messages: Ref<AgentMessage[]>
  isExecuting: Ref<boolean>
  currentExecutionId: Ref<string | null>
  error: Ref<string | null>
  streamingContent: Ref<string>
  
  // 计算属性
  hasMessages: ComputedRef<boolean>
  lastMessage: ComputedRef<AgentMessage | undefined>
  
  // 方法
  clearMessages: () => void
  startListening: () => Promise<void>
  stopListening: () => void
}

/**
 * 将 ChunkType 转换为 MessageType
 */
function chunkTypeToMessageType(chunkType: string, stage?: string): MessageType {
  switch (chunkType) {
    case 'Content':
      return 'final'
    case 'Thinking':
      return 'thinking'
    case 'ToolResult':
      return 'tool_result'
    case 'PlanInfo':
      return 'planning'
    case 'Error':
      return 'error'
    case 'Meta':
      // Meta 类型根据 stage 区分
      if (stage === 'action') return 'tool_call'
      if (stage === 'observation') return 'tool_result'
      if (stage === 'progress') return 'progress'
      return 'thinking'
    default:
      return 'thinking'
  }
}

/**
 * Agent 事件监听
 * @param executionId 可选的执行 ID 过滤
 */
export function useAgentEvents(executionId?: Ref<string> | string): UseAgentEventsReturn {
  const messages = ref<AgentMessage[]>([])
  const isExecuting = ref(false)
  const currentExecutionId = ref<string | null>(null)
  const error = ref<string | null>(null)
  const streamingContent = ref('')
  
  // 用于累积流式内容
  const contentBuffer = ref('')
  
  const unlisteners: UnlistenFn[] = []

  // 获取目标 executionId
  const getTargetId = (): string | undefined => {
    if (!executionId) return undefined
    return typeof executionId === 'string' ? executionId : executionId.value
  }

  // 检查事件是否匹配目标 ID
  const matchesTarget = (eventExecId: string): boolean => {
    const targetId = getTargetId()
    return !targetId || eventExecId === targetId
  }

  // 计算属性
  const hasMessages = computed(() => messages.value.length > 0)
  const lastMessage = computed(() => messages.value[messages.value.length - 1])

  // 清空消息
  const clearMessages = () => {
    messages.value = []
    streamingContent.value = ''
    contentBuffer.value = ''
    error.value = null
    isExecuting.value = false
    currentExecutionId.value = null
  }

  // 开始监听
  const startListening = async () => {
    // 监听 message_chunk 事件（后端实际发送的事件）
    const unlistenChunk = await listen<OrderedMessageChunk>('message_chunk', (event) => {
      const chunk = event.payload
      
      if (!matchesTarget(chunk.execution_id)) return
      
      // 更新执行状态
      if (!isExecuting.value) {
        isExecuting.value = true
        currentExecutionId.value = chunk.execution_id
      }
      
      // 处理不同类型的 chunk
      const chunkType = chunk.chunk_type
      
      // 处理 start 信号
      if (chunkType === 'Meta' && chunk.stage === 'start') {
        isExecuting.value = true
        currentExecutionId.value = chunk.execution_id
        error.value = null
        return
      }
      
      // 处理 complete 信号
      if (chunkType === 'Meta' && chunk.stage === 'complete') {
        isExecuting.value = false
        // 将累积的内容作为最终消息
        if (contentBuffer.value.trim()) {
          const finalMessage: AgentMessage = {
            id: crypto.randomUUID(),
            type: 'final',
            content: contentBuffer.value,
            timestamp: Date.now(),
          }
          messages.value.push(finalMessage)
          contentBuffer.value = ''
        }
        return
      }
      
      // 处理 StreamComplete 类型
      if (chunkType === 'StreamComplete' || chunk.is_final) {
        isExecuting.value = false
        // 将累积的内容作为最终消息
        if (contentBuffer.value.trim()) {
          const finalMessage: AgentMessage = {
            id: crypto.randomUUID(),
            type: 'final',
            content: contentBuffer.value,
            timestamp: Date.now(),
          }
          messages.value.push(finalMessage)
          contentBuffer.value = ''
        }
        return
      }
      
      // 处理错误
      if (chunkType === 'Error') {
        error.value = chunk.content
        const errorMessage: AgentMessage = {
          id: crypto.randomUUID(),
          type: 'error',
          content: chunk.content,
          timestamp: Date.now(),
        }
        messages.value.push(errorMessage)
        isExecuting.value = false
        return
      }
      
      // 累积内容类型的 chunk
      if (chunkType === 'Content') {
        contentBuffer.value += chunk.content
        streamingContent.value = contentBuffer.value
        return
      }
      
      // 处理思考类型
      if (chunkType === 'Thinking') {
        // 思考内容通常较短，直接作为消息添加
        if (chunk.content.trim()) {
          const thinkingMessage: AgentMessage = {
            id: crypto.randomUUID(),
            type: 'thinking',
            content: chunk.content,
            timestamp: Date.now(),
          }
          messages.value.push(thinkingMessage)
        }
        return
      }
      
      // 处理工具调用/结果（Meta with structured_data）
      if (chunkType === 'Meta' && chunk.structured_data) {
        const sd = chunk.structured_data as any
        if (sd.type === 'step' && sd.step?.action) {
          const action = sd.step.action
          const messageType = action.status === 'running' ? 'tool_call' : 'tool_result'
          const toolMessage: AgentMessage = {
            id: crypto.randomUUID(),
            type: messageType,
            content: chunk.content || `Tool: ${action.tool}`,
            timestamp: Date.now(),
            metadata: {
              tool_name: action.tool,
              tool_args: action.args,
              success: action.status === 'completed',
            }
          }
          messages.value.push(toolMessage)
        }
        return
      }
      
      // 处理进度更新
      if (chunkType === 'Meta' && chunk.stage === 'progress') {
        const sd = chunk.structured_data as any
        if (sd) {
          const progressMessage: AgentMessage = {
            id: crypto.randomUUID(),
            type: 'progress',
            content: sd.step_description || chunk.content,
            timestamp: Date.now(),
            metadata: {
              step_index: sd.completed,
              total_steps: sd.total,
            }
          }
          messages.value.push(progressMessage)
        }
        return
      }
    })
    unlisteners.push(unlistenChunk)
  }

  // 停止监听
  const stopListening = () => {
    unlisteners.forEach(unlisten => unlisten())
    unlisteners.length = 0
  }

  // 自动管理生命周期
  onMounted(() => {
    startListening()
  })

  onUnmounted(() => {
    stopListening()
  })

  return {
    messages,
    isExecuting,
    currentExecutionId,
    error,
    streamingContent,
    hasMessages,
    lastMessage,
    clearMessages,
    startListening,
    stopListening,
  }
}

/**
 * 全局 Agent 事件（不过滤 executionId）
 */
export function useGlobalAgentEvents(): UseAgentEventsReturn {
  return useAgentEvents()
}
