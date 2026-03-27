import type { AgentMessage } from '@/types/agent'

import {
  getTeamMessageSequence,
  inferTeamToolSuccess,
  normalizeTeamSequence,
  normalizeToolResult,
  parseToolCallArguments,
} from './agentTeamMessageSupport'

export interface UpsertTeamToolCallParams {
  toolCallId?: string
  toolName?: string
  toolArgs?: unknown
  toolResult?: unknown
  success?: boolean
  timestamp?: number
  teamSequence?: number
  memberId?: string
  memberName?: string
  streamId?: string
  phase?: string
  mode: 'start' | 'result'
}

export const pushTeamShellFallbackNoticeIfNeeded = (params: {
  messages: AgentMessage[]
  activeTeamSessionId?: string | null
  toolName: string
  toolCallId: string
  resultValue: unknown
  memberId?: string
  memberName?: string
  createId: () => string
}): AgentMessage | null => {
  if (params.toolName.toLowerCase() !== 'shell') return null
  const normalizedCallId = String(params.toolCallId || '').trim()
  if (normalizedCallId) {
    const existed = params.messages.some((item) => {
      if (item.metadata?.kind !== 'shell_fallback_notice') return false
      return String(item.metadata?.tool_call_id || '').trim() === normalizedCallId
    })
    if (existed) return null
  }

  const notice = buildShellFallbackNoticeFromResult(params.resultValue)
  if (!notice) return null

  return {
    id: params.createId(),
    type: 'system',
    content: notice,
    timestamp: Date.now(),
    metadata: {
      kind: 'shell_fallback_notice',
      tool_call_id: normalizedCallId || undefined,
      team_member_id: params.memberId,
      team_member_name: params.memberName,
      team_session_id: params.activeTeamSessionId || undefined,
    },
  }
}

const buildShellFallbackNoticeFromResult = (resultValue: unknown): string | null => {
  const parsed = parseToolResultObject(resultValue)
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

const parseToolResultObject = (value: unknown): Record<string, any> | null => {
  if (!value) return null
  if (typeof value === 'object' && !Array.isArray(value)) {
    return value as Record<string, any>
  }
  if (typeof value !== 'string') return null
  try {
    const parsed = JSON.parse(value)
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      return parsed as Record<string, any>
    }
  } catch {
    return null
  }
  return null
}

export const resolveTeamSequenceInsertIndex = (params: {
  messages: AgentMessage[]
  teamSequence?: number | null
  isTeamScopedMainFlowMessage: (message: AgentMessage) => boolean
}): number | null => {
  const seq = normalizeTeamSequence(params.teamSequence)
  if (seq === undefined) return null
  for (let i = 0; i < params.messages.length; i += 1) {
    const candidate = params.messages[i]
    if (!params.isTeamScopedMainFlowMessage(candidate)) continue
    const existingSeq = getTeamMessageSequence(candidate)
    if (existingSeq !== undefined && existingSeq > seq) {
      return i
    }
  }
  return null
}

export const resolveTeamStreamTransientInsertIndex = (params: {
  messages: AgentMessage[]
  teamStreamTempMessageByStreamId: Map<string, string>
  streamId?: string | null
}): number | null => {
  const normalizedStreamId = String(params.streamId || '').trim()
  if (!normalizedStreamId) return null

  const tempMessageId = params.teamStreamTempMessageByStreamId.get(normalizedStreamId)
  let lastStreamIdx = -1
  for (let i = 0; i < params.messages.length; i += 1) {
    const item = params.messages[i]
    const itemStreamId = String(item.metadata?.team_stream_id || '').trim()
    const sameTemp = !!tempMessageId && item.id === tempMessageId
    if (itemStreamId !== normalizedStreamId && !sameTemp) continue
    lastStreamIdx = i
  }
  if (lastStreamIdx >= 0) return lastStreamIdx + 1
  return null
}

export const buildTeamToolCallCompositeKey = (params: {
  activeTeamSessionId?: string | null
  toolCallId?: string | null
  memberId?: string | null
  memberName?: string | null
  streamId?: string | null
}): string | null => {
  const normalizedCallId = String(params.toolCallId || '').trim()
  if (!normalizedCallId) return null
  const normalizedMemberId = String(params.memberId || '').trim()
  const normalizedMemberName = String(params.memberName || '').trim()
  const normalizedStreamId = String(params.streamId || '').trim()
  const sessionId = String(params.activeTeamSessionId || '').trim()
  if (normalizedStreamId) {
    return `team:${sessionId}:stream:${normalizedStreamId}:call:${normalizedCallId}`
  }
  const ownerKey = normalizedMemberId || normalizedMemberName || 'unknown'
  return `team:${sessionId}:owner:${ownerKey}:call:${normalizedCallId}`
}

export const findTeamToolCallMessage = (params: {
  messages: AgentMessage[]
  compositeKey?: string | null
  legacyToolCallId?: string | null
  allowLegacyFallback?: boolean
}): AgentMessage | null => {
  const normalizedComposite = String(params.compositeKey || '').trim()
  const normalizedLegacyId = String(params.legacyToolCallId || '').trim()
  const allowLegacyFallback = params.allowLegacyFallback !== false
  if (!normalizedComposite && !normalizedLegacyId) return null
  if (normalizedComposite) {
    const byComposite =
      params.messages.find((item) => {
        if (item.id === normalizedComposite) return true
        const existingComposite = String(item.metadata?.team_tool_call_key || '').trim()
        return existingComposite.length > 0 && existingComposite === normalizedComposite
      }) || null
    if (byComposite) return byComposite
    if (!allowLegacyFallback) return null
  }
  if (!normalizedLegacyId) return null
  return (
    params.messages.find((item) => {
      if (item.id === normalizedLegacyId) return true
      const existingCallId = String(item.metadata?.tool_call_id || '').trim()
      return existingCallId.length > 0 && existingCallId === normalizedLegacyId
    }) || null
  )
}

