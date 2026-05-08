import { ref, onMounted, onUnmounted, computed, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AgentMessage, MessageType } from '@/types/agent'
import { useAgentTasks } from '@/composables/useAgentTasks'
import { useTerminal } from '@/composables/useTerminal'
import { buildToolsActivatedMessage, buildToolsPreview } from '@/utils/agentToolActivation'
import { applyFileVerificationStatuses } from '@/components/Agent/fileVerificationSupport'
import {
  appendParallelChunk,
  applyParallelSessionStats,
  appendParallelSystemEvent,
  appendParallelToolCall,
  appendParallelToolResult,
} from '@/composables/agentParallelEventSupport'
import { useAgentParallelRunState } from '@/composables/useAgentParallelRunState'
import { schedulePersistParallelModelEvents } from '@/composables/agentParallelEventPersistence'
import type { AgentTasksUpdatePayload } from '@/types/taskRuntime'
import type {
  AgentChunkEvent,
  AgentCompletionGuardFailedEvent,
  AgentContextCompactionRequestedEvent,
  AgentContextCompressionFinishedEvent,
  AgentContextCompressionStartedEvent,
  AgentContextPressureEvent,
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
  ContextCompressionInfo,
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
import {
  buildContextUsageSkeleton as buildContextUsageSkeletonFromMax,
  createAgentEventTargetMatcher,
  deriveInteractiveShellFingerprint,
  hasMeaningfulShellResult,
  handleAgentExecutionFinished,
  inferToolSuccess,
  insertMessageBeforeFollowingFinal,
  isShellContinuation,
  mapMemoryRetrieval,
  normalizeTrackedArtifacts,
  readGeneration,
  resolveDefaultMaxContextTokens as resolveDefaultMaxContextTokensFromOption,
  pushShellFallbackNotice,
  tryParseJsonObject,
} from '@/composables/agentEventSupport'

/**
 * Agent 事件监听
 * @param executionId 可选的执行 ID 过滤
 */
export function useAgentEvents(
  executionId?: Ref<string> | string,
  options?: UseAgentEventsOptions
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
  const contextCompression = ref<ContextCompressionInfo | null>(null)
  const suppressedExecutionId = ref<string | null>(null)
  const settledExecutionId = ref<string | null>(null)
  const currentGeneration = ref<number | null>(null)
  const executionStartedAt = ref<number | null>(null)
  const latestUsage = ref<{ inputTokens: number; outputTokens: number } | null>(null)

  const thinkingBuffer = ref('')
  const currentThinkingMessageId = ref<string | null>(null)

  const currentAssistantMessageId = ref<string | null>(null)
  const assistantSegmentBuffer = ref('')

  const toolCallTracker = new Map<
    string,
    {
      tool_name: string
      arguments: any
      message_id: string
      message_index: number
      silent?: boolean
    }
  >()
  const parallelAgentChunkExecutions = new Set<string>()
  const {
    clearParallelRuns,
    getParallelChild,
    parallelRuns,
    parallelTaskSources,
    rememberParallelRun,
    settleParallelRunIfDone,
    upsertParallelRunMessage,
  } = useAgentParallelRunState({
    isExecuting,
    markExecutionSettled: id => markExecutionSettled(id),
    messages,
    streamingContent,
  })
  const flushParallelChild = (child: ReturnType<typeof getParallelChild>) => {
    if (!child) return
    upsertParallelRunMessage(child.run)
    schedulePersistParallelModelEvents(child.run, child.item)
  }

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
    return resolveDefaultMaxContextTokensFromOption(options?.defaultMaxContextTokens)
  }

  const buildContextUsageSkeleton = (): ContextUsageInfo =>
    buildContextUsageSkeletonFromMax(resolveDefaultMaxContextTokens())

  const { getTargetId, isSuppressedExecution, matchesTarget, releaseSuppressedExecution } =
    createAgentEventTargetMatcher({
      currentExecutionId,
      currentGeneration,
      executionId,
      isExecuting,
      suppressedExecutionId,
    })

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

  const resetExecutionBuffers = (): void => {
    isExecuting.value = false
    streamingContent.value = ''
    thinkingBuffer.value = ''
    currentThinkingMessageId.value = null
    currentAssistantMessageId.value = null
    assistantSegmentBuffer.value = ''
    contentBuffer.value = ''
    contextCompression.value = null
  }

  const markExecutionActive = (executionId: string): void => {
    if (currentExecutionId.value && currentExecutionId.value !== executionId) {
      contextCompression.value = null
    }
    settledExecutionId.value = null
    isExecuting.value = true
    currentExecutionId.value = executionId
  }

  const markExecutionSettled = (executionId: string): void => {
    settledExecutionId.value = executionId
  }

  const isSettledActivityEvent = (executionId: string): boolean =>
    settledExecutionId.value === executionId

  const startExecutionTiming = (): void => {
    executionStartedAt.value = Date.now()
    latestUsage.value = null
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
    contextCompression.value = null
    suppressedExecutionId.value = null
    settledExecutionId.value = null
    currentGeneration.value = null
    executionStartedAt.value = null
    latestUsage.value = null
    clearParallelRuns()
  }

  const resetError = () => {
    error.value = null
  }

  // 停止执行：清空流式内容并更新状态
  const stopExecution = () => {
    console.log(
      '[useAgentEvents] Stopping execution, current execution_id:',
      currentExecutionId.value
    )
    const executionToSuppress = currentExecutionId.value || getTargetId() || null
    const targetId = getTargetId()
    const activeParallelRun = targetId
      ? Array.from(parallelRuns.values()).find(
          run =>
            run.parentConversationId === targetId &&
            run.items.some(item => item.status === 'pending' || item.status === 'running')
        )
      : null
    if (activeParallelRun) {
      void invoke('cancel_ai_parallel_run', {
        request: {
          parallel_run_id: activeParallelRun.id,
          parent_execution_id: activeParallelRun.parentConversationId,
        },
      }).catch(cancelError => {
        console.warn('[useAgentEvents] Failed to cancel parallel run:', cancelError)
      })
    }
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
    executionStartedAt.value = null
    latestUsage.value = null
    contextCompression.value = null

    // 如果有正在流式输出的内容，将其作为最终消息添加
    // 注意：后端取消后可能不会发送 complete 事件，所以这里处理残留内容
  }

  const startListening = async () => {
    const unlistenParallelRunStarted = await listen<any>('agent:parallel_run_started', event => {
      const run = rememberParallelRun(event.payload)
      if (!matchesTarget(run.parentConversationId)) return
      markExecutionActive(run.parentConversationId)
      startExecutionTiming()
      upsertParallelRunMessage(run)
    })
    unlisteners.push(unlistenParallelRunStarted)

    const unlistenParallelRunUpdated = await listen<any>('agent:parallel_run_updated', event => {
      const run = rememberParallelRun(event.payload)
      if (!matchesTarget(run.parentConversationId)) return
      upsertParallelRunMessage(run)
      settleParallelRunIfDone(run)
    })
    unlisteners.push(unlistenParallelRunUpdated)

    // 监听用户消息事件（从后端保存后推送）
    const unlistenUserMessage = await listen<{
      execution_id: string
      conversation_id?: string
      message_id: string
      content: string
      timestamp: number
      document_attachments?: any[]
      image_attachments?: any[]
      referenced_files?: any[]
      referenced_messages?: any[]
      referenced_assets?: any[]
      referenced_traffic?: any[]
    }>('agent:user_message', event => {
      const payload = event.payload
      releaseSuppressedExecution(payload.execution_id)
      if (!matchesTarget(payload.execution_id, payload)) return

      markExecutionActive(payload.execution_id)
      startExecutionTiming()
      error.value = null
      contentBuffer.value = ''
      streamingContent.value = ''
      thinkingBuffer.value = ''
      currentThinkingMessageId.value = null
      currentAssistantMessageId.value = null
      assistantSegmentBuffer.value = ''
      if (
        contextCompression.value &&
        (contextCompression.value.executionId !== payload.execution_id ||
          contextCompression.value.generation !== readGeneration(payload))
      ) {
        contextCompression.value = null
      }

      // 添加用户消息，注入待处理的文档附件和图片附件
      const docAttachments =
        payload.document_attachments ||
        (pendingDocumentAttachments.value.length > 0
          ? [...pendingDocumentAttachments.value]
          : undefined)
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

    const unlistenTasks = await listen<AgentTasksUpdatePayload>('agent-tasks-update', event => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id, payload)) return
    })
    unlisteners.push(unlistenTasks)

    // 监听 agent:start 事件（兼容旧版）
    const unlistenStart = await listen<AgentStartEvent>('agent:start', event => {
      const payload = event.payload
      releaseSuppressedExecution(payload.execution_id)
      if (!matchesTarget(payload.execution_id, payload)) return

      markExecutionActive(payload.execution_id)
      startExecutionTiming()
      error.value = null
      contentBuffer.value = ''
      streamingContent.value = ''
      thinkingBuffer.value = ''
      currentThinkingMessageId.value = null
      currentAssistantMessageId.value = null
      assistantSegmentBuffer.value = ''
      if (
        contextCompression.value &&
        (contextCompression.value.executionId !== payload.execution_id ||
          contextCompression.value.generation !== readGeneration(payload))
      ) {
        contextCompression.value = null
      }
      if (shouldSuppressUserMessages()) {
        return
      }

      // 添加用户任务消息（如果没有通过 user_message 事件收到）
      const hasUserMessage = messages.value.some(
        m => m.type === 'user' && m.content === payload.task
      )
      if (!hasUserMessage) {
        // 注入待处理的文档附件
        const docAttachments =
          pendingDocumentAttachments.value.length > 0
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
      effective_context_tokens?: number
      remaining_tokens?: number
      context_pressure?: string
      warning_threshold_tokens?: number
      auto_compact_threshold_tokens?: number
      blocking_threshold_tokens?: number
      output_reserve_tokens?: number
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
    }>('agent:context_usage', event => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id, payload)) return

      contextUsage.value = {
        usedTokens: payload.used_tokens,
        maxTokens: payload.max_tokens,
        usagePercentage: payload.usage_percentage,
        effectiveContextTokens: payload.effective_context_tokens,
        remainingTokens: payload.remaining_tokens,
        contextPressure: payload.context_pressure || null,
        warningThresholdTokens: payload.warning_threshold_tokens,
        autoCompactThresholdTokens: payload.auto_compact_threshold_tokens,
        blockingThresholdTokens: payload.blocking_threshold_tokens,
        outputReserveTokens: payload.output_reserve_tokens,
        shouldCompact:
          payload.context_pressure === 'AutoCompact' || payload.context_pressure === 'Blocking',
        shouldBlock: payload.context_pressure === 'Blocking',
        pressurePhase: 'context_build',
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

    const unlistenContextPressure = await listen<AgentContextPressureEvent>(
      'agent:context_pressure',
      event => {
        const payload = event.payload
        if (!matchesTarget(payload.execution_id, payload)) return

        const maxTokens = Number(
          payload.max_context_tokens ??
            contextUsage.value?.maxTokens ??
            resolveDefaultMaxContextTokens()
        )
        const usedTokens = Number(payload.used_tokens ?? 0) || 0
        const remainingTokens =
          Number(payload.remaining_tokens ?? Math.max(0, maxTokens - usedTokens)) || 0
        const usagePercentage = Number.isFinite(Number(payload.usage_percentage))
          ? Number(payload.usage_percentage)
          : maxTokens > 0
            ? Math.min(100, (usedTokens / maxTokens) * 100)
            : 0
        const next = contextUsage.value ? { ...contextUsage.value } : buildContextUsageSkeleton()
        contextUsage.value = {
          ...next,
          usedTokens,
          maxTokens,
          usagePercentage,
          effectiveContextTokens: payload.effective_context_tokens ?? next.effectiveContextTokens,
          remainingTokens,
          contextPressure: payload.context_pressure || next.contextPressure || null,
          warningThresholdTokens: payload.warning_threshold_tokens ?? next.warningThresholdTokens,
          autoCompactThresholdTokens:
            payload.auto_compact_threshold_tokens ?? next.autoCompactThresholdTokens,
          blockingThresholdTokens:
            payload.blocking_threshold_tokens ?? next.blockingThresholdTokens,
          outputReserveTokens: payload.output_reserve_tokens ?? next.outputReserveTokens,
          shouldCompact: payload.should_compact === true,
          shouldBlock: payload.should_block === true,
          pressurePhase: payload.phase || next.pressurePhase || null,
          systemPromptTokens: payload.system_prompt_tokens ?? next.systemPromptTokens,
          taskTokens: payload.task_tokens ?? next.taskTokens,
          historyTokens: payload.history_tokens ?? next.historyTokens,
          historyCount: payload.history_count ?? next.historyCount,
        }
      }
    )
    unlisteners.push(unlistenContextPressure)

    const unlistenContextCompactionRequested = await listen<AgentContextCompactionRequestedEvent>(
      'agent:context_compaction_requested',
      event => {
        const payload = event.payload
        if (!matchesTarget(payload.execution_id, payload)) return
        if (isSettledActivityEvent(payload.execution_id)) return

        if (!isExecuting.value) {
          markExecutionActive(payload.execution_id)
        }
        error.value = null
        contextCompression.value = {
          active: true,
          executionId: payload.execution_id,
          generation: payload.generation ?? null,
          reason: payload.phase || 'context_pressure',
          recentTokens: Number(payload.used_tokens ?? 0) || 0,
          thresholdTokens: 0,
          messageCount: 0,
          recentMessageCount: 0,
          startedAt: Date.now(),
        }
      }
    )
    unlisteners.push(unlistenContextCompactionRequested)

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
    }>('agent:context_snapshot', event => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id, payload)) return

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
        payload.sentinel_compression_aggressiveness ||
        next.sentinelCompressionAggressiveness ||
        null
      contextUsage.value = next
    })
    unlisteners.push(unlistenContextSnapshot)

    const unlistenContextCompressionStarted = await listen<AgentContextCompressionStartedEvent>(
      'agent:context_compression_started',
      event => {
        const payload = event.payload
        if (!matchesTarget(payload.execution_id, payload)) return
        if (isSettledActivityEvent(payload.execution_id)) return

        if (!isExecuting.value) {
          markExecutionActive(payload.execution_id)
        }
        error.value = null
        contextCompression.value = {
          active: true,
          executionId: payload.execution_id,
          generation: payload.generation ?? null,
          reason: payload.reason,
          recentTokens: Number(payload.recent_tokens ?? 0) || 0,
          thresholdTokens: Number(payload.threshold_tokens ?? 0) || 0,
          messageCount: Number(payload.message_count ?? 0) || 0,
          recentMessageCount: Number(payload.recent_message_count ?? 0) || 0,
          startedAt: Date.now(),
        }
      }
    )
    unlisteners.push(unlistenContextCompressionStarted)

    const unlistenContextCompressionFinished = await listen<AgentContextCompressionFinishedEvent>(
      'agent:context_compression_finished',
      event => {
        const payload = event.payload
        if (!matchesTarget(payload.execution_id, payload)) return
        if (contextCompression.value?.executionId === payload.execution_id) {
          contextCompression.value = null
        }
      }
    )
    unlisteners.push(unlistenContextCompressionFinished)

    const unlistenSubagentStart = await listen<SubagentStartEvent>('subagent:start', event => {
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
    const unlistenIteration = await listen<AgentIterationEvent>('agent:iteration', event => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id, payload)) return
      if (isSettledActivityEvent(payload.execution_id)) return

      // Auto-recover execution state after page refresh
      if (!isExecuting.value) {
        console.log('[useAgentEvents] Auto-recovering execution state from iteration event')
        markExecutionActive(payload.execution_id)
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
        },
      })
    })
    unlisteners.push(unlistenIteration)

    // 监听 agent:chunk 事件
    const unlistenChunk = await listen<AgentChunkEvent>('agent:chunk', event => {
      const payload = event.payload
      const parallelChild = getParallelChild(payload.execution_id)
      if (parallelChild) {
        parallelAgentChunkExecutions.add(payload.execution_id)
        appendParallelChunk(parallelChild.item, payload.chunk_type, payload.content, {
          inputTokens: payload.input_tokens,
          outputTokens: payload.output_tokens,
        })
        flushParallelChild(parallelChild)
        if (parallelChild.item.status === 'failed') {
          settleParallelRunIfDone(parallelChild.run)
        }
        return
      }
      if (!matchesTarget(payload.execution_id, payload)) return
      if (isSettledActivityEvent(payload.execution_id)) return

      // Auto-recover execution state after page refresh
      if (!isExecuting.value) {
        console.log('[useAgentEvents] Auto-recovering execution state from chunk event')
        markExecutionActive(payload.execution_id)
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
            metadata: {
              execution_id: payload.execution_id,
            },
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
          latestUsage.value = {
            inputTokens,
            outputTokens,
          }

          // Update context usage with real values from LLM
          if (contextUsage.value) {
            // Use the real input_tokens from LLM as used_tokens (more accurate than our estimate)
            const maxTokens = contextUsage.value.maxTokens
            const usedTokens = inputTokens + outputTokens
            const usagePercentage =
              maxTokens > 0 ? Math.min(100, (usedTokens / maxTokens) * 100) : 0
            contextUsage.value = {
              ...contextUsage.value,
              usedTokens,
              remainingTokens: Math.max(0, maxTokens - usedTokens),
              usagePercentage,
            }
          } else {
            // If no context usage yet, create a basic one from configured context window.
            const maxTokens = resolveDefaultMaxContextTokens()
            const usedTokens = inputTokens + outputTokens
            const usagePercentage = Math.min(100, (usedTokens / maxTokens) * 100)
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
    const unlistenToolCall = await listen<AgentToolCallEvent>('agent:tool_call', event => {
      const payload = event.payload
      const parallelChild = getParallelChild(payload.execution_id)
      if (parallelChild) {
        appendParallelToolCall(
          parallelChild.item,
          payload.tool_name,
          payload.tool_input,
          payload.tool_id
        )
        flushParallelChild(parallelChild)
        return
      }
      if (!matchesTarget(payload.execution_id, payload)) return
      if (isSettledActivityEvent(payload.execution_id)) return

      // Auto-recover execution state after page refresh
      if (!isExecuting.value) {
        console.log('[useAgentEvents] Auto-recovering execution state from tool_call event')
        markExecutionActive(payload.execution_id)
      }

      if (isShellContinuation(payload.tool_name, payload.tool_input)) {
        return
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
        },
      })

      // ❌ 不要在这里打开终端，等待 tool_result 事件中的 session_id
      // 检测可能返回 terminal session 的 shell 调用
      if (payload.tool_name === 'shell') {
        console.log(
          '[Agent] Detected terminal-capable shell call, will sync terminal session when result arrives'
        )
      }
    })
    unlisteners.push(unlistenToolCall)

    // 监听 agent:tool_call_complete 事件（新格式 - rig-core）
    const unlistenToolCallComplete = await listen<AgentToolCallCompleteEvent>(
      'agent:tool_call_complete',
      event => {
        const payload = event.payload
        const parallelChild = getParallelChild(payload.execution_id)
        if (parallelChild) {
          let parsedArgs: any = {}
          try {
            parsedArgs = JSON.parse(payload.arguments || '{}')
          } catch (e) {
            parsedArgs = { raw: payload.arguments }
          }
          toolCallTracker.set(payload.tool_call_id, {
            tool_name: payload.tool_name,
            arguments: parsedArgs,
            message_id: '',
            message_index: -1,
          })
          appendParallelToolCall(
            parallelChild.item,
            payload.tool_name,
            parsedArgs,
            payload.tool_call_id
          )
          flushParallelChild(parallelChild)
          return
        }
        if (!matchesTarget(payload.execution_id, payload)) return
        if (isSettledActivityEvent(payload.execution_id)) return

        // Auto-recover execution state after page refresh
        if (!isExecuting.value) {
          console.log(
            '[useAgentEvents] Auto-recovering execution state from tool_call_complete event'
          )
          markExecutionActive(payload.execution_id)
        }

        // 解析参数 JSON
        let parsedArgs: any = {}
        try {
          parsedArgs = JSON.parse(payload.arguments || '{}')
        } catch (e) {
          parsedArgs = { raw: payload.arguments }
        }

        const silentShellContinuation = isShellContinuation(payload.tool_name, parsedArgs)

        const messageId = crypto.randomUUID()
        const messageIndex = messages.value.length // 记录消息索引便于后续更新

        // 保存到追踪 Map，用于后续关联结果
        toolCallTracker.set(payload.tool_call_id, {
          tool_name: payload.tool_name,
          arguments: parsedArgs,
          message_id: silentShellContinuation ? '' : messageId,
          message_index: silentShellContinuation ? -1 : messageIndex,
          silent: silentShellContinuation,
        })

        if (silentShellContinuation) {
          return
        }

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
          },
        })

        // ❌ 不要在这里打开终端，等待 tool_result 事件中的 session_id
        // 检测可能返回 terminal session 的 shell 调用
        if (payload.tool_name === 'shell') {
          console.log(
            '[Agent] Detected terminal-capable shell call (complete), will sync terminal session when result arrives'
          )
        }
      }
    )
    unlisteners.push(unlistenToolCallComplete)

    // 监听 agent:tool_result 事件（旧格式兼容）
    const unlistenToolResult = await listen<AgentToolResultEvent>('agent:tool_result', event => {
      const payload = event.payload
      const parallelChild = getParallelChild(payload.execution_id)
      if (parallelChild) {
        const nextPayload = payload as any
        const callInfo = nextPayload.tool_call_id
          ? toolCallTracker.get(nextPayload.tool_call_id)
          : null
        const toolName = payload.tool_name || callInfo?.tool_name || 'unknown'
        const result = nextPayload.result ?? payload.tool_result ?? ''
        const success =
          typeof nextPayload.success === 'boolean' ? nextPayload.success : inferToolSuccess(result)
        appendParallelToolResult(
          parallelChild.item,
          toolName,
          result,
          success,
          nextPayload.tool_call_id
        )
        flushParallelChild(parallelChild)
        return
      }
      if (!matchesTarget(payload.execution_id, payload)) return
      if (isSettledActivityEvent(payload.execution_id)) return

      // Auto-recover execution state after page refresh
      if (!isExecuting.value) {
        console.log('[useAgentEvents] Auto-recovering execution state from tool_result event')
        markExecutionActive(payload.execution_id)
      }

      // 检查是否是新格式（有 tool_call_id 而没有 tool_name）
      const newPayload = payload as any
      if (newPayload.tool_call_id && !newPayload.tool_name) {
        // 新格式：从追踪 Map 获取工具信息
        // 解析结果 JSON
        let resultContent = newPayload.result || ''
        try {
          const parsed = JSON.parse(resultContent)
          resultContent = JSON.stringify(parsed, null, 2)
        } catch (e) {
          // 保持原始字符串
        }

        const callInfo = toolCallTracker.get(newPayload.tool_call_id)
        if (callInfo?.silent && !hasMeaningfulShellResult(resultContent)) {
          toolCallTracker.delete(newPayload.tool_call_id)
          return
        }

        // 更新原有的 tool_call 消息状态，并将结果合并到该消息中
        if (callInfo) {
          const existingMsg = messages.value.find(m => m.id === callInfo.message_id)
          if (callInfo.silent && !existingMsg) {
            const success =
              typeof newPayload.success === 'boolean'
                ? newPayload.success
                : inferToolSuccess(newPayload.result)
            insertMessageBeforeFollowingFinal(
              messages,
              {
                id: crypto.randomUUID(),
                type: 'tool_call',
                content: `工具调用完成: ${callInfo.tool_name}`,
                timestamp: Date.now(),
                metadata: {
                  tool_name: callInfo.tool_name,
                  tool_args: callInfo.arguments,
                  tool_result: resultContent,
                  tool_call_id: newPayload.tool_call_id,
                  status: success ? 'completed' : 'failed',
                  success,
                  execution_id: payload.execution_id,
                  tracked_artifacts: normalizeTrackedArtifacts(newPayload.tracked_artifacts),
                },
              },
              callInfo.message_index
            )
          } else if (existingMsg && existingMsg.metadata) {
            const success =
              typeof newPayload.success === 'boolean'
                ? newPayload.success
                : inferToolSuccess(newPayload.result)
            existingMsg.metadata.status = success ? 'completed' : 'failed'
            existingMsg.metadata.tool_result = resultContent
            existingMsg.metadata.success = success
            existingMsg.metadata.tracked_artifacts = normalizeTrackedArtifacts(
              newPayload.tracked_artifacts
            )
            existingMsg.content = `工具调用完成: ${callInfo.tool_name}`
            pushShellFallbackNotice({
              executionIdForMsg: payload.execution_id,
              messages,
              resultRaw: newPayload.result,
              toolCallId: newPayload.tool_call_id,
              toolName: callInfo.tool_name,
            })

            // 如果 shell 返回 session_id，只同步终端会话绑定，不自动打开终端面板。
            if (callInfo.tool_name === 'shell') {
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
                  const terminal = useTerminal()
                  terminal.syncActiveSession(
                    parsed.session_id,
                    deriveInteractiveShellFingerprint(parsed, callInfo.arguments)
                  )
                }
              } catch (e) {}
            }
          }
        }

        // 从追踪 Map 中移除（不再创建单独的 tool_result 消息）
        toolCallTracker.delete(newPayload.tool_call_id)
      } else {
        if (
          isShellContinuation(payload.tool_name, payload.tool_input) &&
          !hasMeaningfulShellResult(payload.tool_result)
        ) {
          return
        }

        // 旧格式：尝试合并到最近的匹配 tool_call 消息
        const matchingToolCall = messages.value
          .slice()
          .reverse()
          .find(
            m =>
              m.type === 'tool_call' &&
              m.metadata?.tool_name === payload.tool_name &&
              !m.metadata?.tool_result // 还没有结果的
          )

        if (matchingToolCall && matchingToolCall.metadata) {
          const success = inferToolSuccess(payload.tool_result)
          matchingToolCall.metadata.status = success ? 'completed' : 'failed'
          matchingToolCall.metadata.tool_result = payload.tool_result
          matchingToolCall.metadata.success = success
          matchingToolCall.metadata.tracked_artifacts = normalizeTrackedArtifacts(
            (payload as any).tracked_artifacts
          )
          matchingToolCall.content = `工具调用完成: ${payload.tool_name}`
          pushShellFallbackNotice({
            executionIdForMsg: payload.execution_id,
            messages,
            resultRaw: payload.tool_result,
            toolCallId: String(matchingToolCall.metadata?.tool_call_id || ''),
            toolName: payload.tool_name,
          })

          // 旧格式路径：如果 shell 返回 session_id，只同步终端会话绑定，不自动打开终端面板。
          if (payload.tool_name === 'shell') {
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
                const terminal = useTerminal()
                terminal.syncActiveSession(
                  parsed.session_id,
                  deriveInteractiveShellFingerprint(parsed, matchingToolCall.metadata?.tool_args)
                )
              }
            } catch (e) {}
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
            },
          })
        }
      }

      applyFileVerificationStatuses(messages.value)
    })
    unlisteners.push(unlistenToolResult)

    // 监听 agent:tools_selected 事件（仅记录日志，不显示消息）
    const unlistenToolsSelected = await listen<AgentToolsSelectedEvent>(
      'agent:tools_selected',
      event => {
        const payload = event.payload
        if (!matchesTarget(payload.execution_id, payload)) return

        // 仅记录日志，不再添加到消息列表显示
        console.log(`[Agent] Selected ${payload.tools.length} tools:`, payload.tools)
      }
    )
    unlisteners.push(unlistenToolsSelected)

    const unlistenToolsActivated = await listen<AgentToolsActivatedEvent>(
      'agent:tools_activated',
      event => {
        const payload = event.payload
        const parallelChild = getParallelChild(payload.execution_id)
        if (parallelChild) {
          appendParallelSystemEvent(
            parallelChild.item,
            '工具已启用',
            buildToolsPreview(payload.tools)
          )
          flushParallelChild(parallelChild)
          return
        }
        if (!matchesTarget(payload.execution_id, payload)) return

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
          },
        })
      }
    )
    unlisteners.push(unlistenToolsActivated)

    // 监听 agent:skill_loaded 事件（显示技能加载提示）
    const unlistenSkillLoaded = await listen<{
      execution_id: string
      skill_id: string
      skill_name: string
    }>('agent:skill_loaded', event => {
      const payload = event.payload
      const parallelChild = getParallelChild(payload.execution_id)
      if (parallelChild) {
        appendParallelSystemEvent(
          parallelChild.item,
          '技能已加载',
          `${payload.skill_name} (${payload.skill_id})`
        )
        flushParallelChild(parallelChild)
        return
      }
      if (!matchesTarget(payload.execution_id, payload)) return

      messages.value.push({
        id: crypto.randomUUID(),
        type: 'system',
        content: `Skill loaded: ${payload.skill_name} (${payload.skill_id})`,
        timestamp: Date.now(),
        metadata: {
          kind: 'skill_loaded',
          skill_id: payload.skill_id,
          skill_name: payload.skill_name,
        },
      })
    })
    unlisteners.push(unlistenSkillLoaded)

    // 监听 agent:tool_executed 事件
    const unlistenToolExecuted = await listen<AgentToolExecutedEvent>(
      'agent:tool_executed',
      event => {
        const payload = event.payload
        const parallelChild = getParallelChild(payload.execution_id)
        if (parallelChild) {
          appendParallelToolResult(
            parallelChild.item,
            payload.tool,
            payload.result,
            payload.success
          )
          flushParallelChild(parallelChild)
          return
        }
        if (!matchesTarget(payload.execution_id, payload)) return

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
          },
        })
      }
    )
    unlisteners.push(unlistenToolExecuted)

    // 监听助手消息保存成功事件
    const unlistenAssistantSaved = await listen<{
      execution_id: string
      message_id: string
      content: string
      metadata?: Record<string, any> | null
      reasoning_content?: string | null
      timestamp: number
    }>('agent:assistant_message_saved', event => {
      const payload = event.payload
      const parallelChild = getParallelChild(payload.execution_id)
      if (parallelChild) {
        applyParallelSessionStats(parallelChild.item, payload.metadata?.session_stats)
        parallelChild.item.content = payload.content || parallelChild.item.content
        if (parallelChild.item.status === 'pending') {
          parallelChild.item.status = 'running'
        }
        flushParallelChild(parallelChild)
        return
      }
      if (!matchesTarget(payload.execution_id, payload)) return

      console.log('[useAgentEvents] Assistant message saved:', payload.message_id)
      const reasoningContent =
        typeof payload.reasoning_content === 'string' ? payload.reasoning_content.trim() : ''

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
          const previousMessage =
            lastAssistantIndex > 0 ? messages.value[lastAssistantIndex - 1] : null
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
          lastAssistant.metadata = {
            ...(lastAssistant.metadata || {}),
            ...(payload.metadata && typeof payload.metadata === 'object' ? payload.metadata : {}),
            ...(ragMetaInfo.value ? { rag_info: ragMetaInfo.value } : {}),
          }
        }
      }
      currentAssistantMessageId.value = null
      assistantSegmentBuffer.value = ''

      // The assistant response is visible at this point, but the execution is not finished
      // until agent:execution_finished arrives. Only clear streaming buffers here.
      thinkingBuffer.value = ''
      currentThinkingMessageId.value = null
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
    }>('ai_meta_info', event => {
      const payload = event.payload
      if (!matchesTarget(payload.conversation_id, payload)) return

      console.log('[useAgentEvents] Meta info received:', payload)

      if (payload.rag_applied) {
        const used = payload.rag_sources_used === true
        const count = typeof payload.source_count === 'number' ? payload.source_count : 0
        ragMetaInfo.value = {
          rag_applied: true,
          rag_sources_used: used,
          source_count: count,
          citations: payload.citations,
        }
      }
    })
    unlisteners.push(unlistenMetaInfo)

    // 监听 agent:rag_retrieval_complete 事件
    const unlistenRagComplete = await listen<{
      execution_id: string
      citations: any[]
    }>('agent:rag_retrieval_complete', event => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id, payload)) return

      console.log('[useAgentEvents] RAG retrieval complete:', payload)

      if (payload.citations) {
        if (!ragMetaInfo.value) {
          ragMetaInfo.value = {
            rag_applied: true,
            rag_sources_used: payload.citations.length > 0,
            source_count: payload.citations.length,
            citations: payload.citations,
          }
        } else {
          ragMetaInfo.value.citations = payload.citations
          ragMetaInfo.value.rag_sources_used = payload.citations.length > 0
          ragMetaInfo.value.source_count = payload.citations.length
        }
      }
    })
    unlisteners.push(unlistenRagComplete)

    const unlistenExecutionFinished = await listen<AgentExecutionFinishedEvent>(
      'agent:execution_finished',
      event => {
        const payload = event.payload
        const parallelChild = getParallelChild(payload.execution_id)
        if (parallelChild) {
          parallelChild.item.status =
            payload.outcome === 'cancelled' ? 'cancelled' : payload.success ? 'succeeded' : 'failed'
          if (payload.response && payload.response.trim()) {
            parallelChild.item.content = payload.response
          }
          parallelChild.item.completedAtMs = Date.now()
          if (payload.error || payload.message) {
            parallelChild.item.error = payload.error || payload.message || undefined
          }
          flushParallelChild(parallelChild)
          settleParallelRunIfDone(parallelChild.run)
          return
        }
        handleAgentExecutionFinished({
          currentExecutionId,
          error,
          executionStartedAt,
          isExecuting,
          latestUsage,
          markExecutionSettled,
          matchesTarget,
          messages,
          payload,
          ragMetaInfo,
          resetExecutionBuffers,
        })
      }
    )
    unlisteners.push(unlistenExecutionFinished)

    const unlistenSubagentDone = await listen<SubagentDoneEvent>('subagent:done', event => {
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

    const unlistenSubagentError = await listen<SubagentErrorEvent>('subagent:error', event => {
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
    const unlistenSegmentSummary = await listen<AgentSegmentSummaryCreatedEvent>(
      'agent:segment_summary_created',
      event => {
        const payload = event.payload
        if (!matchesTarget(payload.conversation_id, payload)) return

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
          },
        })
      }
    )
    unlisteners.push(unlistenSegmentSummary)

    // 监听 agent:global_summary_updated 事件（滑动窗口全局摘要）
    const unlistenGlobalSummary = await listen<AgentGlobalSummaryUpdatedEvent>(
      'agent:global_summary_updated',
      event => {
        const payload = event.payload
        if (!matchesTarget(payload.conversation_id, payload)) return

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
          },
        })
      }
    )
    unlisteners.push(unlistenGlobalSummary)

    // 监听 agent:retry 事件
    const unlistenRetry = await listen<AgentRetryEvent>('agent:retry', event => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id, payload)) return

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
        },
      })
    })
    unlisteners.push(unlistenRetry)

    // 监听 agent:completion_guard_failed 事件
    const unlistenCompletionGuardFailed = await listen<AgentCompletionGuardFailedEvent>(
      'agent:completion_guard_failed',
      event => {
        const payload = event.payload
        if (!matchesTarget(payload.execution_id, payload)) return

        const reasons = Array.isArray(payload.reasons)
          ? payload.reasons.filter(reason => typeof reason === 'string' && reason.trim().length > 0)
          : []
        const reasonContent =
          reasons.length > 0
            ? `\nReasons:\n${reasons.map((reason, index) => `${index + 1}. ${reason}`).join('\n')}`
            : ''
        const artifactContent = payload.required_artifact
          ? `\nRequired artifact: ${payload.required_artifact}`
          : ''
        const statsContent =
          typeof payload.response_length === 'number' || typeof payload.tool_calls === 'number'
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
          },
        })
      }
    )
    unlisteners.push(unlistenCompletionGuardFailed)

    // 监听 agent:tenth_man_critique 事件
    const unlistenTenthMan = await listen<AgentTenthManCritiqueEvent>(
      'agent:tenth_man_critique',
      event => {
        const payload = event.payload
        if (!matchesTarget(payload.execution_id, payload)) return

        console.log('[useAgentEvents] Tenth Man critique received:', payload.message_id)

        messages.value.push({
          id: payload.message_id,
          type: 'system',
          content: payload.critique,
          timestamp: Date.now(),
          metadata: {
            kind: 'tenth_man_critique',
            execution_id: payload.execution_id,
          },
        })
      }
    )
    unlisteners.push(unlistenTenthMan)

    // 兼容旧的 message_chunk 事件
    const unlistenOldChunk = await listen<OrderedMessageChunk>('message_chunk', event => {
      const chunk = event.payload
      const parallelChild = getParallelChild(chunk.execution_id)
      if (parallelChild) {
        if (parallelAgentChunkExecutions.has(chunk.execution_id)) return
        appendParallelChunk(parallelChild.item, chunk.chunk_type, chunk.content)
        flushParallelChild(parallelChild)
        if (parallelChild.item.status === 'failed') {
          settleParallelRunIfDone(parallelChild.run)
        }
        return
      }
      if (!matchesTarget(chunk.execution_id, chunk)) return
      if (isSettledActivityEvent(chunk.execution_id)) return

      if (!isExecuting.value) {
        markExecutionActive(chunk.execution_id)
      }

      const chunkType = chunk.chunk_type

      if (chunkType === 'Meta' && chunk.stage === 'start') {
        markExecutionActive(chunk.execution_id)
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
            metadata: {
              execution_id: chunk.execution_id,
            },
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
    }>('agent:tenth_man_warning', event => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id, payload)) return

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
        },
      })
    })
    unlisteners.push(unlistenTenthManWarning)

    // 监听 agent:tenth_man_intervention 事件（结论检测时的干预）
    const unlistenTenthManIntervention = await listen<{
      execution_id: string
      trigger: string
      critique: string
      timestamp: number
    }>('agent:tenth_man_intervention', event => {
      const payload = event.payload
      if (!matchesTarget(payload.execution_id, payload)) return

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
        },
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
    }>('shell-background-task-update', event => {
      const payload = event.payload
      if (!payload?.execution_id || !matchesTarget(payload.execution_id, payload)) return
      if (payload.status === 'running') return

      const msgId = crypto.randomUUID()
      const statusLabel =
        payload.status === 'completed'
          ? '后台 Shell 任务完成'
          : payload.status === 'failed'
            ? '后台 Shell 任务失败'
            : '后台 Shell 任务已停止'
      const exitSuffix =
        typeof payload.exit_code === 'number' ? `\n\nExit code: ${payload.exit_code}` : ''
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
    contextCompression,
    parallelTaskSources,
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
