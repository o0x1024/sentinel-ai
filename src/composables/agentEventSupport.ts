import type { Ref } from 'vue'

import type { AgentMessage } from '@/types/agent'
import { buildTerminalSessionFingerprint } from '@/composables/useTerminal'
import { buildAgentSessionStats } from '@/components/Agent/agentSessionStatsSupport'
import type {
  AgentExecutionFinishedEvent,
  ContextUsageInfo,
  MemoryRetrievalInfo,
  RagMetaInfo,
} from '@/composables/useAgentEventTypes'

export const isShellContinuation = (toolName: string | undefined | null, args: any): boolean => {
  if (
    String(toolName || '')
      .trim()
      .toLowerCase() !== 'shell'
  ) {
    return false
  }
  if (!args || typeof args !== 'object' || Array.isArray(args)) return false

  const action = typeof args.action === 'string' ? args.action.trim().toLowerCase() : ''
  if (['poll', 'key', 'write', 'submit', 'cancel', 'stop'].includes(action)) return true

  const hasSession = Boolean(String(args.session_id || args.process_id || '').trim())
  const hasCommand = Boolean(String(args.command || args.cmd || '').trim())
  const hasInput = ['chars', 'input', 'input_text'].some(key =>
    Object.prototype.hasOwnProperty.call(args, key)
  )
  return hasSession && !hasCommand && !hasInput
}

export const parseShellResultPayload = (value: any): any => {
  if (typeof value === 'string') {
    const trimmed = value.trim()
    if (!trimmed) return ''
    try {
      return parseShellResultPayload(JSON.parse(trimmed))
    } catch {
      return value
    }
  }

  if (Array.isArray(value)) {
    const textItem = value.find(
      item => item && typeof item === 'object' && typeof item.text === 'string'
    )
    if (textItem) return parseShellResultPayload(textItem.text)
  }

  return value
}

export const hasMeaningfulShellResult = (result: any): boolean => {
  const parsed = parseShellResultPayload(result)
  if (typeof parsed === 'string') return parsed.trim().length > 0
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return false

  const output = parsed.output && typeof parsed.output === 'object' ? parsed.output : {}
  const textFields = [
    parsed.stdout,
    parsed.stderr,
    parsed.error,
    output.stdout,
    output.stderr,
    output.error,
  ]
  return textFields.some(value => typeof value === 'string' && value.trim().length > 0)
}

export const insertMessageBeforeFollowingFinal = (
  messages: Ref<AgentMessage[]>,
  message: AgentMessage,
  preferredIndex: number
) => {
  const start = Math.max(0, Math.min(preferredIndex, messages.value.length))
  let insertAt = messages.value.length
  for (let index = start; index < messages.value.length; index += 1) {
    if (messages.value[index]?.type === 'final') {
      insertAt = index
      break
    }
  }
  messages.value.splice(insertAt, 0, message)
}

export const resolveDefaultMaxContextTokens = (raw: unknown): number => {
  if (typeof raw === 'number') {
    return Number.isFinite(raw) && raw > 0 ? Math.floor(raw) : 128000
  }
  if (raw && typeof raw === 'object' && 'value' in raw) {
    const value = Number((raw as { value?: unknown }).value)
    return Number.isFinite(value) && value > 0 ? Math.floor(value) : 128000
  }
  return 128000
}

export const buildContextUsageSkeleton = (maxTokens: number): ContextUsageInfo => ({
  usedTokens: 0,
  maxTokens,
  usagePercentage: 0,
  remainingTokens: 0,
  contextPressure: null,
  pressurePhase: null,
  systemPromptTokens: 0,
  historyTokens: 0,
  historyCount: 0,
  summaryTokens: 0,
  summaryGlobalTokens: 0,
  summarySegmentTokens: 0,
  summarySegmentCount: 0,
  memoryRetrieval: null,
})