export const upsertTeamToolCallInMainFlow = (params: {
  messages: AgentMessage[]
  activeTeamSessionId?: string | null
  teamStreamTempMessageByStreamId: Map<string, string>
  isTeamScopedMainFlowMessage: (message: AgentMessage) => boolean
  insertMainFlowMessageAtPreferredIndex: (
    message: AgentMessage,
    preferredIndex?: number | null
  ) => void
  appendMainFlowMessage: (message: AgentMessage) => void
  createId: () => string
  upsert: UpsertTeamToolCallParams
}): void => {
  const normalizedId = (params.upsert.toolCallId || '').trim()
  const compositeKey = buildTeamToolCallCompositeKey({
    activeTeamSessionId: params.activeTeamSessionId,
    toolCallId: normalizedId,
    memberId: params.upsert.memberId,
    memberName: params.upsert.memberName,
    streamId: params.upsert.streamId,
  })
  const hasStreamScope = String(params.upsert.streamId || '').trim().length > 0
  const stableId = compositeKey || normalizedId || `team-toolcall:${params.createId()}`
  const existing = findTeamToolCallMessage({
    messages: params.messages,
    compositeKey: compositeKey || stableId,
    legacyToolCallId: normalizedId,
    allowLegacyFallback: !hasStreamScope,
  })
  const existingMeta = (existing?.metadata || {}) as Record<string, any>
  const toolName =
    (params.upsert.toolName || existingMeta.tool_name || 'unknown').trim() || 'unknown'
  const nextArgs =
    params.upsert.toolArgs !== undefined
      ? parseToolCallArguments(params.upsert.toolArgs)
      : parseToolCallArguments(existingMeta.tool_args)
  const nextResult =
    params.upsert.toolResult !== undefined
      ? normalizeToolResult(params.upsert.toolResult)
      : normalizeToolResult(existingMeta.tool_result)
  const success =
    typeof params.upsert.success === 'boolean'
      ? params.upsert.success
      : params.upsert.mode === 'result'
        ? inferTeamToolSuccess(nextResult)
        : existingMeta.success !== false
  const existingStatus = String(existingMeta.status || '').toLowerCase()
  const hasTerminalStatus =
    existingStatus === 'completed' || existingStatus === 'failed'
  const isLateStartAfterTerminal =
    params.upsert.mode === 'start' && hasTerminalStatus
  const status: 'running' | 'completed' | 'failed' = isLateStartAfterTerminal
    ? (existingStatus as 'completed' | 'failed')
    : params.upsert.mode === 'start'
      ? 'running'
      : success
        ? 'completed'
        : 'failed'
  const content =
    status === 'running' ? `正在调用工具: ${toolName}` : `工具调用完成: ${toolName}`
  const timestamp = existing?.timestamp ?? params.upsert.timestamp ?? Date.now()
  const teamSequence = normalizeTeamSequence(
    params.upsert.teamSequence ?? existingMeta.team_sequence
  )
  const metadata = {
    ...existingMeta,
    kind: 'tool_call',
    tool_name: toolName,
    tool_args: nextArgs,
    tool_result: nextResult,
    tool_call_id: normalizedId || stableId,
    team_tool_call_key: compositeKey || existingMeta.team_tool_call_key,
    status,
    success,
    team_member_id: params.upsert.memberId || existingMeta.team_member_id,
    team_member_name: params.upsert.memberName || existingMeta.team_member_name,
    team_stream_id: params.upsert.streamId || existingMeta.team_stream_id,
    team_session_id: params.activeTeamSessionId || existingMeta.team_session_id,
    team_sequence: teamSequence,
  }

  if (existing) {
    existing.type = 'tool_call'
    existing.content = content
    existing.timestamp = timestamp
    existing.metadata = metadata
    if (params.upsert.mode === 'result') {
      const notice = pushTeamShellFallbackNoticeIfNeeded({
        messages: params.messages,
        activeTeamSessionId: params.activeTeamSessionId,
        toolName,
        toolCallId: normalizedId || stableId,
        resultValue: nextResult,
        memberId: metadata.team_member_id,
        memberName: metadata.team_member_name,
        createId: params.createId,
      })
      if (notice) {
        params.appendMainFlowMessage(notice)
      }
    }
    return
  }

  const preferredIndex = resolveTeamSequenceInsertIndex({
    messages: params.messages,
    teamSequence,
    isTeamScopedMainFlowMessage: params.isTeamScopedMainFlowMessage,
  })
  const fallbackStreamIndex = resolveTeamStreamTransientInsertIndex({
    messages: params.messages,
    teamStreamTempMessageByStreamId: params.teamStreamTempMessageByStreamId,
    streamId: params.upsert.streamId,
  })
  params.insertMainFlowMessageAtPreferredIndex(
    {
      id: stableId,
      type: 'tool_call',
      content,
      timestamp,
      metadata,
    },
    preferredIndex ?? fallbackStreamIndex,
  )
  if (params.upsert.mode === 'result') {
    const notice = pushTeamShellFallbackNoticeIfNeeded({
      messages: params.messages,
      activeTeamSessionId: params.activeTeamSessionId,
      toolName,
      toolCallId: normalizedId || stableId,
      resultValue: nextResult,
      memberId: metadata.team_member_id,
      memberName: metadata.team_member_name,
      createId: params.createId,
    })
    if (notice) {
      params.appendMainFlowMessage(notice)
    }
  }
}
