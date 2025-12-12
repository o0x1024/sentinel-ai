/**
 * Agent 事件监听 Composable
 * 监听后端 Agent 执行事件
 */

import { ref, onMounted, onUnmounted, type Ref, type ComputedRef, computed } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AgentMessage, MessageType } from '@/types/agent'

// 后端发送的 agent:start 事件
interface AgentStartEvent {
  execution_id: string
  task: string
}

// 后端发送的 agent:chunk 事件
interface AgentChunkEvent {
  execution_id: string
  chunk_type: string  // 'text' | 'reasoning'
  content: string
}

// 后端发送的 agent:tool_call 事件（旧格式兼容）
interface AgentToolCallEvent {
  execution_id: string
  tool_id: string
  tool_name: string
  tool_input: any
}

// 后端发送的 agent:tool_call_complete 事件（新格式 - rig-core）
interface AgentToolCallCompleteEvent {
  execution_id: string
  tool_call_id: string
  tool_name: string
  arguments: string  // JSON 字符串格式的参数
}

// 后端发送的 agent:tool_result 事件（旧格式兼容）
interface AgentToolResultEvent {
  execution_id: string
  tool_name: string
  tool_input: any
  tool_result: string
}

// 后端发送的 agent:tool_result 事件（新格式 - rig-core）
interface AgentToolResultNewEvent {
  execution_id: string
  tool_call_id: string
  result: string  // JSON 字符串格式的结果
}

// 后端发送的 agent:tools_selected 事件
interface AgentToolsSelectedEvent {
  execution_id: string
  tools: string[]
}

// 后端发送的 agent:tool_executed 事件
interface AgentToolExecutedEvent {
  execution_id: string
  tool: string
  arguments: any
  result: string
  success: boolean
  iteration: number
}

// 后端发送的 agent:iteration 事件
interface AgentIterationEvent {
  execution_id: string
  iteration: number
  max_iterations: number
}

// 后端发送的 agent:complete 事件
interface AgentCompleteEvent {
  execution_id: string
  success: boolean
  response?: string
}

// 后端发送的 agent:error 事件
interface AgentErrorEvent {
  execution_id: string
  error: string
}

// 后端发送的 OrderedMessageChunk 结构 (兼容旧格式)
interface OrderedMessageChunk {
  execution_id: string
  message_id: string
  conversation_id?: string
  sequence: number
  chunk_type: string
  content: string
  timestamp: { secs_since_epoch: number; nanos_since_epoch: number }
  is_final: boolean
  stage?: string
  tool_name?: string
  architecture?: string
  structured_data?: any
}

// RAG元信息
interface RagMetaInfo {
  rag_applied: boolean
  rag_sources_used: boolean
  source_count: number
}

export interface UseAgentEventsReturn {
  messages: Ref<AgentMessage[]>
  isExecuting: Ref<boolean>
  currentExecutionId: Ref<string | null>
  error: Ref<string | null>
  streamingContent: Ref<string>
  hasMessages: ComputedRef<boolean>
  lastMessage: ComputedRef<AgentMessage | undefined>
  ragMetaInfo: Ref<RagMetaInfo | null>
  clearMessages: () => void
  stopExecution: () => void
  startListening: () => Promise<void>
  stopListening: () => void
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
  const contentBuffer = ref('')
  const ragMetaInfo = ref<RagMetaInfo | null>(null)

  // 工具调用追踪 Map: tool_call_id -> { tool_name, arguments, message_id, message_index }
  const toolCallTracker = new Map<string, { tool_name: string; arguments: any; message_id: string; message_index: number }>()

  const unlisteners: UnlistenFn[] = []

  const getTargetId = (): string | undefined => {
    if (!executionId) return undefined
    return typeof executionId === 'string' ? executionId : executionId.value
  }

  const matchesTarget = (eventExecId: string): boolean => {
    const targetId = getTargetId()
    return !targetId || eventExecId === targetId
  }

  const hasMessages = computed(() => messages.value.length > 0)
  const lastMessage = computed(() => messages.value[messages.value.length - 1])

  const clearMessages = () => {
    messages.value = []
    streamingContent.value = ''
    contentBuffer.value = ''
    error.value = null
    isExecuting.value = false
    currentExecutionId.value = null
    ragMetaInfo.value = null
  }

  // 停止执行：清空流式内容并更新状态
  const stopExecution = () => {
    console.log('[useAgentEvents] Stopping execution, current execution_id:', currentExecutionId.value)
    isExecuting.value = false
    streamingContent.value = ''
    contentBuffer.value = ''

    // 如果有正在流式输出的内容，将其作为最终消息添加
    // 注意：后端取消后可能不会发送 complete 事件，所以这里处理残留内容
  }