export const mapMemoryRetrieval = (raw: any): MemoryRetrievalInfo | null => {
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

export const readGeneration = (payload?: any): number | null => {
  const raw = payload?.generation ?? payload?.structured_data?.generation
  const value = Number(raw)
  return Number.isFinite(value) && value > 0 ? value : null
}

export const createAgentEventTargetMatcher = (params: {
  currentExecutionId: Ref<string | null>
  currentGeneration: Ref<number | null>
  executionId?: Ref<string> | string
  isExecuting: Ref<boolean>
  suppressedExecutionId: Ref<string | null>
}) => {
  const hasExplicitTarget = params.executionId !== undefined

  const getTargetId = (): string | undefined => {
    if (!hasExplicitTarget) return undefined
    const raw =
      typeof params.executionId === 'string' ? params.executionId : params.executionId?.value
    return typeof raw === 'string' && raw.trim().length > 0 ? raw : undefined
  }

  const matchesTarget = (eventExecId: string, payload?: any): boolean => {
    const targetId = getTargetId()
    if (hasExplicitTarget && !targetId) return false
    if (params.suppressedExecutionId.value && eventExecId === params.suppressedExecutionId.value) {
      return false
    }
    if (targetId && eventExecId !== targetId) {
      const payloadConversationId = String(payload?.conversation_id || '').trim()
      const isCurrentExecution = params.currentExecutionId.value === eventExecId
      const belongsToTargetConversation = payloadConversationId === targetId
      if (!isCurrentExecution && !belongsToTargetConversation) return false
    }
    const generation = readGeneration(payload)
    if (generation !== null) {
      if (
        params.currentGeneration.value !== null &&
        params.currentGeneration.value !== generation
      ) {
        if (generation < params.currentGeneration.value) return false
        if (params.isExecuting.value && params.currentExecutionId.value === eventExecId) {
          return false
        }
      }
      params.currentGeneration.value = generation
      return true
    }
    if (targetId && params.currentGeneration.value !== null && eventExecId === targetId) {
      return false
    }
    return true
  }

  const isSuppressedExecution = (eventExecId: string): boolean =>
    !!params.suppressedExecutionId.value && params.suppressedExecutionId.value === eventExecId

  const releaseSuppressedExecution = (eventExecId: string): void => {
    if (!isSuppressedExecution(eventExecId)) return
    console.log('[useAgentEvents] Releasing suppressed execution:', eventExecId)
    params.suppressedExecutionId.value = null
  }

  return {
    getTargetId,
    isSuppressedExecution,
    matchesTarget,
    releaseSuppressedExecution,
  }
}

export const deriveInteractiveShellFingerprint = (
  parsedResult: any,
  toolArgs?: any
): string | undefined => {
  const explicit =
    typeof parsedResult?.session_fingerprint === 'string'
      ? parsedResult.session_fingerprint.trim()
      : ''
  if (explicit) return explicit

  const executionMode = parsedResult?.execution_mode ?? toolArgs?.execution_mode ?? 'docker'
  const dockerImage =
    parsedResult?.docker_image ?? toolArgs?.docker_image ?? 'sentinel-sandbox:latest'
  const shell = parsedResult?.shell ?? toolArgs?.shell ?? 'bash'
  const workingDirectory = parsedResult?.working_dir ?? toolArgs?.working_dir ?? ''
  if (
    typeof executionMode !== 'string' ||
    typeof dockerImage !== 'string' ||
    typeof shell !== 'string' ||
    typeof workingDirectory !== 'string'
  ) {
    return undefined
  }

  return buildTerminalSessionFingerprint(
    executionMode === 'host' ? 'host' : 'docker',
    dockerImage,
    shell,
    workingDirectory
  )
}

export const inferToolSuccess = (raw: any): boolean => {
  const isStructuredHttpResponse = (value: Record<string, any>): boolean =>
    typeof value.status_code === 'number' &&
    typeof value.headers === 'object' &&
    value.headers !== null &&
    (typeof value.url === 'string' || typeof value.status_text === 'string')

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

export const tryParseJsonObject = (raw: unknown): Record<string, any> | null => {
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

export const normalizeTrackedArtifacts = (raw: unknown): any[] | undefined =>
  Array.isArray(raw) ? raw : undefined

export const buildShellFallbackNotice = (resultRaw: unknown): string | null => {
  const parsed = tryParseJsonObject(resultRaw)
  if (!parsed) return null
  const fallbackFrom = String(parsed.fallback_from || '')
    .trim()
    .toLowerCase()
  const executionMode = String(parsed.execution_mode || '')
    .trim()
    .toLowerCase()
  if (fallbackFrom !== 'docker' || executionMode !== 'host') return null
  const reason = String(parsed.fallback_reason || '').trim()
  if (reason) {
    return `系统提示: shell 工具在 Docker 中执行失败，已自动回退到宿主机。原因: ${reason}`
  }
  return '系统提示: shell 工具在 Docker 中执行失败，已自动回退到宿主机。'
}

export const pushShellFallbackNotice = (params: {
  executionIdForMsg: string
  messages: Ref<AgentMessage[]>
  resultRaw: unknown
  toolCallId?: string
  toolName: string | undefined
}) => {
  if ((params.toolName || '').toLowerCase() !== 'shell') return
  const notice = buildShellFallbackNotice(params.resultRaw)
  if (!notice) return
  const normalizedToolCallId = String(params.toolCallId || '').trim()
  if (normalizedToolCallId) {
    const existed = params.messages.value.some(item => {
      if (item.metadata?.kind !== 'shell_fallback_notice') return false
      return String(item.metadata?.tool_call_id || '').trim() === normalizedToolCallId
    })
    if (existed) return
  }
  params.messages.value.push({
    id: crypto.randomUUID(),
    type: 'system',
    content: notice,
    timestamp: Date.now(),
    metadata: {
      kind: 'shell_fallback_notice',
      execution_id: params.executionIdForMsg,
      tool_call_id: normalizedToolCallId || undefined,
    },
  })
}

export const attachSessionStatsToLatestAssistant = (params: {
  endedAt: number
  executionId: string
  executionStartedAt: Ref<number | null>
  latestUsage: Ref<{ inputTokens: number; outputTokens: number } | null>
  messages: Ref<AgentMessage[]>
}) => {
  const stats = buildAgentSessionStats({
    startedAt: params.executionStartedAt.value,
    endedAt: params.endedAt,
    inputTokens: params.latestUsage.value?.inputTokens,
    outputTokens: params.latestUsage.value?.outputTokens,
  })
  if (!stats) return

  for (let i = params.messages.value.length - 1; i >= 0; i -= 1) {
    const message = params.messages.value[i]
    if (message.type !== 'final') continue
    if (message.metadata?.execution_id !== params.executionId) continue
    message.metadata = {
      ...(message.metadata || {}),
      session_stats: stats,
    }
    return
  }
}

export const handleAgentExecutionFinished = (params: {
  currentExecutionId: Ref<string | null>
  error: Ref<string | null>
  isExecuting: Ref<boolean>
  latestUsage: Ref<{ inputTokens: number; outputTokens: number } | null>
  markExecutionSettled: (executionId: string) => void
  matchesTarget: (executionId: string, payload?: any) => boolean
  messages: Ref<AgentMessage[]>
  payload: AgentExecutionFinishedEvent
  ragMetaInfo: Ref<RagMetaInfo | null>
  resetExecutionBuffers: () => void
  executionStartedAt: Ref<number | null>
}) => {
  const { payload } = params
  if (!params.matchesTarget(payload.execution_id, payload)) return

  if (
    payload.outcome === 'cancelled' &&
    params.isExecuting.value &&
    params.currentExecutionId.value === payload.execution_id
  ) {
    console.log(
      '[useAgentEvents] Ignoring stale cancelled event for active execution:',
      payload.execution_id
    )
    return
  }

  params.resetExecutionBuffers()
  params.markExecutionSettled(payload.execution_id)

  if (payload.outcome === 'failed') {
    const err = payload.error || 'Agent execution failed'
    params.error.value = err
    params.messages.value.push({
      id: crypto.randomUUID(),
      type: 'error',
      content: err,
      timestamp: Date.now(),
    })
    return
  }

  params.error.value = null

  if (payload.outcome === 'succeeded') {
    attachSessionStatsToLatestAssistant({
      endedAt: Date.now(),
      executionId: payload.execution_id,
      executionStartedAt: params.executionStartedAt,
      latestUsage: params.latestUsage,
      messages: params.messages,
    })
    if (params.ragMetaInfo.value) {
      const lastAssistant = [...params.messages.value].reverse().find(m => m.type === 'final')
      if (lastAssistant) {
        lastAssistant.metadata = {
          ...(lastAssistant.metadata || {}),
          rag_info: params.ragMetaInfo.value,
        }
      }
    }
    return
  }

  console.log('[useAgentEvents] Execution cancelled:', payload.execution_id)
}
