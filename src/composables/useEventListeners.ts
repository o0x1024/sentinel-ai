import { ref, Ref } from 'vue'
import { listen } from '@tauri-apps/api/event'

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
}

export const useEventListeners = (
  messages: Ref<ChatMessage[]>,
  currentExecutionId: Ref<string | null>,
  currentConversationId: Ref<string | null>,
  streamStartTime: Ref<number | null>,
  streamCharCount: Ref<number>,
  typewriterHandlers: any,
  emitHandlers: any,
  scrollToBottom: () => void,
  saveMessagesToConversation: (messages: ChatMessage[]) => Promise<void>
) => {
  const unlistenCallbacks: (() => void)[] = []
  
  // Extract emit functions from the handlers object
  const mainEmit = emitHandlers['execution-started'] ? emitHandlers : null
  const streamCompletedEmit = emitHandlers['stream-completed']
  const streamErrorEmit = emitHandlers['stream-error']

  const setupEventListeners = async () => {
    // Execution progress listener
    const unlistenProgress = await listen('execution_progress', (event) => {
      const data = event.payload as any
      if (data.execution_id === currentExecutionId.value) {
        if (mainEmit && typeof mainEmit === 'function') {
          mainEmit('execution-progress', data.progress || data.percentage || 0)
        }
        
        const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
        if (lastAssistantMessage) {
          updateExecutionProgress(lastAssistantMessage, data)
        }
        scrollToBottom()
      }
    })

    // Execution completed listener
    const unlistenComplete = await listen('execution_completed', (event) => {
      const data = event.payload as any
      if (data.execution_id === currentExecutionId.value) {
        const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
        if (lastAssistantMessage) {
          updateExecutionComplete(lastAssistantMessage, data)
        }
        
        if (mainEmit && typeof mainEmit === 'function') {
          mainEmit('execution-completed', data.result || data)
        }
        currentExecutionId.value = null
        scrollToBottom()
      }
    })

    // Stream message listener
    const unlistenStream = await listen('ai_stream_message', (event) => {
      handleStreamMessage(event.payload as any)
    })

    // Stream start listener
    const unlistenStreamStart = await listen('ai_stream_start', (event) => {
      handleStreamStart(event.payload as any)
    })

    // Stream error listener
    const unlistenStreamError = await listen('ai_stream_error', (event) => {
      handleStreamError(event.payload as any)
    })

    // Stream complete listener
    const unlistenStreamComplete = await listen('ai_stream_complete', (event) => {
      handleStreamComplete(event.payload as any)
    })

    // Steps initialization listener
    const unlistenStepsInit = await listen('execution_steps_initialized', (event) => {
      handleStepsInit(event.payload as any)
    })

    // Step start listener
    const unlistenStepStart = await listen('execution_step_started', (event) => {
      handleStepStart(event.payload as any)
    })

    // Step complete listener
    const unlistenStepComplete = await listen('execution_step_completed', (event) => {
      handleStepComplete(event.payload as any)
    })

    // Tool execution started listener
    const unlistenToolExecStart = await listen('tool_execution_started', (event) => {
      handleToolExecutionStarted(event.payload as any)
    })

    // Tool step started listener
    const unlistenToolStepStart = await listen('tool_step_started', (event) => {
      handleToolStepStarted(event.payload as any)
    })

    // Tool step completed listener
    const unlistenToolStepComplete = await listen('tool_step_completed', (event) => {
      handleToolStepCompleted(event.payload as any)
    })

    // Tool execution completed listener
    const unlistenToolExecComplete = await listen('tool_execution_completed', (event) => {
      handleToolExecutionCompleted(event.payload as any)
    })

    unlistenCallbacks.push(
      unlistenProgress,
      unlistenComplete,
      unlistenStream,
      unlistenStreamStart,
      unlistenStreamError,
      unlistenStreamComplete,
      unlistenStepsInit,
      unlistenStepStart,
      unlistenStepComplete,
      unlistenToolExecStart,
      unlistenToolStepStart,
      unlistenToolStepComplete,
      unlistenToolExecComplete
    )
  }

  const updateExecutionProgress = (message: ChatMessage, data: any) => {
    message.executionProgress = data.progress || data.percentage || 0
    message.currentStep = data.current_step || data.step_name
    message.totalSteps = data.total_steps
    message.completedSteps = data.completed_steps

    if (data.tool_execution) {
      if (!message.toolExecutions) {
        message.toolExecutions = []
      }
      const existingTool = message.toolExecutions.find((t: any) => t.id === data.tool_execution.id)
      if (existingTool) {
        Object.assign(existingTool, data.tool_execution)
      } else {
        message.toolExecutions.push(data.tool_execution)
      }
    }

    if (data.step_update && message.executionPlan?.steps) {
      const step = message.executionPlan.steps.find((s: any) => s.id === data.step_update.id)
      if (step) {
        Object.assign(step, data.step_update)
      }
    }
  }

  const updateExecutionComplete = (message: ChatMessage, data: any) => {
    message.isStreaming = false
    message.executionResult = data.result || data
    message.content = data.final_response || data.response || message.content
    message.executionProgress = 100

    if (message.executionPlan?.steps && data.result) {
      const stepResults = data.result.step_results || 
                         data.result.data?.step_results || 
                         {}

      message.executionPlan.steps.forEach((step: any) => {
        const stepResult = stepResults[step.id] || stepResults[step.name]
        if (stepResult) {
          step.status = stepResult.status || 'completed'
          step.started_at = stepResult.started_at
          step.completed_at = stepResult.completed_at
          step.result_data = stepResult.result_data
          step.result = stepResult.result_data
          step.error = stepResult.error
        } else {
          step.status = step.status === 'failed' ? 'failed' : 'completed'
        }
      })

      message.completedSteps = message.executionPlan.steps.filter((s: any) => s.status === 'completed').length
    }

    if (currentConversationId.value) {
      saveMessagesToConversation([message]).catch(console.error)
    }
  }

  const handleStreamMessage = (data: any) => {
    console.log('Stream message event:', {
      messageId: data.message_id,
      conversationId: data.conversation_id,
      isIncremental: data.is_incremental,
      isComplete: data.is_complete,
      contentDelta: data.content_delta,
      contentLength: (data.content || '').length
    })
    
    streamCharCount.value += (data.content_delta || data.content || '').length

    if (data.execution_id === currentExecutionId.value) {
      const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
      if (lastAssistantMessage) {
        if (data.is_complete) {
          typewriterHandlers.stopTypewriter(lastAssistantMessage.id)
          lastAssistantMessage.content = data.content || typewriterHandlers.getFinalContentFromTypewriterState(lastAssistantMessage.id) || ''
          lastAssistantMessage.isStreaming = false
        } else {
          if (data.is_incremental && data.content_delta) {
            typewriterHandlers.updateTypewriterContentIncremental(lastAssistantMessage.id, data.content_delta)
          } else if (data.content) {
            typewriterHandlers.updateTypewriterContentIncremental(lastAssistantMessage.id, data.content)
          }
        }
      }
      scrollToBottom()
    }

    if (data.conversation_id === currentConversationId.value && data.message_id) {
      const targetMessage = messages.value.find(m => m.id === data.message_id)
      if (targetMessage) {
        handleMessageStreamUpdate(targetMessage, data)
      } else {
        console.warn('Target message not found for stream update:', data.message_id)
      }
    }
  }

  const handleMessageStreamUpdate = (targetMessage: ChatMessage, data: any) => {
    const newContent = data.content || ''
    targetMessage.isStreaming = true

    if (data.is_complete) {
      completeStreamMessage(targetMessage, newContent, data.message_id)
    } else {
      if (data.is_incremental && data.content_delta) {
        // For incremental updates, add the delta content
        typewriterHandlers.updateTypewriterContentIncremental(data.message_id, data.content_delta)
      } else if (newContent) {
        // For non-incremental updates, treat the full content as incremental
        typewriterHandlers.updateTypewriterContentIncremental(data.message_id, newContent)
      }
    }
    scrollToBottom()
  }

  const completeStreamMessage = (targetMessage: any, finalContent: string, messageId: string) => {
    const actualFinalContent = typewriterHandlers.getFinalContentFromTypewriterState(messageId) || finalContent || ''
    
    typewriterHandlers.stopTypewriter(messageId)
    targetMessage.content = actualFinalContent
    targetMessage.isStreaming = false

    if (currentConversationId.value) {
      saveMessagesToConversation([targetMessage]).catch(console.error)
    }

    streamStartTime.value = null
    streamCharCount.value = 0
    scrollToBottom()
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
        scrollToBottom()
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
        
        // Stop typewriter on error
        typewriterHandlers.stopTypewriter(targetMessage.id)
        
        const isConfigError = data.error.includes('not configured') || 
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
        scrollToBottom()
      }
      
      // Always emit stream-error event to reset loading state
      if (streamErrorEmit && typeof streamErrorEmit === 'function') {
        streamErrorEmit({ 
          messageId: data.message_id || targetMessage?.id, 
          error: data.error,
          conversationId: data.conversation_id
        })
      }
    }
  }

  const handleStreamComplete = (data: any) => {
    console.log('Stream complete event received:', data)
    
    if (data.conversation_id === currentConversationId.value || data.message_id) {
      const targetMessage = data.message_id 
        ? messages.value.find(m => m.id === data.message_id)
        : messages.value.filter(m => m.role === 'assistant').pop()
        
      if (targetMessage && targetMessage.isStreaming) {
        console.log('Completing stream for message:', targetMessage.id)
        
        // Stop typewriter and finalize content
        typewriterHandlers.stopTypewriter(targetMessage.id)
        targetMessage.isStreaming = false
        
        const actualContent = typewriterHandlers.getFinalContentFromTypewriterState(targetMessage.id) || targetMessage.content || ''
        
        if (!actualContent || actualContent.trim().length === 0) {
          targetMessage.content = `⚠️ **AI返回了空响应**\n\n这可能是由于配置问题导致的。请检查AI配置设置。`
          targetMessage.hasError = true
        } else {
          targetMessage.content = actualContent
        }
        
        if (currentConversationId.value) {
          saveMessagesToConversation([targetMessage]).catch(console.error)
        }
        
        streamStartTime.value = null
        streamCharCount.value = 0
        scrollToBottom()
      }
      
      // Always emit stream-completed event to reset loading state
      if (streamCompletedEmit && typeof streamCompletedEmit === 'function') {
        streamCompletedEmit({ 
          messageId: data.message_id || targetMessage?.id,
          conversationId: data.conversation_id,
          hasError: data.error || false
        })
      }
    }
  }

  const handleStepsInit = (data: any) => {
    if (data.execution_id === currentExecutionId.value) {
      const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
      if (lastAssistantMessage && data.steps) {
        if (!lastAssistantMessage.executionPlan) {
          lastAssistantMessage.executionPlan = {}
        }
        lastAssistantMessage.executionPlan.steps = data.steps.map((step: any) => ({
          id: step.id,
          name: step.name || step.description,
          description: step.description,
          status: step.status || 'pending',
          started_at: step.started_at,
          completed_at: step.completed_at,
          result: step.result,
          error: step.error
        }))
        lastAssistantMessage.totalSteps = data.steps.length
      }
      scrollToBottom()
    }
  }

  const handleStepStart = (data: any) => {
    if (data.execution_id === currentExecutionId.value) {
      const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
      if (lastAssistantMessage && lastAssistantMessage.executionPlan?.steps) {
        const step = lastAssistantMessage.executionPlan.steps.find((s: any) => 
          s.id === data.step_id || s.name === data.step_name)
        if (step) {
          step.status = 'executing'
          step.started_at = data.started_at || Date.now() / 1000
        }
        
        lastAssistantMessage.executionPlan.steps.forEach((s: any) => {
          if (s.id !== data.step_id && s.name !== data.step_name && s.status === 'executing') {
            s.status = 'completed'
            if (!s.completed_at) {
              s.completed_at = Date.now() / 1000
            }
          }
        })
        
        lastAssistantMessage.currentStep = data.step_name
      }
      scrollToBottom()
    }
  }

  const handleStepComplete = (data: any) => {
    if (data.execution_id === currentExecutionId.value) {
      const lastAssistantMessage = messages.value.filter(m => m.role === 'assistant').pop()
      if (lastAssistantMessage && lastAssistantMessage.executionPlan?.steps) {
        const step = lastAssistantMessage.executionPlan.steps.find((s: any) => 
          s.id === data.step_id || s.name === data.step_name)
        if (step) {
          step.status = data.status || 'completed'
          step.completed_at = data.completed_at || Date.now() / 1000
          step.result_data = data.result_data || data.result
          step.result = data.result_data || data.result
          step.error = data.error
        }
        
        const completedSteps = lastAssistantMessage.executionPlan.steps.filter((s: any) => s.status === 'completed').length
        lastAssistantMessage.completedSteps = completedSteps
        if (lastAssistantMessage.totalSteps && lastAssistantMessage.totalSteps > 0) {
          lastAssistantMessage.executionProgress = (completedSteps / lastAssistantMessage.totalSteps) * 100
        }
      }
      scrollToBottom()
    }
  }

  const handleToolExecutionStarted = (data: any) => {
    const targetMessage = messages.value.find(m => 
      m.id === data.message_id || 
      (m.role === 'assistant' && m.timestamp && new Date(m.timestamp).getTime() > Date.now() - 60000)
    )
    
    if (targetMessage) {
      // Initialize tool executions array if not present
      if (!targetMessage.toolExecutions) {
        targetMessage.toolExecutions = []
      }
      
      // Add tool execution plan based on tool_calls data
      if (data.tool_calls && Array.isArray(data.tool_calls)) {
        data.tool_calls.forEach((tool: any, index: number) => {
          targetMessage.toolExecutions.push({
            id: tool.id,
            name: tool.name,
            status: 'pending',
            started_at: null,
            completed_at: null,
            result: null,
            error: null,
            progress: 0,
            step_index: index,
            total_steps: data.tool_calls.length
          })
        })
      }
      
      targetMessage.isStreaming = true
      scrollToBottom()
    }
  }

  const handleToolStepStarted = (data: any) => {
    const targetMessage = messages.value.find(m => 
      m.id === data.message_id || 
      (m.role === 'assistant' && m.timestamp && new Date(m.timestamp).getTime() > Date.now() - 60000)
    )
    
    if (targetMessage && targetMessage.toolExecutions) {
      const toolExecution = targetMessage.toolExecutions.find((t: any) => 
        t.id === data.tool_call_id || t.name === data.tool_name
      )
      
      if (toolExecution) {
        toolExecution.status = 'executing'
        toolExecution.started_at = data.started_at || Date.now() / 1000
        toolExecution.progress = 0
      }
      
      // Update current step indicator
      targetMessage.currentStep = `Executing: ${data.tool_name}`
      
      scrollToBottom()
    }
  }

  const handleToolStepCompleted = (data: any) => {
    const targetMessage = messages.value.find(m => 
      m.id === data.message_id || 
      (m.role === 'assistant' && m.timestamp && new Date(m.timestamp).getTime() > Date.now() - 60000)
    )
    
    if (targetMessage && targetMessage.toolExecutions) {
      const toolExecution = targetMessage.toolExecutions.find((t: any) => 
        t.id === data.tool_call_id || t.name === data.tool_name
      )
      
      if (toolExecution) {
        toolExecution.status = data.status || 'completed'
        toolExecution.completed_at = data.completed_at || Date.now() / 1000
        toolExecution.result = data.result
        toolExecution.error = data.error
        toolExecution.progress = 100
      }
      
      // Update execution progress
      if (data.total_tools && data.step_index !== undefined) {
        const overallProgress = ((data.step_index + 1) / data.total_tools) * 100
        targetMessage.executionProgress = overallProgress
        
        // Update completed steps count
        const completedTools = targetMessage.toolExecutions.filter((t: any) => 
          t.status === 'completed' || t.status === 'failed'
        ).length
        targetMessage.completedSteps = completedTools
        targetMessage.totalSteps = data.total_tools
      }
      
      scrollToBottom()
    }
  }

  const handleToolExecutionCompleted = (data: any) => {
    const targetMessage = messages.value.find(m => 
      m.id === data.message_id || 
      (m.role === 'assistant' && m.timestamp && new Date(m.timestamp).getTime() > Date.now() - 60000)
    )
    
    if (targetMessage) {
      targetMessage.isStreaming = false
      targetMessage.executionProgress = 100
      targetMessage.currentStep = undefined
      
      // Mark all tool executions as completed if they aren't already
      if (targetMessage.toolExecutions) {
        targetMessage.toolExecutions.forEach((tool: any) => {
          if (tool.status === 'pending' || tool.status === 'executing') {
            tool.status = 'completed'
            tool.completed_at = Date.now() / 1000
            tool.progress = 100
          }
        })
        
        targetMessage.completedSteps = targetMessage.toolExecutions.length
        targetMessage.totalSteps = targetMessage.toolExecutions.length
      }
      
      scrollToBottom()
      
      // Save message to conversation if applicable
      if (currentConversationId.value) {
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
    cleanup
  }
}