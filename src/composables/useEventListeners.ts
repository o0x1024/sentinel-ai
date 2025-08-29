import { ref, Ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { v4 as uuidv4 } from 'uuid'
import { useConversation } from './useConversation'

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
  execution_id?: string
  conversation_id?: string
  is_complete?: boolean
  tool_calls?: any[]
  tool_outputs?: any[]
}

export const useEventListeners = (
  messages: Ref<ChatMessage[]>,
  currentExecutionId: Ref<string | null>,
  currentConversationId: Ref<string | null>,
  streamStartTime: Ref<number | null>,
  streamCharCount: Ref<number>,
  emitHandlers: any,
  saveMessagesToConversation: (messages: ChatMessage[]) => Promise<void>
) => {
  const unlistenCallbacks: (() => void)[] = []

  // Extract emit functions from the handlers object
  const mainEmit = emitHandlers['execution-started'] ? emitHandlers : null
  const streamCompletedEmit = emitHandlers['stream-completed']
  const streamErrorEmit = emitHandlers['stream-error']

  const setupEventListeners = async () => {
    // Stream message listener
    const unlistenStream = await listen('ai_stream_message', event => {
      handleStreamMessage(event.payload as any)
    })

    // Stream start listener
    const unlistenStreamStart = await listen('ai_stream_start', event => {
      handleStreamStart(event.payload as any)
    })

    // Stream error listener
    const unlistenStreamError = await listen('ai_stream_error', event => {
      handleStreamError(event.payload as any)
    })

    // Stream completed listener
    const unlistenStreamCompleted = await listen('ai_stream_completed', event => {
      handleStreamCompleted(event.payload as any)
    })

    unlistenCallbacks.push(
      unlistenStream,
      unlistenStreamStart,
      unlistenStreamError,
      unlistenStreamCompleted
    )
  }

  const handleStreamCompleted = async (data: any) => {
    console.log('Stream completed event:', data)
    const targetMessage = data.message_id
      ? messages.value.find(m => m.id === data.message_id)
      : messages.value.filter(m => m.role === 'assistant').pop()

    if (targetMessage) {
      targetMessage.isStreaming = false

      // Save assistant message
      await saveMessagesToConversation([targetMessage])
    }
  }

  const handleStreamMessage = (data: any) => {
    console.log('Unified stream message event:', data)

    let targetMessage = messages.value.find(
      m =>
        (data.message_id && m.id === data.message_id) ||
        (data.execution_id && m.execution_id === data.execution_id) ||
        (data.conversation_id && m.role === 'assistant' && !m.execution_id) // Fallback for general conversation messages
    )

    // If no message is found and we have an execution_id, create a new message.
    // This handles cases where the first stream event is not a content delta (e.g., a tool update).
    if (!targetMessage && data.execution_id) {
      const newMessage: ChatMessage = {
        id: uuidv4(), // The message itself still needs a unique ID
        conversation_id: data.conversation_id || currentConversationId.value,
        role: 'assistant',
        content: '',
        timestamp: new Date(),
        isStreaming: false,
        execution_id: data.execution_id,
        is_complete: false,
        tool_calls: [],
        tool_outputs: [],
      }
      messages.value.push(newMessage)
      targetMessage = newMessage // Re-find the message to proceed
    }

    if (!targetMessage) {
      console.warn('No target message found for stream data and could not create one:', data)
      return
    }

    streamCharCount.value += (data.content_delta || '').length

    targetMessage.isStreaming = !data.is_complete

    switch (data.message_type) {
      case 'Content':
        if (data.content_delta) {
          if (!targetMessage.content) {
            targetMessage.content = ''
          }
          targetMessage.content += data.content_delta
        }
        saveMessagesToConversation([targetMessage]).catch(console.error)
        break

      case 'ToolUpdate':
        if (data.tool_execution) {
          if (!targetMessage.toolExecutions) {
            targetMessage.toolExecutions = []
          }
          const existingTool = targetMessage.toolExecutions.find(
            (t: any) => t.id === data.tool_execution.id
          )

          // Map backend data to frontend format
          const toolExecutionData = {
            ...data.tool_execution,
            tool: data.tool_execution.name, // Ensure 'tool' property is set for the UI
          }

          if (existingTool) {
            Object.assign(existingTool, toolExecutionData)
          } else {
            targetMessage.toolExecutions.push(toolExecutionData)
          }
        }
        saveMessagesToConversation([targetMessage]).catch(console.error)
        break

      case 'PlanUpdate':
        if (data.execution_plan) {
          targetMessage.executionPlan = data.execution_plan
        }
        break

      case 'FinalResult':
        if (data.final_content) {
          targetMessage.content = data.final_content
        }
        targetMessage.isStreaming = false
        saveMessagesToConversation([targetMessage]).catch(console.error)
        break

      case 'Error':
        targetMessage.hasError = true
        targetMessage.isStreaming = false
        if (data.error) {
          targetMessage.content = `❌ **Error**\n\n${data.error}`
        }
        saveMessagesToConversation([targetMessage]).catch(console.error)
        break
    }

    // scrollToBottom()
  }

  const handleStreamStart = (data: any) => {
    console.log('Stream start event:', data)
    streamStartTime.value = Date.now()
    streamCharCount.value = 0

    if (data.conversation_id === currentConversationId.value && data.message_id) {
      const targetMessage = messages.value.find(m => m.id === data.message_id)
      if (targetMessage) {
        console.log('Found target message for stream start:', targetMessage.id)
        targetMessage.isStreaming = true
        targetMessage.content = ''
        targetMessage.hasError = false
        // scrollToBottom()
      } else {
        console.warn('Target message not found for stream start:', data.message_id)
      }
    }
  }

  const handleStreamError = (data: any) => {
    console.log('Stream error event received:', data)

    if (data.conversation_id === currentConversationId.value || data.message_id) {
      const targetMessage = data.message_id
        ? messages.value.find(m => m.id === data.message_id)
        : messages.value.filter(m => m.role === 'assistant').pop()

      if (targetMessage) {
        console.log('Handling stream error for message:', targetMessage.id)

        // Stop typewriter on error - NO LONGER NEEDED
        // typewriterHandlers.stopTypewriter(targetMessage.id)

        const isConfigError =
          data.error.includes('not configured') ||
          data.error.includes('API key') ||
          data.error.includes('provider') ||
          data.error.includes('configuration')

        if (isConfigError) {
          targetMessage.content = `⚠️ **AI配置问题**\n\n${data.error}\n\n**解决方案：**\n1. 点击左侧导航栏的"设置"\n2. 选择"AI配置"\n3. 配置至少一个AI提供商\n4. 输入有效的API密钥\n5. 保存配置后重试`
        } else {
          targetMessage.content = `❌ **AI响应错误**\n\n${data.error}\n\n💡 **建议：**\n- 检查网络连接\n- 验证API密钥是否有效\n- 点击下方"重新发送"按钮重试`
        }

        targetMessage.isStreaming = false
        targetMessage.hasError = true

        streamStartTime.value = null
        streamCharCount.value = 0
        // scrollToBottom()
      }

      // Always emit stream-error event to reset loading state
      if (streamErrorEmit && typeof streamErrorEmit === 'function') {
        streamErrorEmit({
          messageId: data.message_id || targetMessage?.id,
          error: data.error,
          conversationId: data.conversation_id,
        })
      }

      if (targetMessage) {
        saveMessagesToConversation([targetMessage]).catch(console.error)
      }
    }
  }

  const cleanup = () => {
    unlistenCallbacks.forEach(callback => callback())
    unlistenCallbacks.length = 0
  }

  return {
    setupEventListeners,
    cleanup,
  }
}
