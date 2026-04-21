/**
 * Agent 事件监听 Composable
 * 监听后端 Agent 执行事件
 */

import { ref, onMounted, onUnmounted, computed, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AgentMessage, MessageType } from '@/types/agent'
import { useAgentTasks } from '@/composables/useAgentTasks'
import { buildTerminalSessionFingerprint, useTerminal } from '@/composables/useTerminal'
import {
  buildToolsActivatedMessage,
  buildToolsPreview,
} from '@/utils/agentToolActivation'
import { applyFileVerificationStatuses } from '@/components/Agent/fileVerificationSupport'
import type { AgentTasksUpdatePayload } from '@/types/taskRuntime'
import type {
  AgentChunkEvent,
  AgentCompletionGuardFailedEvent,
  AgentExecutionFinishedEvent,
  AgentGlobalSummaryUpdatedEvent,
  AgentIterationEvent,
  AgentRetryEvent,
  AgentSegmentSummaryCreatedEvent,
  AgentStartEvent,
  AgentTenthManCritiqueEvent,
  AgentToolCallCompleteEvent,
  AgentToolCallEvent,
  AgentToolExecutedEvent,
  AgentToolResultEvent,
  AgentToolResultNewEvent,
  AgentToolsActivatedEvent,
  AgentToolsSelectedEvent,
  ContextUsageInfo,
  MemoryRetrievalInfo,
  OrderedMessageChunk,
  RagMetaInfo,
  SubagentDoneEvent,
  SubagentErrorEvent,
  SubagentItem,
  SubagentStartEvent,
  UseAgentEventsOptions,
  UseAgentEventsReturn,
} from '@/composables/useAgentEventTypes'

/**
 * Agent 事件监听
 * @param executionId 可选的执行 ID 过滤
 */