  const startListening = async () => {
    // 监听用户消息事件（从后端保存后推送）
    const unlistenUserMessage = await listen<{
      execution_id: string
      message_id: string
      content: string
      timestamp: number
    }>('agent:user_message', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      isExecuting.value = true
      currentExecutionId.value = payload.execution_id
      error.value = null
      contentBuffer.value = ''
      streamingContent.value = ''

      // 添加用户消息
      messages.value.push({
        id: payload.message_id,
        type: 'user',
        content: payload.content,
        timestamp: payload.timestamp,
      })
    })
    unlisteners.push(unlistenUserMessage)

    // 监听 agent:start 事件（兼容旧版）
    const unlistenStart = await listen<AgentStartEvent>('agent:start', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      isExecuting.value = true
      currentExecutionId.value = payload.execution_id
      error.value = null
      contentBuffer.value = ''
      streamingContent.value = ''

      // 添加用户任务消息（如果没有通过 user_message 事件收到）
      const hasUserMessage = messages.value.some(m =>
        m.type === 'user' && m.content === payload.task
      )
      if (!hasUserMessage) {
        messages.value.push({
          id: crypto.randomUUID(),
          type: 'user',
          content: payload.task,
          timestamp: Date.now(),
        })
      }
    })
    unlisteners.push(unlistenStart)

    // 监听 agent:iteration 事件
    const unlistenIteration = await listen<AgentIterationEvent>('agent:iteration', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      messages.value.push({
        id: crypto.randomUUID(),
        type: 'progress',
        content: `Iteration ${payload.iteration}/${payload.max_iterations}`,
        timestamp: Date.now(),
        metadata: {
          step_index: payload.iteration,
          total_steps: payload.max_iterations,
        }
      })
    })
    unlisteners.push(unlistenIteration)

    // 监听 agent:chunk 事件
    const unlistenChunk = await listen<AgentChunkEvent>('agent:chunk', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      if (payload.chunk_type === 'text') {
        contentBuffer.value += payload.content
        streamingContent.value = contentBuffer.value
      } else if (payload.chunk_type === 'reasoning') {
        messages.value.push({
          id: crypto.randomUUID(),
          type: 'thinking',
          content: payload.content,
          timestamp: Date.now(),
        })
      }
    })
    unlisteners.push(unlistenChunk)

    // 监听 agent:tool_call 事件
    const unlistenToolCall = await listen<AgentToolCallEvent>('agent:tool_call', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      messages.value.push({
        id: crypto.randomUUID(),
        type: 'tool_call',
        content: `Calling tool: ${payload.tool_name}`,
        timestamp: Date.now(),
        metadata: {
          tool_name: payload.tool_name,
          tool_args: payload.tool_input,
        }
      })
    })
    unlisteners.push(unlistenToolCall)

    // 监听 agent:tool_call_complete 事件（新格式 - rig-core）
    const unlistenToolCallComplete = await listen<AgentToolCallCompleteEvent>('agent:tool_call_complete', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      // 解析参数 JSON
      let parsedArgs: any = {}
      try {
        parsedArgs = JSON.parse(payload.arguments || '{}')
      } catch (e) {
        parsedArgs = { raw: payload.arguments }
      }

      const messageId = crypto.randomUUID()
      const messageIndex = messages.value.length  // 记录消息索引便于后续更新

      // 保存到追踪 Map，用于后续关联结果
      toolCallTracker.set(payload.tool_call_id, {
        tool_name: payload.tool_name,
        arguments: parsedArgs,
        message_id: messageId,
        message_index: messageIndex,
      })

      messages.value.push({
        id: messageId,
        type: 'tool_call',
        content: `正在调用工具: ${payload.tool_name}`,
        timestamp: Date.now(),
        metadata: {
          tool_name: payload.tool_name,
          tool_args: parsedArgs,
          tool_call_id: payload.tool_call_id,
          status: 'running',
          execution_id: payload.execution_id,
        }
      })
    })
    unlisteners.push(unlistenToolCallComplete)

    // 监听 agent:tool_result 事件（旧格式兼容）
    const unlistenToolResult = await listen<AgentToolResultEvent>('agent:tool_result', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      // 检查是否是新格式（有 tool_call_id 而没有 tool_name）
      const newPayload = payload as any
      if (newPayload.tool_call_id && !newPayload.tool_name) {
        // 新格式：从追踪 Map 获取工具信息
        const callInfo = toolCallTracker.get(newPayload.tool_call_id)

        // 解析结果 JSON
        let resultContent = newPayload.result || ''
        try {
          const parsed = JSON.parse(resultContent)
          resultContent = JSON.stringify(parsed, null, 2)
        } catch (e) {
          // 保持原始字符串
        }

        // 更新原有的 tool_call 消息状态，并将结果合并到该消息中
        if (callInfo) {
          const existingMsg = messages.value.find(m => m.id === callInfo.message_id)
          if (existingMsg && existingMsg.metadata) {
            existingMsg.metadata.status = 'completed'
            existingMsg.metadata.tool_result = resultContent
            existingMsg.metadata.success = !resultContent.toLowerCase().includes('error')
            existingMsg.content = `工具调用完成: ${callInfo.tool_name}`
          }
        }

        // 从追踪 Map 中移除（不再创建单独的 tool_result 消息）
        toolCallTracker.delete(newPayload.tool_call_id)
      } else {
        // 旧格式：尝试合并到最近的匹配 tool_call 消息
        const matchingToolCall = messages.value.slice().reverse().find(m =>
          m.type === 'tool_call' &&
          m.metadata?.tool_name === payload.tool_name &&
          !m.metadata?.tool_result  // 还没有结果的
        )

        if (matchingToolCall && matchingToolCall.metadata) {
          matchingToolCall.metadata.status = 'completed'
          matchingToolCall.metadata.tool_result = payload.tool_result
          matchingToolCall.metadata.success = !payload.tool_result.startsWith('Error:')
          matchingToolCall.content = `工具调用完成: ${payload.tool_name}`
        } else {
          // 找不到匹配的 tool_call，创建独立消息（兜底）
          messages.value.push({
            id: crypto.randomUUID(),
            type: 'tool_result',
            content: payload.tool_result,
            timestamp: Date.now(),
            metadata: {
              tool_name: payload.tool_name,
              tool_args: payload.tool_input,
              success: !payload.tool_result.startsWith('Error:'),
            }
          })
        }
      }
    })
    unlisteners.push(unlistenToolResult)

    // 监听 agent:tools_selected 事件（仅记录日志，不显示消息）
    const unlistenToolsSelected = await listen<AgentToolsSelectedEvent>('agent:tools_selected', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      // 仅记录日志，不再添加到消息列表显示
      console.log(`[Agent] Selected ${payload.tools.length} tools:`, payload.tools)
    })
    unlisteners.push(unlistenToolsSelected)

    // 监听 agent:tool_executed 事件
    const unlistenToolExecuted = await listen<AgentToolExecutedEvent>('agent:tool_executed', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      messages.value.push({
        id: crypto.randomUUID(),
        type: 'tool_result',
        content: payload.result,
        timestamp: Date.now(),
        metadata: {
          tool_name: payload.tool,
          tool_args: payload.arguments,
          success: payload.success,
          iteration: payload.iteration,
        }
      })
    })
    unlisteners.push(unlistenToolExecuted)

    // 监听助手消息保存成功事件
    const unlistenAssistantSaved = await listen<{
      execution_id: string
      message_id: string
      content: string
      timestamp: number
    }>('agent:assistant_message_saved', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      console.log('[useAgentEvents] Assistant message saved:', payload.message_id)

      // 检测是否引用了知识库内容
      if (ragMetaInfo.value?.rag_applied) {
        const sourcePattern = /\[SOURCE\s+\d+\]/gi
        const matches = payload.content.match(sourcePattern)
        if (matches && matches.length > 0) {
          ragMetaInfo.value.rag_sources_used = true
          ragMetaInfo.value.source_count = matches.length
          console.log(`[useAgentEvents] Detected ${matches.length} knowledge base citations`)
        } else {
          console.log('[useAgentEvents] RAG enabled but no citations found in response')
        }
      }

      // 检查是否已经有这条消息（通过ID或内容）
      const existingById = messages.value.find(m => m.id === payload.message_id)
      const existingByContent = messages.value.find(m =>
        m.type === 'final' && m.content === payload.content
      )

      if (!existingById && !existingByContent) {
        // 如果没有，添加最终消息
        messages.value.push({
          id: payload.message_id,
          type: 'final',
          content: payload.content,
          timestamp: payload.timestamp,
          metadata: ragMetaInfo.value ? { rag_info: ragMetaInfo.value } : undefined,
        })
      }

      // 清空缓冲区，避免 agent:complete 事件重复添加
      contentBuffer.value = ''
      streamingContent.value = ''
    })
    unlisteners.push(unlistenAssistantSaved)

    // 监听 ai_meta_info 事件（RAG等元信息）
    const unlistenMetaInfo = await listen<{
      conversation_id: string
      message_id: string
      rag_applied?: boolean
      web_search_applied?: boolean
      rag_sources_used?: boolean
      source_count?: number
    }>('ai_meta_info', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.conversation_id)) return

      console.log('[useAgentEvents] Meta info received:', payload)

      if (payload.rag_applied) {
        const used = payload.rag_sources_used === true
        const count = typeof payload.source_count === 'number' ? payload.source_count : 0
        ragMetaInfo.value = {
          rag_applied: true,
          rag_sources_used: used,
          source_count: count
        }
      }
    })
    unlisteners.push(unlistenMetaInfo)

    // 监听 agent:complete 事件
    const unlistenComplete = await listen<AgentCompleteEvent>('agent:complete', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      isExecuting.value = false
      streamingContent.value = ''

      // 只有当缓冲区有内容时才尝试添加消息
      // 注意：agent:assistant_message_saved 事件已经会清空缓冲区，
      // 所以如果消息已经通过那个事件添加，这里的 contentBuffer 应该是空的
      const finalContent = contentBuffer.value.trim()
      if (finalContent) {
        // 再次检查是否已有相同内容的消息（双重保险）
        const hasAssistantMessage = messages.value.some(m =>
          m.type === 'final' && m.content === finalContent
        )

        if (!hasAssistantMessage) {
          // 检测流式内容中的知识库引用
          if (ragMetaInfo.value?.rag_applied && !ragMetaInfo.value.rag_sources_used) {
            const sourcePattern = /\[SOURCE\s+\d+\]/gi
            const matches = finalContent.match(sourcePattern)
            if (matches && matches.length > 0) {
              ragMetaInfo.value.rag_sources_used = true
              ragMetaInfo.value.source_count = matches.length
              console.log(`[useAgentEvents] Detected ${matches.length} knowledge base citations in stream`)
            }
          }

          messages.value.push({
            id: crypto.randomUUID(),
            type: 'final',
            content: finalContent,
            timestamp: Date.now(),
            metadata: ragMetaInfo.value ? { rag_info: ragMetaInfo.value } : undefined,
          })
        }
      }

      // 始终清空缓冲区
      contentBuffer.value = ''
    })
    unlisteners.push(unlistenComplete)

    // 监听 agent:error 事件
    const unlistenError = await listen<AgentErrorEvent>('agent:error', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      error.value = payload.error
      isExecuting.value = false

      messages.value.push({
        id: crypto.randomUUID(),
        type: 'error',
        content: payload.error,
        timestamp: Date.now(),
      })
    })
    unlisteners.push(unlistenError)

    // 兼容旧的 message_chunk 事件
    const unlistenOldChunk = await listen<OrderedMessageChunk>('message_chunk', (event) => {
      const chunk = event.payload
      if (!matchesTarget(chunk.execution_id)) return

      if (!isExecuting.value) {
        isExecuting.value = true
        currentExecutionId.value = chunk.execution_id
      }

      const chunkType = chunk.chunk_type

      if (chunkType === 'Meta' && chunk.stage === 'start') {
        isExecuting.value = true
        currentExecutionId.value = chunk.execution_id
        error.value = null
        return
      }

      // Meta complete 和 StreamComplete 只清理状态，不添加消息（由 assistant_message_saved 处理）
      // 注意：不清空 contentBuffer，以便在 assistant_message_saved 未到达时，agent:complete 可以作为后备
      if (chunkType === 'Meta' && chunk.stage === 'complete') {
        isExecuting.value = false
        streamingContent.value = ''
        // 不添加消息，等待 assistant_message_saved 事件
        return
      }

      if (chunkType === 'StreamComplete' || chunk.is_final) {
        isExecuting.value = false
        streamingContent.value = ''
        // 不添加消息，等待 assistant_message_saved 事件
        return
      }

      if (chunkType === 'Error') {
        error.value = chunk.content
        messages.value.push({
          id: crypto.randomUUID(),
          type: 'error',
          content: chunk.content,
          timestamp: Date.now(),
        })
        isExecuting.value = false
        return
      }

      if (chunkType === 'Content') {
        contentBuffer.value += chunk.content
        streamingContent.value = contentBuffer.value
        return
      }

      if (chunkType === 'Thinking') {
        if (chunk.content.trim()) {
          messages.value.push({
            id: crypto.randomUUID(),
            type: 'thinking',
            content: chunk.content,
            timestamp: Date.now(),
          })
        }
        return
      }
    })
    unlisteners.push(unlistenOldChunk)

    // 监听 agent:cancelled 事件（用户取消执行）
    const unlistenCancelled = await listen<{
      execution_id: string
      message: string
    }>('agent:cancelled', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      console.log('[useAgentEvents] Execution cancelled:', payload.execution_id)

      isExecuting.value = false
      streamingContent.value = ''
      contentBuffer.value = ''

      // 可选：添加一条取消消息到聊天记录
      // messages.value.push({
      //   id: crypto.randomUUID(),
      //   type: 'system',
      //   content: '执行已被用户取消',
      //   timestamp: Date.now(),
      // })
    })
    unlisteners.push(unlistenCancelled)
  }

  const stopListening = () => {
    unlisteners.forEach(unlisten => unlisten())
    unlisteners.length = 0
  }

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
    ragMetaInfo,
    clearMessages,
    stopExecution,
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
