import type { AgentMessage } from '@/types/agent'
import type {
  AgentTeamMessage,
  AgentTeamToolCallEvent,
  AgentTeamToolResultEvent,
} from '@/types/agentTeam'

import {
  buildTeamMirroredConversationRole,
  buildTeamPersistedToolEventKey,
  normalizeToolResult,
  parseToolCallArguments,
} from './agentTeamMessageSupport'
import {
  buildTeamToolCallCompositeKey,
  findTeamToolCallMessage,
} from './agentTeamToolCallSupport'

type TeamToolEventType = 'tool_call' | 'tool_result'

export const collectTeamMirroredSourceIds = (rows: unknown[]): Set<string> => {
  const mirrored = new Set<string>()
  for (const row of rows || []) {
    const payload = (row as any)?.payload
    if (payload && typeof payload === 'object' && typeof payload.source_message_id === 'string') {
      mirrored.add(payload.source_message_id)
    }
  }
  return mirrored
}

export const mirrorTeamMessageToConversation = async (params: {
  msg: AgentTeamMessage
  conversationId?: string | null
  activeTeamSessionId?: string | null
  teamMirroredConversationMessageIds: Set<string>
  shouldSuppressTeamMirrorNoiseMessage: (content: unknown) => boolean
  persistMessage: (request: {
    id: string
    conversation_id: string
    role: string
    content: string
    metadata: Record<string, unknown>
  }) => Promise<void>
}): Promise<void> => {
  const convId = params.conversationId
  const sessionId = params.activeTeamSessionId
  const msg = params.msg
  if (!convId || !sessionId || !msg?.id) return

  const role = buildTeamMirroredConversationRole(msg.role)
  if (!role) return

  const deterministicId = `teamv3:${msg.id}`
  if (params.teamMirroredConversationMessageIds.has(deterministicId)) return

  const rawContent = (msg.content || '').trim()
  if (!rawContent) return
  if (msg.role === 'system' && params.shouldSuppressTeamMirrorNoiseMessage(rawContent)) return

  const memberLabel =
    msg.member_name || msg.member_id || (msg.role === 'system' ? 'team_system' : 'team')
  const mirroredContent = `[Team/${memberLabel}] ${rawContent}`
  const metadata: Record<string, unknown> = {
    kind: 'team_v3_mirror',
    team_session_id: sessionId,
    team_message_id: msg.id,
    team_member_id: msg.member_id,
    team_member_name: msg.member_name,
    team_role: msg.role,
    source: 'team_v3_messages',
  }

  if (msg.role === 'tool_call' || msg.role === 'tool_result') {
    const toolCalls = Array.isArray(msg.tool_calls) ? msg.tool_calls : []
    const firstTool = toolCalls.find((item) => item && typeof item === 'object') as
      | Record<string, unknown>
      | undefined
    const toolName =
      typeof firstTool?.name === 'string' && firstTool.name.trim().length > 0
        ? firstTool.name.trim()
        : 'unknown'
    const parsedArgs = parseToolCallArguments(firstTool?.arguments)
    const hasResult = firstTool?.result !== undefined || msg.role === 'tool_result'
    const resultText = hasResult
      ? normalizeToolResult(firstTool?.result ?? rawContent)
      : undefined
    const success = hasResult ? firstTool?.success !== false : true
    metadata.tool_name = toolName
    metadata.tool_args = parsedArgs
    metadata.tool_result = resultText
    metadata.tool_call_id =
      typeof firstTool?.id === 'string' && firstTool.id.trim().length > 0
        ? firstTool.id.trim()
        : msg.id
    metadata.status = hasResult ? (success ? 'completed' : 'failed') : 'running'
    metadata.success = success
  }

  try {
    await params.persistMessage({
      id: deterministicId,
      conversation_id: convId,
      role,
      content: mirroredContent,
      metadata,
    })
    params.teamMirroredConversationMessageIds.add(deterministicId)
  } catch (error: any) {
    const err = String(error || '')
    if (
      err.includes('UNIQUE constraint failed') ||
      err.toLowerCase().includes('duplicate key') ||
      err.toLowerCase().includes('already exists')
    ) {
      params.teamMirroredConversationMessageIds.add(deterministicId)
      return
    }
    throw error
  }
}

export const persistTeamToolEvent = async (params: {
  messageType: TeamToolEventType
  payload: AgentTeamToolCallEvent | AgentTeamToolResultEvent
  persistedToolEventKeys: Set<string>
  messages: AgentMessage[]
  activeTeamSessionId?: string | null
  sendTeamMessage: (sessionId: string, request: Record<string, unknown>) => Promise<void>
}): Promise<void> => {
  const sessionId = String(params.payload?.session_id || '').trim()
  const toolCallId = String(params.payload?.tool_call_id || '').trim()
  if (!sessionId || !toolCallId) return

  const persistKey = buildTeamPersistedToolEventKey(
    sessionId,
    params.payload.stream_id,
    toolCallId,
    params.messageType,
  )
  if (params.persistedToolEventKeys.has(persistKey)) return
  params.persistedToolEventKeys.add(persistKey)

  try {
    if (params.messageType === 'tool_call') {
      const toolPayload = params.payload as AgentTeamToolCallEvent
      await params.sendTeamMessage(sessionId, {
        thread_id: sessionId,
        from_agent_id: toolPayload.member_id || toolPayload.member_name || 'team_tool',
        to_agent_id: null,
        message_type: 'tool_call',
        payload: {
          content: `[Tool Call] ${toolPayload.name || 'unknown'}`,
          tool_call_id: toolCallId,
          name: toolPayload.name,
          arguments: toolPayload.arguments,
          stream_id: toolPayload.stream_id,
          phase: toolPayload.phase,
          timestamp: toolPayload.timestamp || new Date().toISOString(),
        },
      })
      return
    }

    const resultPayload = params.payload as AgentTeamToolResultEvent
    const existing = findTeamToolCallMessage({
      messages: params.messages,
      compositeKey: buildTeamToolCallCompositeKey({
        activeTeamSessionId: params.activeTeamSessionId,
        toolCallId,
        memberId: resultPayload.member_id,
        memberName: resultPayload.member_name,
        streamId: resultPayload.stream_id,
      }),
      legacyToolCallId: toolCallId,
    })
    const toolName = String(existing?.metadata?.tool_name || 'unknown').trim() || 'unknown'

    await params.sendTeamMessage(sessionId, {
      thread_id: sessionId,
      from_agent_id: resultPayload.member_id || resultPayload.member_name || 'team_tool',
      to_agent_id: null,
      message_type: 'tool_result',
      payload: {
        content: `[Tool Result] ${toolName}`,
        tool_call_id: toolCallId,
        tool_name: toolName,
        result: resultPayload.result,
        success: resultPayload.success !== false,
        stream_id: resultPayload.stream_id,
        phase: resultPayload.phase,
        timestamp: resultPayload.timestamp || new Date().toISOString(),
      },
    })
  } catch (error) {
    params.persistedToolEventKeys.delete(persistKey)
    throw error
  }
}