export function useAgentEvents(
  executionId?: Ref<string> | string,
  options?: UseAgentEventsOptions,
): UseAgentEventsReturn {
  const messages = ref<AgentMessage[]>([])
  const isExecuting = ref(false)
  const currentExecutionId = ref<string | null>(null)
  const error = ref<string | null>(null)
  const streamingContent = ref('')
  const contentBuffer = ref('')
  const ragMetaInfo = ref<RagMetaInfo | null>(null)
  const subagents = ref<SubagentItem[]>([])
  const contextUsage = ref<ContextUsageInfo | null>(null)
  const suppressedExecutionId = ref<string | null>(null)

  // Thinking content buffer for incremental display
  const thinkingBuffer = ref('')
  const currentThinkingMessageId = ref<string | null>(null)

  // Assistant streaming message (so message order reflects arrival order)
  const currentAssistantMessageId = ref<string | null>(null)
  // Current assistant segment buffer (reset on tool-call boundaries)
  const assistantSegmentBuffer = ref('')

  // 工具调用追踪 Map: tool_call_id -> { tool_name, arguments, message_id, message_index }
  const toolCallTracker = new Map<string, { tool_name: string; arguments: any; message_id: string; message_index: number }>()

  // Pending document attachments to inject into next user message
  const pendingDocumentAttachments = ref<any[]>([])

  const unlisteners: UnlistenFn[] = []
  const shouldSuppressUserMessages = () => {
    const raw = options?.suppressUserMessages
    if (typeof raw === 'boolean') return raw
    if (raw && typeof raw === 'object' && 'value' in raw) {
      return !!raw.value
    }
    return false
  }

  const resolveDefaultMaxContextTokens = () => {
    const raw = options?.defaultMaxContextTokens
    if (typeof raw === 'number') {
      return Number.isFinite(raw) && raw > 0 ? Math.floor(raw) : 128000
    }
    if (raw && typeof raw === 'object' && 'value' in raw) {
      const value = Number(raw.value)
      return Number.isFinite(value) && value > 0 ? Math.floor(value) : 128000
    }
    return 128000
  }

  const buildContextUsageSkeleton = (): ContextUsageInfo => ({
    usedTokens: 0,
    maxTokens: resolveDefaultMaxContextTokens(),
    usagePercentage: 0,
    systemPromptTokens: 0,
    historyTokens: 0,
    historyCount: 0,
    summaryTokens: 0,
    summaryGlobalTokens: 0,
    summarySegmentTokens: 0,
    summarySegmentCount: 0,
    memoryRetrieval: null,
  })

  const mapMemoryRetrieval = (raw: any): MemoryRetrievalInfo | null => {
    if (!raw || typeof raw !== 'object') return null
    return {
      queryPreview: typeof raw.query_preview === 'string' ? raw.query_preview : '',
      requestedTopK: Number(raw.requested_top_k ?? 0) || 0,
      hitCount: Number(raw.hit_count ?? 0) || 0,
      usedCanonicalFallback: raw.used_canonical_fallback === true,
      includeReflection: raw.include_reflection === true,
      sourceBreakdown: Array.isArray(raw.source_breakdown) ? raw.source_breakdown : [],
      kindBreakdown: Array.isArray(raw.kind_breakdown) ? raw.kind_breakdown : [],
    }
  }

  const hasExplicitTarget = executionId !== undefined

  const getTargetId = (): string | undefined => {
    if (!hasExplicitTarget) return undefined
    const raw = typeof executionId === 'string' ? executionId : executionId.value
    return typeof raw === 'string' && raw.trim().length > 0 ? raw : undefined
  }

  const matchesTarget = (eventExecId: string): boolean => {
    const targetId = getTargetId()
    // If caller provided an explicit target but it's currently empty (e.g. session switching),
    // reject all events to avoid cross-session message bleed.
    if (hasExplicitTarget && !targetId) return false
    if (suppressedExecutionId.value && eventExecId === suppressedExecutionId.value) {
      return false
    }
    return !targetId || eventExecId === targetId
  }

  const isSuppressedExecution = (eventExecId: string): boolean => {
    return !!suppressedExecutionId.value && suppressedExecutionId.value === eventExecId
  }

  const releaseSuppressedExecution = (eventExecId: string): void => {
    if (!isSuppressedExecution(eventExecId)) return
    console.log('[useAgentEvents] Releasing suppressed execution:', eventExecId)
    suppressedExecutionId.value = null
  }

  const matchesSubagentParent = (parentExecutionId: string): boolean => {
    if (isSuppressedExecution(parentExecutionId)) return false
    if (matchesTarget(parentExecutionId)) return true
    const matcher = options?.subagentParentExecutionMatcher
    if (!matcher) return false
    try {
      return matcher(parentExecutionId) === true
    } catch (e) {
      console.warn('[useAgentEvents] subagent parent matcher failed:', e)
      return false
    }
  }

  const deriveInteractiveShellFingerprint = (
    parsedResult: any,
    toolArgs?: any,
  ): string | undefined => {
    const explicit = typeof parsedResult?.session_fingerprint === 'string'
      ? parsedResult.session_fingerprint.trim()
      : ''
    if (explicit) {
      return explicit
    }

    const executionMode = parsedResult?.execution_mode ?? toolArgs?.execution_mode ?? 'docker'
    const dockerImage = parsedResult?.docker_image ?? toolArgs?.docker_image ?? 'sentinel-sandbox:latest'
    const shell = parsedResult?.shell ?? toolArgs?.shell ?? 'bash'
    if (typeof executionMode !== 'string' || typeof dockerImage !== 'string' || typeof shell !== 'string') {
      return undefined
    }

    return buildTerminalSessionFingerprint(
      executionMode === 'host' ? 'host' : 'docker',
      dockerImage,
      shell,
    )
  }

  const inferToolSuccess = (raw: any): boolean => {
    const isStructuredHttpResponse = (value: Record<string, any>): boolean => {
      return typeof value.status_code === 'number'
        && typeof value.headers === 'object'
        && value.headers !== null
        && (
          typeof value.url === 'string'
          || typeof value.status_text === 'string'
        )
    }

    const visit = (value: any): boolean => {
      if (value == null) return true
      if (typeof value === 'boolean') return value
      if (typeof value === 'number') return value === 0
      if (typeof value === 'string') {
        const trimmed = value.trim()
        if (!trimmed) return true
        const lower = trimmed.toLowerCase()
        if (lower.startsWith('error:') || lower.startsWith('failed:')) return false
        if (trimmed.startsWith('{') || trimmed.startsWith('[')) {
          try {
            return visit(JSON.parse(trimmed))
          } catch {
            return !lower.includes(' no such file or directory')
          }
        }
        return !lower.includes(' no such file or directory')
      }
      if (Array.isArray(value)) return value.every(item => visit(item))
      if (typeof value === 'object') {
        if (typeof value.success === 'boolean') return value.success
        if (typeof value.ok === 'boolean') return value.ok
        if (typeof value.completed === 'boolean' && value.completed === false) return false
        if (typeof value.exit_code === 'number') return value.exit_code === 0
        if (typeof value.code === 'number') return value.code === 0
        if (typeof value.error === 'string' && value.error.trim()) return false
        if (isStructuredHttpResponse(value)) return true
        return Object.values(value).every(item => visit(item))
      }
      return true
    }

    return visit(raw)
  }

  const tryParseJsonObject = (raw: unknown): Record<string, any> | null => {
    if (!raw) return null
    if (typeof raw === 'object' && !Array.isArray(raw)) {
      return raw as Record<string, any>
    }
    if (typeof raw !== 'string') return null
    try {
      const parsed = JSON.parse(raw)
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
        return parsed as Record<string, any>
      }
      return null
    } catch {
      return null
    }
  }

  const normalizeTrackedArtifacts = (raw: unknown): any[] | undefined => {
    return Array.isArray(raw) ? raw : undefined
  }

  const resetExecutionBuffers = (): void => {
    isExecuting.value = false
    streamingContent.value = ''
    thinkingBuffer.value = ''
    currentThinkingMessageId.value = null
    currentAssistantMessageId.value = null
    assistantSegmentBuffer.value = ''
    contentBuffer.value = ''
  }

  const handleExecutionFinished = (payload: AgentExecutionFinishedEvent): void => {
    if (!matchesTarget(payload.execution_id)) return

    if (
      payload.outcome === 'cancelled'
      && isExecuting.value
      && currentExecutionId.value === payload.execution_id
    ) {
      console.log('[useAgentEvents] Ignoring stale cancelled event for active execution:', payload.execution_id)
      return
    }

    resetExecutionBuffers()

    if (payload.outcome === 'failed') {
      const err = payload.error || 'Agent execution failed'
      error.value = err
      messages.value.push({
        id: crypto.randomUUID(),
        type: 'error',
        content: err,
        timestamp: Date.now(),
      })
      return
    }

    error.value = null

    if (payload.outcome === 'succeeded') {
      if (ragMetaInfo.value) {
        const lastAssistant = [...messages.value].reverse().find(m => m.type === 'final')
        if (lastAssistant) {
          lastAssistant.metadata = { ...(lastAssistant.metadata || {}), rag_info: ragMetaInfo.value }
        }
      }
      return
    }

    console.log('[useAgentEvents] Execution cancelled:', payload.execution_id)
  }

  const buildShellFallbackNotice = (resultRaw: unknown): string | null => {
    const parsed = tryParseJsonObject(resultRaw)
    if (!parsed) return null
    const fallbackFrom = String(parsed.fallback_from || '').trim().toLowerCase()
    const executionMode = String(parsed.execution_mode || '').trim().toLowerCase()
    if (fallbackFrom !== 'docker' || executionMode !== 'host') return null
    const reason = String(parsed.fallback_reason || '').trim()
    if (reason) {
      return `系统提示: shell 工具在 Docker 中执行失败，已自动回退到宿主机。原因: ${reason}`
    }
    return '系统提示: shell 工具在 Docker 中执行失败，已自动回退到宿主机。'
  }

  const pushShellFallbackNotice = (
    toolName: string | undefined,
    resultRaw: unknown,
    executionIdForMsg: string,
    toolCallId?: string,
  ) => {
    if ((toolName || '').toLowerCase() !== 'shell') return
    const notice = buildShellFallbackNotice(resultRaw)
    if (!notice) return
    const normalizedToolCallId = String(toolCallId || '').trim()
    if (normalizedToolCallId) {
      const existed = messages.value.some((item) => {
        if (item.metadata?.kind !== 'shell_fallback_notice') return false
        return String(item.metadata?.tool_call_id || '').trim() === normalizedToolCallId
      })
      if (existed) return
    }
    messages.value.push({
      id: crypto.randomUUID(),
      type: 'system',
      content: notice,
      timestamp: Date.now(),
      metadata: {
        kind: 'shell_fallback_notice',
        execution_id: executionIdForMsg,
        tool_call_id: normalizedToolCallId || undefined,
      },
    })
  }

  const hasMessages = computed(() => messages.value.length > 0)
  const lastMessage = computed(() => messages.value[messages.value.length - 1])

  const clearMessages = () => {
    messages.value = []
    streamingContent.value = ''
    contentBuffer.value = ''
    thinkingBuffer.value = ''
    currentThinkingMessageId.value = null
    currentAssistantMessageId.value = null
    assistantSegmentBuffer.value = ''
    error.value = null
    isExecuting.value = false
    currentExecutionId.value = null
    ragMetaInfo.value = null
    subagents.value = []
    contextUsage.value = null
    suppressedExecutionId.value = null
  }

  const resetError = () => {
    error.value = null
  }

  // 停止执行：清空流式内容并更新状态
  const stopExecution = () => {
    console.log('[useAgentEvents] Stopping execution, current execution_id:', currentExecutionId.value)
    const executionToSuppress = currentExecutionId.value || getTargetId() || null
    if (executionToSuppress) {
      suppressedExecutionId.value = executionToSuppress
    }
    isExecuting.value = false
    streamingContent.value = ''
    contentBuffer.value = ''
    thinkingBuffer.value = ''
    currentThinkingMessageId.value = null
    currentAssistantMessageId.value = null
    assistantSegmentBuffer.value = ''

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
      document_attachments?: any[]
      image_attachments?: any[]
      referenced_files?: any[]
      referenced_messages?: any[]
      referenced_assets?: any[]
      referenced_traffic?: any[]
    }>('agent:user_message', (event) => {
      const payload = event.payload
      releaseSuppressedExecution(payload.execution_id)
      if (!matchesTarget(payload.execution_id)) return

      isExecuting.value = true
      currentExecutionId.value = payload.execution_id
      error.value = null
      contentBuffer.value = ''
      streamingContent.value = ''
      thinkingBuffer.value = ''
      currentThinkingMessageId.value = null
      currentAssistantMessageId.value = null
      assistantSegmentBuffer.value = ''

      // 添加用户消息，注入待处理的文档附件和图片附件
      const docAttachments = payload.document_attachments || (
        pendingDocumentAttachments.value.length > 0 
          ? [...pendingDocumentAttachments.value] 
          : undefined
      )
      const imgAttachments = payload.image_attachments
      const referencedFiles = payload.referenced_files
      const referencedMessages = payload.referenced_messages
      const referencedAssets = payload.referenced_assets
      const referencedTraffic = payload.referenced_traffic
      pendingDocumentAttachments.value = [] // Clear after use
      if (shouldSuppressUserMessages()) {
        return
      }
      
      // Build metadata
      const metadata: any = {}
      if (docAttachments) {
        metadata.document_attachments = docAttachments
      }
      if (imgAttachments) {
        metadata.image_attachments = imgAttachments
      }
      if (referencedFiles) {
        metadata.referenced_files = referencedFiles
      }
      if (referencedMessages) {
        metadata.referenced_messages = referencedMessages
      }
      if (referencedAssets) {
        metadata.referenced_assets = referencedAssets
      }
      if (referencedTraffic) {
        metadata.referenced_traffic = referencedTraffic
      }

      const existingById = messages.value.find(item => item.id === payload.message_id)
      if (existingById) {
        existingById.type = 'user'
        existingById.content = payload.content
        existingById.timestamp = payload.timestamp
        existingById.metadata = Object.keys(metadata).length > 0 ? metadata : undefined
        return
      }

      // Reconcile fallback message inserted by agent:start when it arrives before user_message.
      let fallbackIndex = -1
      for (let i = messages.value.length - 1; i >= 0; i -= 1) {
        const item = messages.value[i]
        if (item.type !== 'user') continue
        if (item.content !== payload.content) continue
        const kind = String(item.metadata?.kind || '')
        if (kind === 'user_start_fallback') {
          fallbackIndex = i
          break
        }
      }
      if (fallbackIndex >= 0) {
        const target = messages.value[fallbackIndex]
        target.id = payload.message_id
        target.type = 'user'
        target.content = payload.content
        target.timestamp = payload.timestamp
        target.metadata = Object.keys(metadata).length > 0 ? metadata : undefined
        return
      }
      
      messages.value.push({
        id: payload.message_id,
        type: 'user',
        content: payload.content,
        timestamp: payload.timestamp,
        metadata: Object.keys(metadata).length > 0 ? metadata : undefined,
      })
    })
    unlisteners.push(unlistenUserMessage)

    const unlistenTasks = await listen<AgentTasksUpdatePayload>('agent-tasks-update', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return
    })
    unlisteners.push(unlistenTasks)

    // 监听 agent:start 事件（兼容旧版）
    const unlistenStart = await listen<AgentStartEvent>('agent:start', (event) => {
      const payload = event.payload
      releaseSuppressedExecution(payload.execution_id)
      if (!matchesTarget(payload.execution_id)) return

      isExecuting.value = true
      currentExecutionId.value = payload.execution_id
      error.value = null
      contentBuffer.value = ''
      streamingContent.value = ''
      thinkingBuffer.value = ''
      currentThinkingMessageId.value = null
      currentAssistantMessageId.value = null
      assistantSegmentBuffer.value = ''
      if (shouldSuppressUserMessages()) {
        return
      }

      // 添加用户任务消息（如果没有通过 user_message 事件收到）
      const hasUserMessage = messages.value.some(m =>
        m.type === 'user' && m.content === payload.task
      )
      if (!hasUserMessage) {
        // 注入待处理的文档附件
        const docAttachments = pendingDocumentAttachments.value.length > 0 
          ? [...pendingDocumentAttachments.value] 
          : undefined
        pendingDocumentAttachments.value = [] // Clear after use
        
        messages.value.push({
          id: crypto.randomUUID(),
          type: 'user',
          content: payload.task,
          timestamp: Date.now(),
          metadata: docAttachments
            ? { document_attachments: docAttachments, kind: 'user_start_fallback' }
            : { kind: 'user_start_fallback' },
        })
      }
    })
    unlisteners.push(unlistenStart)

    // Listen for context usage events
    const unlistenContextUsage = await listen<{
      execution_id: string
      used_tokens: number
      max_tokens: number
      usage_percentage: number
      system_prompt_tokens: number
      history_tokens: number
      history_count: number
      summary_tokens?: number
      summary_global_tokens?: number
      summary_segment_tokens?: number
      summary_segment_count?: number
      sentinel_mode?: boolean
      sentinel_active_intent?: {
        intent_id?: string
        relation?: string
        transition?: string
        confidence?: number
        parent_intent_id?: string | null
      } | null
      sentinel_clarification?: {
        needed?: boolean
        status?: string
        source?: string
        compression_aggressiveness?: string
      } | null
    }>('agent:context_usage', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      contextUsage.value = {
        usedTokens: payload.used_tokens,
        maxTokens: payload.max_tokens,
        usagePercentage: payload.usage_percentage,
        systemPromptTokens: payload.system_prompt_tokens,
        historyTokens: payload.history_tokens,
        historyCount: payload.history_count,
        summaryTokens: payload.summary_tokens ?? 0,
        summaryGlobalTokens: payload.summary_global_tokens ?? 0,
        summarySegmentTokens: payload.summary_segment_tokens ?? 0,
        summarySegmentCount: payload.summary_segment_count ?? 0,
        sentinelMode: payload.sentinel_mode === true,
        sentinelIntentId: payload.sentinel_active_intent?.intent_id || null,
        sentinelIntentConfidence: payload.sentinel_active_intent?.confidence ?? null,
        sentinelIntentTransition: payload.sentinel_active_intent?.transition || null,
        sentinelParentIntentId: payload.sentinel_active_intent?.parent_intent_id || null,
        sentinelClarificationNeeded: payload.sentinel_clarification?.needed === true,
        sentinelClarificationStatus: payload.sentinel_clarification?.status || null,
        sentinelCompressionAggressiveness:
          payload.sentinel_clarification?.compression_aggressiveness || null,
      }
      console.log('[useAgentEvents] Context usage updated:', contextUsage.value)
    })
    unlisteners.push(unlistenContextUsage)

    const unlistenContextSnapshot = await listen<{
      execution_id: string
      memory_retrieval?: any
      sentinel_mode?: boolean
      sentinel_intent_id?: string | null
      sentinel_intent_confidence?: number | null
      sentinel_intent_transition?: string | null
      sentinel_parent_intent_id?: string | null
      sentinel_clarification_needed?: boolean
      sentinel_clarification_status?: string | null
      sentinel_compression_aggressiveness?: string | null
    }>('agent:context_snapshot', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      const next = contextUsage.value ? { ...contextUsage.value } : buildContextUsageSkeleton()
      next.memoryRetrieval = mapMemoryRetrieval(payload.memory_retrieval)
      next.sentinelMode = payload.sentinel_mode === true
      next.sentinelIntentId = payload.sentinel_intent_id || next.sentinelIntentId || null
      next.sentinelIntentConfidence =
        payload.sentinel_intent_confidence ?? next.sentinelIntentConfidence ?? null
      next.sentinelIntentTransition =
        payload.sentinel_intent_transition || next.sentinelIntentTransition || null
      next.sentinelParentIntentId =
        payload.sentinel_parent_intent_id || next.sentinelParentIntentId || null
      next.sentinelClarificationNeeded =
        payload.sentinel_clarification_needed === true
          ? true
          : (next.sentinelClarificationNeeded ?? false)
      next.sentinelClarificationStatus =
        payload.sentinel_clarification_status || next.sentinelClarificationStatus || null
      next.sentinelCompressionAggressiveness =
        payload.sentinel_compression_aggressiveness
        || next.sentinelCompressionAggressiveness
        || null
      contextUsage.value = next
    })
    unlisteners.push(unlistenContextSnapshot)

    const unlistenSubagentStart = await listen<SubagentStartEvent>('subagent:start', (event) => {
      const payload = event.payload
      if (!matchesSubagentParent(payload.parent_execution_id)) return

      const existing = subagents.value.find(s => s.id === payload.execution_id)
      if (!existing) {
        subagents.value.unshift({
          id: payload.execution_id,
          parentId: payload.parent_execution_id,
          role: payload.role,
          status: 'running',
          progress: 0,
          task: payload.task,
          startedAt: Date.now(),
        })
      } else {
        existing.status = 'running'
        existing.progress = existing.progress ?? 0
        existing.task = payload.task
        existing.startedAt = existing.startedAt ?? Date.now()
      }
    })
    unlisteners.push(unlistenSubagentStart)

    // 监听 agent:iteration 事件
    const unlistenIteration = await listen<AgentIterationEvent>('agent:iteration', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      // Auto-recover execution state after page refresh
      if (!isExecuting.value) {
        console.log('[useAgentEvents] Auto-recovering execution state from iteration event')
        isExecuting.value = true
        currentExecutionId.value = payload.execution_id
      }
      error.value = null

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

      // Auto-recover execution state after page refresh
      if (!isExecuting.value) {
        console.log('[useAgentEvents] Auto-recovering execution state from chunk event')
        isExecuting.value = true
        currentExecutionId.value = payload.execution_id
      }
      error.value = null

      if (payload.chunk_type === 'text') {
        // Text content: reset thinking state and accumulate text
        if (currentThinkingMessageId.value) {
          currentThinkingMessageId.value = null
          thinkingBuffer.value = ''
        }
        contentBuffer.value += payload.content
        streamingContent.value = contentBuffer.value
        assistantSegmentBuffer.value += payload.content

        // Ensure there's a visible assistant message in the message list so ordering matches arrival order
        if (!currentAssistantMessageId.value) {
          const msgId = crypto.randomUUID()
          currentAssistantMessageId.value = msgId
          messages.value.push({
            id: msgId,
            type: 'final',
            content: assistantSegmentBuffer.value,
            timestamp: Date.now(),
          })
        } else {
          const existingMsg = messages.value.find(m => m.id === currentAssistantMessageId.value)
          if (existingMsg) {
            existingMsg.content = assistantSegmentBuffer.value
          }
        }
      } else if (payload.chunk_type === 'usage') {
        // Usage report from LLM API - update context usage with real token counts
        if (payload.input_tokens !== undefined && payload.output_tokens !== undefined) {
          const inputTokens = payload.input_tokens
          const outputTokens = payload.output_tokens
          
          // Update context usage with real values from LLM
          if (contextUsage.value) {
            // Use the real input_tokens from LLM as used_tokens (more accurate than our estimate)
            const maxTokens = contextUsage.value.maxTokens
            const usedTokens = inputTokens + outputTokens
            const usagePercentage = maxTokens > 0
              ? Math.min(100, (usedTokens / maxTokens * 100))
              : 0
            contextUsage.value = {
              ...contextUsage.value,
              usedTokens,
              usagePercentage,
            }
          } else {
            // If no context usage yet, create a basic one from configured context window.
            const maxTokens = resolveDefaultMaxContextTokens()
            const usedTokens = inputTokens + outputTokens
            const usagePercentage = Math.min(100, (usedTokens / maxTokens * 100))
            contextUsage.value = {
              usedTokens,
              maxTokens,
              usagePercentage,
              systemPromptTokens: 0,
              historyTokens: inputTokens,
              historyCount: 0,
              summaryTokens: 0,
              summaryGlobalTokens: 0,
              summarySegmentTokens: 0,
              summarySegmentCount: 0,
            }
          }
        }
      } else if (payload.chunk_type === 'reasoning') {
        // Reasoning content: accumulate in existing thinking message or create new one
        thinkingBuffer.value += payload.content

        if (currentThinkingMessageId.value) {
          // Update existing thinking message
          const existingMsg = messages.value.find(m => m.id === currentThinkingMessageId.value)
          if (existingMsg) {
            existingMsg.content = thinkingBuffer.value
          }
        } else {
          // Create new thinking message
          const msgId = crypto.randomUUID()
          currentThinkingMessageId.value = msgId
          messages.value.push({
            id: msgId,
            type: 'thinking',
            content: thinkingBuffer.value,
            timestamp: Date.now(),
          })
        }
      }
    })
    unlisteners.push(unlistenChunk)

    // 监听 agent:tool_call 事件
    const unlistenToolCall = await listen<AgentToolCallEvent>('agent:tool_call', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      // Auto-recover execution state after page refresh
      if (!isExecuting.value) {
        console.log('[useAgentEvents] Auto-recovering execution state from tool_call event')
        isExecuting.value = true
        currentExecutionId.value = payload.execution_id
      }

      // Close current assistant segment so later assistant text won't appear above this tool call
      currentAssistantMessageId.value = null
      assistantSegmentBuffer.value = ''

      messages.value.push({
        id: crypto.randomUUID(),
        type: 'tool_call',
        content: `Calling tool: ${payload.tool_name}`,
        timestamp: Date.now(),
        metadata: {
          tool_name: payload.tool_name,
          tool_args: payload.tool_input,
          status: 'running',
          execution_id: payload.execution_id,
        }
      })

      // ❌ 不要在这里打开终端，等待 tool_result 事件中的 session_id
      // 检测 interactive_shell 工具调用
      if (payload.tool_name === 'interactive_shell') {
        console.log('[Agent] Detected interactive_shell call, will open terminal when result arrives')
      }
    })
    unlisteners.push(unlistenToolCall)

    // 监听 agent:tool_call_complete 事件（新格式 - rig-core）
    const unlistenToolCallComplete = await listen<AgentToolCallCompleteEvent>('agent:tool_call_complete', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      // Auto-recover execution state after page refresh
      if (!isExecuting.value) {
        console.log('[useAgentEvents] Auto-recovering execution state from tool_call_complete event')
        isExecuting.value = true
        currentExecutionId.value = payload.execution_id
      }

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

      // Close current assistant segment so later assistant text won't appear above this tool call
      currentAssistantMessageId.value = null
      assistantSegmentBuffer.value = ''

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

      // ❌ 不要在这里打开终端，等待 tool_result 事件中的 session_id
      // 检测 interactive_shell 工具调用
      if (payload.tool_name === 'interactive_shell') {
        console.log('[Agent] Detected interactive_shell call (complete), will open terminal when result arrives')
      }
    })
    unlisteners.push(unlistenToolCallComplete)

    // 监听 agent:tool_result 事件（旧格式兼容）
    const unlistenToolResult = await listen<AgentToolResultEvent>('agent:tool_result', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      // Auto-recover execution state after page refresh
      if (!isExecuting.value) {
        console.log('[useAgentEvents] Auto-recovering execution state from tool_result event')
        isExecuting.value = true
        currentExecutionId.value = payload.execution_id
      }

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
            const success = typeof newPayload.success === 'boolean'
              ? newPayload.success
              : inferToolSuccess(newPayload.result)
            existingMsg.metadata.status = success ? 'completed' : 'failed'
            existingMsg.metadata.tool_result = resultContent
            existingMsg.metadata.success = success
            existingMsg.metadata.tracked_artifacts = normalizeTrackedArtifacts(
              newPayload.tracked_artifacts,
            )
            existingMsg.content = `工具调用完成: ${callInfo.tool_name}`
            pushShellFallbackNotice(
              callInfo.tool_name,
              newPayload.result,
              payload.execution_id,
              newPayload.tool_call_id,
            )
            
            // 如果是 interactive_shell 工具，自动打开终端面板，关闭任务面板
            if (callInfo.tool_name === 'interactive_shell') {
              // First close task panel
              const tasks = useAgentTasks()
              tasks.close()

              const terminal = useTerminal()

              // 深度解析函数：自动挖掘嵌套的 JSON 字符串或数组
              const deepParse = (input: any, depth = 0): any => {
                // 如果输入是字符串，尝试解析
                if (typeof input === 'string') {
                  try {
                    const parsed = JSON.parse(input)
                    return deepParse(parsed, depth)
                  } catch (e) {
                    return input
                  }
                }

                // 如果输入是数组，取第一个元素继续解析
                if (Array.isArray(input) && input.length > 0) {
                  return deepParse(input[0], depth + 1)
                }

                // 如果输入是对象，尝试解析内部的 text 字段
                if (typeof input === 'object' && input !== null) {
                  if (input.text && typeof input.text === 'string') {
                    try {
                      const inner = deepParse(input.text, depth + 1)
                      if (typeof inner === 'object' && inner !== null && !Array.isArray(inner)) {
                        return { ...inner, type: input.type }
                      }
                      return inner
                    } catch (e) {
                      // 解析失败，返回原对象
                    }
                  }
                  return input
                }

                return input
              }

              try {
                const parsed = deepParse(resultContent)
                if (parsed.session_id) {
                  terminal.syncActiveSession(
                    parsed.session_id,
                    deriveInteractiveShellFingerprint(parsed, callInfo.arguments),
                  )
                  terminal.openTerminal()
                } else {
                  terminal.openTerminal()
                }
              } catch (e) {
                terminal.openTerminal()
              }
            }
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
          const success = inferToolSuccess(payload.tool_result)
          matchingToolCall.metadata.status = success ? 'completed' : 'failed'
          matchingToolCall.metadata.tool_result = payload.tool_result
          matchingToolCall.metadata.success = success
          matchingToolCall.metadata.tracked_artifacts = normalizeTrackedArtifacts(
            (payload as any).tracked_artifacts,
          )
          matchingToolCall.content = `工具调用完成: ${payload.tool_name}`
          pushShellFallbackNotice(
            payload.tool_name,
            payload.tool_result,
            payload.execution_id,
            String(matchingToolCall.metadata?.tool_call_id || ''),
          )
          
          // 旧格式路径：如果是 interactive_shell 工具，也自动打开终端面板，关闭任务面板
          if (payload.tool_name === 'interactive_shell') {
            // First close task panel
            const tasks = useAgentTasks()
            tasks.close()

            const terminal = useTerminal()

            const deepParse = (input: any): any => {
              if (typeof input !== 'string') {
                if (Array.isArray(input) && input.length > 0) return deepParse(input[0])
                return input
              }
              try {
                const parsed = JSON.parse(input)
                if (typeof parsed === 'object' && parsed !== null) {
                  if (parsed.text && typeof parsed.text === 'string') {
                    const inner = deepParse(parsed.text)
                    return { ...inner, ...parsed, text: parsed.text }
                  }
                  if (Array.isArray(parsed) && parsed.length > 0) return deepParse(parsed[0])
                  return parsed
                }
                return parsed
              } catch (e) {
                return input
              }
            }

            try {
              const parsed = deepParse(payload.tool_result)
              if (parsed.session_id) {
                terminal.syncActiveSession(
                  parsed.session_id,
                  deriveInteractiveShellFingerprint(parsed, matchingToolCall.metadata?.tool_args),
                )
                terminal.openTerminal()
              } else {
                terminal.openTerminal()
              }
            } catch (e) {
              terminal.openTerminal()
            }
          }
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
              tracked_artifacts: normalizeTrackedArtifacts((payload as any).tracked_artifacts),
            }
          })
        }
      }

      applyFileVerificationStatuses(messages.value)
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

    const unlistenToolsActivated = await listen<AgentToolsActivatedEvent>('agent:tools_activated', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      messages.value.push({
        id: crypto.randomUUID(),
        type: 'system',
        content: buildToolsActivatedMessage(payload),
        timestamp: Date.now(),
        metadata: {
          kind: 'tools_activated',
          tool_ids: payload.tool_ids,
          tools: payload.tools,
          tools_preview: buildToolsPreview(payload.tools),
          query: payload.query || undefined,
          runtime_hint: payload.runtime_hint || undefined,
        }
      })
    })
    unlisteners.push(unlistenToolsActivated)

    // 监听 agent:skill_loaded 事件（显示技能加载提示）
    const unlistenSkillLoaded = await listen<{
      execution_id: string
      skill_id: string
      skill_name: string
    }>('agent:skill_loaded', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      messages.value.push({
        id: crypto.randomUUID(),
        type: 'system',
        content: `Skill loaded: ${payload.skill_name} (${payload.skill_id})`,
        timestamp: Date.now(),
        metadata: {
          kind: 'skill_loaded',
          skill_id: payload.skill_id,
          skill_name: payload.skill_name,
        }
      })
    })
    unlisteners.push(unlistenSkillLoaded)

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
      reasoning_content?: string | null
      timestamp: number
    }>('agent:assistant_message_saved', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      console.log('[useAgentEvents] Assistant message saved:', payload.message_id)
      const reasoningContent = typeof payload.reasoning_content === 'string'
        ? payload.reasoning_content.trim()
        : ''

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

      // IMPORTANT:
      // We render assistant text as multiple segments (split by tool-call boundaries) to preserve arrival order.
      // So we DO NOT overwrite earlier segments with the full final content here (that would "jump" above tool calls).
      // We only attach metadata to the latest assistant segment if present.
      if (messages.value.length > 0) {
        const lastAssistantIndex = (() => {
          for (let i = messages.value.length - 1; i >= 0; i -= 1) {
            if (messages.value[i]?.type === 'final') return i
          }
          return -1
        })()
        const lastAssistant = lastAssistantIndex >= 0 ? messages.value[lastAssistantIndex] : null

        if (lastAssistant && reasoningContent) {
          const previousMessage = lastAssistantIndex > 0 ? messages.value[lastAssistantIndex - 1] : null
          if (previousMessage?.type === 'thinking') {
            previousMessage.content = reasoningContent
          } else {
            messages.value.splice(lastAssistantIndex, 0, {
              id: `thinking:${payload.message_id}`,
              type: 'thinking',
              content: reasoningContent,
              timestamp: payload.timestamp,
            })
          }
        }

        if (lastAssistant) {
          lastAssistant.metadata = ragMetaInfo.value ? { rag_info: ragMetaInfo.value } : lastAssistant.metadata
        }
      }
      currentAssistantMessageId.value = null
      assistantSegmentBuffer.value = ''

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
      citations?: any[]
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
          source_count: count,
          citations: payload.citations
        }
      }
    })
    unlisteners.push(unlistenMetaInfo)

    // 监听 agent:rag_retrieval_complete 事件
    const unlistenRagComplete = await listen<{
      execution_id: string
      citations: any[]
    }>('agent:rag_retrieval_complete', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      console.log('[useAgentEvents] RAG retrieval complete:', payload)

      if (payload.citations) {
        if (!ragMetaInfo.value) {
          ragMetaInfo.value = {
            rag_applied: true,
            rag_sources_used: payload.citations.length > 0,
            source_count: payload.citations.length,
            citations: payload.citations
          }
        } else {
          ragMetaInfo.value.citations = payload.citations
          ragMetaInfo.value.rag_sources_used = payload.citations.length > 0
          ragMetaInfo.value.source_count = payload.citations.length
        }
      }
    })
    unlisteners.push(unlistenRagComplete)

    const unlistenExecutionFinished = await listen<AgentExecutionFinishedEvent>('agent:execution_finished', (event) => {
      handleExecutionFinished(event.payload)
    })
    unlisteners.push(unlistenExecutionFinished)

    const unlistenSubagentDone = await listen<SubagentDoneEvent>('subagent:done', (event) => {
      const payload = event.payload
      if (!matchesSubagentParent(payload.parent_execution_id)) return

      const now = Date.now()
      const existing = subagents.value.find(s => s.id === payload.execution_id)
      if (existing) {
        existing.status = payload.success ? 'completed' : 'failed'
        existing.progress = 100
        existing.summary = payload.output?.slice(0, 200) || existing.summary
        existing.duration = existing.startedAt ? now - existing.startedAt : undefined
      } else {
        subagents.value.unshift({
          id: payload.execution_id,
          parentId: payload.parent_execution_id,
          status: payload.success ? 'completed' : 'failed',
          progress: 100,
          summary: payload.output?.slice(0, 200),
        })
      }
    })
    unlisteners.push(unlistenSubagentDone)

    const unlistenSubagentError = await listen<SubagentErrorEvent>('subagent:error', (event) => {
      const payload = event.payload
      if (!matchesSubagentParent(payload.parent_execution_id)) return

      const now = Date.now()
      const existing = subagents.value.find(s => s.id === payload.execution_id)
      if (existing) {
        existing.status = 'failed'
        existing.progress = 100
        existing.error = payload.error
        existing.duration = existing.startedAt ? now - existing.startedAt : undefined
      } else {
        subagents.value.unshift({
          id: payload.execution_id,
          parentId: payload.parent_execution_id,
          status: 'failed',
          progress: 100,
          error: payload.error,
        })
      }
    })
    unlisteners.push(unlistenSubagentError)

    // 监听 agent:segment_summary_created 事件（滑动窗口段落摘要）
    const unlistenSegmentSummary = await listen<AgentSegmentSummaryCreatedEvent>('agent:segment_summary_created', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.conversation_id)) return

      console.log('[useAgentEvents] Segment summary created:', payload)

      messages.value.push({
        id: crypto.randomUUID(),
        type: 'system',
        content: `Memory segment #${payload.segment_index} compressed (${payload.tokens} tokens)`,
        timestamp: Date.now(),
        metadata: {
          kind: 'segment_summary',
          segment_index: payload.segment_index,
          summary_tokens: payload.tokens,
          summary_content: payload.summary,
        }
      })
    })
    unlisteners.push(unlistenSegmentSummary)

    // 监听 agent:global_summary_updated 事件（滑动窗口全局摘要）
    const unlistenGlobalSummary = await listen<AgentGlobalSummaryUpdatedEvent>('agent:global_summary_updated', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.conversation_id)) return

      console.log('[useAgentEvents] Global summary updated:', payload)

      messages.value.push({
        id: crypto.randomUUID(),
        type: 'system',
        content: `Long-term memory updated (${payload.tokens} tokens)`,
        timestamp: Date.now(),
        metadata: {
          kind: 'global_summary',
          summary_tokens: payload.tokens,
          summary_content: payload.summary,
        }
      })
    })
    unlisteners.push(unlistenGlobalSummary)

    // 监听 agent:retry 事件
    const unlistenRetry = await listen<AgentRetryEvent>('agent:retry', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      console.warn('[useAgentEvents] Agent retry event received:', payload)
      error.value = null
      
      // 保留已完成的工具调用和助手消息，只清理流式状态
      // 不再清空 messages 数组，让用户看到已完成的进度
      assistantSegmentBuffer.value = ''
      streamingContent.value = ''
      contentBuffer.value = ''
      thinkingBuffer.value = ''
      currentThinkingMessageId.value = null
      currentAssistantMessageId.value = null

      // 添加一条重试系统消息（显示累积的进度）
      const accProgress = (payload as any).accumulated_progress
      const progressInfo = accProgress 
        ? ` (Progress saved: ${accProgress.tool_calls} tools, ${accProgress.output_chars} chars)`
        : ''
      
      messages.value.push({
        id: crypto.randomUUID(),
        type: 'system',
        content: `Something went wrong, retrying... (Attempt ${payload.retry_count}/${payload.max_retries})${progressInfo}`,
        timestamp: Date.now(),
        metadata: {
          kind: 'retry_notification',
          retry_count: payload.retry_count,
          error: payload.error,
        }
      })
    })
    unlisteners.push(unlistenRetry)

    // 监听 agent:completion_guard_failed 事件
    const unlistenCompletionGuardFailed = await listen<AgentCompletionGuardFailedEvent>('agent:completion_guard_failed', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      const reasons = Array.isArray(payload.reasons)
        ? payload.reasons.filter(reason => typeof reason === 'string' && reason.trim().length > 0)
        : []
      const reasonContent = reasons.length > 0
        ? `\nReasons:\n${reasons.map((reason, index) => `${index + 1}. ${reason}`).join('\n')}`
        : ''
      const artifactContent = payload.required_artifact
        ? `\nRequired artifact: ${payload.required_artifact}`
        : ''
      const statsContent = (typeof payload.response_length === 'number' || typeof payload.tool_calls === 'number')
        ? `\nStats: response_length=${payload.response_length ?? 'n/a'}, tool_calls=${payload.tool_calls ?? 'n/a'}`
        : ''

      messages.value.push({
        id: crypto.randomUUID(),
        type: 'system',
        content: `Completion guard blocked a false completion.${artifactContent}${reasonContent}${statsContent}`,
        timestamp: Date.now(),
        metadata: {
          kind: 'completion_guard_failed',
          execution_id: payload.execution_id,
        }
      })
    })
    unlisteners.push(unlistenCompletionGuardFailed)

    // 监听 agent:tenth_man_critique 事件
    const unlistenTenthMan = await listen<AgentTenthManCritiqueEvent>('agent:tenth_man_critique', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      console.log('[useAgentEvents] Tenth Man critique received:', payload.message_id)

      messages.value.push({
        id: payload.message_id,
        type: 'system',
        content: payload.critique,
        timestamp: Date.now(),
        metadata: {
          kind: 'tenth_man_critique',
          execution_id: payload.execution_id
        }
      })
    })
    unlisteners.push(unlistenTenthMan)

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
        // Text content: reset thinking state and accumulate text
        if (currentThinkingMessageId.value) {
          currentThinkingMessageId.value = null
          thinkingBuffer.value = ''
        }
        contentBuffer.value += chunk.content
        streamingContent.value = contentBuffer.value
        assistantSegmentBuffer.value += chunk.content

        // Ensure there's a visible assistant message in the message list
        if (!currentAssistantMessageId.value) {
          const msgId = crypto.randomUUID()
          currentAssistantMessageId.value = msgId
          messages.value.push({
            id: msgId,
            type: 'final',
            content: assistantSegmentBuffer.value,
            timestamp: Date.now(),
          })
        } else {
          const existingMsg = messages.value.find(m => m.id === currentAssistantMessageId.value)
          if (existingMsg) {
            existingMsg.content = assistantSegmentBuffer.value
          }
        }
        return
      }

      if (chunkType === 'Thinking') {
        if (chunk.content.trim()) {
          // Accumulate thinking content
          thinkingBuffer.value += chunk.content

          if (currentThinkingMessageId.value) {
            // Update existing thinking message
            const existingMsg = messages.value.find(m => m.id === currentThinkingMessageId.value)
            if (existingMsg) {
              existingMsg.content = thinkingBuffer.value
            }
          } else {
            // Create new thinking message
            const msgId = crypto.randomUUID()
            currentThinkingMessageId.value = msgId
            messages.value.push({
              id: msgId,
              type: 'thinking',
              content: thinkingBuffer.value,
              timestamp: Date.now(),
            })
          }
        }
        return
      }
    })
    unlisteners.push(unlistenOldChunk)

    // 监听 agent:tenth_man_warning 事件（工具调用前的警告）
    const unlistenTenthManWarning = await listen<{
      execution_id: string
      trigger: string
      tool_name: string
      critique: string
      requires_confirmation: boolean
    }>('agent:tenth_man_warning', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      console.log('[useAgentEvents] Tenth Man warning:', payload)

      // 显示警告通知
      // TODO: 如果 requires_confirmation 为 true，显示确认对话框
      // 目前先以消息形式展示
      const msgId = crypto.randomUUID()
      messages.value.push({
        id: msgId,
        type: 'system',
        content: `⚠️ **第十人警告** (${payload.tool_name})\n\n${payload.critique}`,
        timestamp: Date.now(),
        metadata: {
          kind: 'tenth_man_warning',
          trigger: payload.trigger,
          tool_name: payload.tool_name,
          requires_confirmation: payload.requires_confirmation,
        }
      })
    })
    unlisteners.push(unlistenTenthManWarning)

    // 监听 agent:tenth_man_intervention 事件（结论检测时的干预）
    const unlistenTenthManIntervention = await listen<{
      execution_id: string
      trigger: string
      critique: string
      timestamp: number
    }>('agent:tenth_man_intervention', (event) => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id)) return

      console.log('[useAgentEvents] Tenth Man intervention:', payload)

      // 添加第十人干预消息（使用特殊类型以便前端渲染）
      const msgId = crypto.randomUUID()
      messages.value.push({
        id: msgId,
        type: 'system',
        content: payload.critique,
        timestamp: payload.timestamp || Date.now(),
        metadata: {
          kind: 'tenth_man_intervention',
          trigger: payload.trigger,
        }
      })
    })
    unlisteners.push(unlistenTenthManIntervention)

    const unlistenShellBackgroundTask = await listen<{
      id: string
      execution_id?: string | null
      session_id: string
      command: string
      status: 'running' | 'completed' | 'failed' | 'cancelled'
      exit_code?: number | null
      output_preview?: string
    }>('shell-background-task-update', (event) => {
      const payload = event.payload
      if (!payload?.execution_id || !matchesTarget(payload.execution_id)) return
      if (payload.status === 'running') return

      const msgId = crypto.randomUUID()
      const statusLabel = payload.status === 'completed'
        ? '后台 Shell 任务完成'
        : payload.status === 'failed'
          ? '后台 Shell 任务失败'
          : '后台 Shell 任务已停止'
      const exitSuffix = typeof payload.exit_code === 'number'
        ? `\n\nExit code: ${payload.exit_code}`
        : ''
      const preview = String(payload.output_preview || '').trim()
      const previewBlock = preview ? `\n\n${preview}` : ''

      messages.value.push({
        id: msgId,
        type: 'system',
        content: `**${statusLabel}**\n\n\`${payload.command}\`${exitSuffix}${previewBlock}`,
        timestamp: Date.now(),
        metadata: {
          kind: 'shell_background_task',
          task_id: payload.id,
          session_id: payload.session_id,
          command: payload.command,
          status: payload.status,
          exit_code: payload.exit_code ?? undefined,
        },
      })
    })
    unlisteners.push(unlistenShellBackgroundTask)
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
    subagents,
    hasMessages,
    lastMessage,
    ragMetaInfo,
    contextUsage,
    clearMessages,
    resetError,
    stopExecution,
    startListening,
    stopListening,
    // Set pending document attachments to be injected into next user message
    setPendingDocumentAttachments: (docs: any[]) => {
      pendingDocumentAttachments.value = docs
    },
  }
}

/**
 * 全局 Agent 事件（不过滤 executionId）
 */
export function useGlobalAgentEvents(): UseAgentEventsReturn {
  return useAgentEvents()
}
