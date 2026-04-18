import type { AgentMessage } from '@/types/agent'
import type { AgentTeamMessage } from '@/types/agentTeam'
import type { UpsertTeamToolCallParams } from './agentTeamToolCallSupport'

import {
  buildTeamPersistedAssistantSuppressionKey,
  mapTeamMessageType,
  normalizeTeamSequence,
  parseTeamMessageTimestamp,
  shouldSuppressTeamMirrorNoiseMessage,
} from './agentTeamMessageSupport'

export interface AppendTeamMessagesToMainFlowParams {
  messagesResp: AgentTeamMessage[]
  teamMainFlowMessageIds: Set<string>
  teamMirroredConversationMessageIds: Set<string>
  teamPersistedAssistantSuppressionKeys: Set<string>
  pushMainFlowMessage: (message: AgentMessage) => void
  insertMainFlowMessageAtPreferredIndex: (
    message: AgentMessage,
    preferredIndex?: number | null
  ) => void
  upsertTeamToolCallToMainFlow: (params: UpsertTeamToolCallParams) => void
  mirrorTeamMessageToConversation: (msg: AgentTeamMessage) => void | Promise<void>
  consumeTeamLocalHumanInputForPersistedMessage: (
    sessionId: string,
    content: string
  ) => number | null
  consumeTeamStreamTempForPersistedMessage: (
    msg: AgentTeamMessage
  ) => number | null
}

export const appendTeamMessagesToMainFlow = (
  params: AppendTeamMessagesToMainFlowParams
) => {
  const { messagesResp } = params
  if (!Array.isArray(messagesResp) || messagesResp.length === 0) return

  for (const msg of messagesResp) {
    if (!msg?.id || params.teamMainFlowMessageIds.has(msg.id)) continue

    const mirroredId = `teamv3:${msg.id}`
    if (params.teamMirroredConversationMessageIds.has(mirroredId)) {
      params.teamMainFlowMessageIds.add(msg.id)
      continue
    }

    const msgTime = parseTeamMessageTimestamp(msg.timestamp)

    if (msg.role === 'system') {
      const systemContent = (msg.content || '').trim()
      if (systemContent && !shouldSuppressTeamMirrorNoiseMessage(systemContent)) {
        const teamMetadata =
          msg.metadata && typeof msg.metadata === 'object' ? msg.metadata : {}
        const kind =
          typeof teamMetadata.kind === 'string' && teamMetadata.kind.trim().length > 0
            ? teamMetadata.kind.trim()
            : 'team_system'
        params.pushMainFlowMessage({
          id: `team:${msg.id}`,
          type: 'system',
          content: systemContent,
          timestamp: msgTime,
          metadata: {
            ...teamMetadata,
            kind,
            team_member_id: msg.member_id,
            team_member_name: msg.member_name,
            team_member_role: msg.role,
            team_session_id: msg.session_id,
            team_task_record_id:
              typeof teamMetadata.task_record_id === 'string'
                ? teamMetadata.task_record_id
                : undefined,
            team_task_key:
              typeof teamMetadata.task_key === 'string'
                ? teamMetadata.task_key
                : undefined,
            team_task_title:
              typeof teamMetadata.task_title === 'string'
                ? teamMetadata.task_title
                : undefined,
            action_label:
              typeof teamMetadata.action_label === 'string'
                ? teamMetadata.action_label
                : undefined,
            team_sequence: normalizeTeamSequence(msg.sequence),
          },
        })
        void params.mirrorTeamMessageToConversation(msg)
      }
      params.teamMainFlowMessageIds.add(msg.id)
      continue
    }

    if (msg.role === 'tool_call' || msg.role === 'tool_result') {
      const toolCalls = Array.isArray(msg.tool_calls) ? msg.tool_calls : []
      if (toolCalls.length > 0) {
        for (let i = 0; i < toolCalls.length; i += 1) {
          const tc = toolCalls[i]
          if (!tc || typeof tc !== 'object') continue
          const hasResult = (tc as any).result !== undefined
          const persistedStreamId =
            typeof (tc as any).stream_id === 'string'
              ? (tc as any).stream_id
              : msg.stream_id
          params.upsertTeamToolCallToMainFlow({
            toolCallId:
              typeof (tc as any).id === 'string'
                ? (tc as any).id
                : `team:${msg.id}:tool:${i}`,
            toolName: typeof (tc as any).name === 'string' ? (tc as any).name : 'unknown',
            toolArgs: (tc as any).arguments,
            toolResult: hasResult ? (tc as any).result : undefined,
            success: hasResult ? (tc as any).success !== false : undefined,
            timestamp: msgTime,
            teamSequence: msg.sequence,
            memberId: msg.member_id,
            memberName: msg.member_name,
            streamId: persistedStreamId,
            mode: msg.role === 'tool_result' || hasResult ? 'result' : 'start',
          })
        }
      } else {
        params.upsertTeamToolCallToMainFlow({
          toolCallId: `team:${msg.id}`,
          toolName: msg.content || 'unknown',
          toolResult: msg.role === 'tool_result' ? msg.content : undefined,
          success: msg.role === 'tool_result' ? true : undefined,
          timestamp: msgTime,
          teamSequence: msg.sequence,
          memberId: msg.member_id,
          memberName: msg.member_name,
          streamId: msg.stream_id,
          mode: msg.role === 'tool_result' ? 'result' : 'start',
        })
      }
      void params.mirrorTeamMessageToConversation(msg)
      params.teamMainFlowMessageIds.add(msg.id)
      continue
    }

    if (msg.role === 'assistant') {
      const assistantContent = (msg.content || '').trim()
      if (assistantContent) {
        const suppressionKey = buildTeamPersistedAssistantSuppressionKey(
          msg.session_id,
          msg.member_name,
          assistantContent,
        )
        if (params.teamPersistedAssistantSuppressionKeys.has(suppressionKey)) {
          params.teamMainFlowMessageIds.add(msg.id)
          void params.mirrorTeamMessageToConversation(msg)
          continue
        }
      }
    }

    if ((msg.content || '').trim()) {
      let preferredIndex: number | null = null
      if (msg.role === 'user') {
        preferredIndex = params.consumeTeamLocalHumanInputForPersistedMessage(
          msg.session_id,
          msg.content || '',
        )
      }
      const kind = msg.role === 'user' ? 'team_human_input' : 'team_member_output'
      const streamTempIndex = params.consumeTeamStreamTempForPersistedMessage(msg)
      if (preferredIndex === null && streamTempIndex !== null) {
        preferredIndex = streamTempIndex
      }
      params.teamMainFlowMessageIds.add(msg.id)
      params.insertMainFlowMessageAtPreferredIndex(
        {
          id: `team:${msg.id}`,
          type: mapTeamMessageType(msg.role),
          content: msg.content,
          timestamp: msgTime,
          metadata: {
            kind,
            team_member_id: msg.member_id,
            team_member_name: msg.member_name,
            team_member_role: msg.role,
            team_stream_id: msg.stream_id,
            team_session_id: msg.session_id,
            team_sequence: normalizeTeamSequence(msg.sequence),
          },
        },
        preferredIndex,
      )
      void params.mirrorTeamMessageToConversation(msg)
    }

    if (msg.role === 'assistant' && Array.isArray(msg.tool_calls)) {
      for (let i = 0; i < msg.tool_calls.length; i += 1) {
        const tc = msg.tool_calls[i]
        if (!tc || typeof tc !== 'object') continue
        const toolCallId = typeof tc.id === 'string' ? tc.id : `team:${msg.id}:tool:${i}`
        const hasResult = (tc as any).result !== undefined
        params.upsertTeamToolCallToMainFlow({
          toolCallId,
          toolName: typeof tc.name === 'string' ? tc.name : 'unknown',
          toolArgs: (tc as any).arguments,
          toolResult: hasResult ? (tc as any).result : undefined,
          success: hasResult ? (tc as any).success !== false : undefined,
          timestamp: msgTime,
          teamSequence: msg.sequence,
          memberId: msg.member_id,
          memberName: msg.member_name,
          streamId: typeof (tc as any).stream_id === 'string' ? (tc as any).stream_id : msg.stream_id,
          mode: hasResult ? 'result' : 'start',
        })
      }
    }
  }
}
