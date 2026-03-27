import type { AgentMessage } from '@/types/agent'
import type { AgentTeamMessage } from '@/types/agentTeam'

import { buildTeamMessageSignature } from './agentTeamMessageSupport'
import { resolveTeamStreamTransientInsertIndex } from './agentTeamToolCallSupport'

export const allocateTeamStreamSegmentMessageId = (params: {
  streamId: string
  teamStreamSegmentSeqByStreamId: Map<string, number>
  createId: () => string
}): string => {
  const normalized = String(params.streamId || '').trim()
  if (!normalized) return `team-stream:${params.createId()}`
  const current = params.teamStreamSegmentSeqByStreamId.get(normalized) || 0
  const next = current + 1
  params.teamStreamSegmentSeqByStreamId.set(normalized, next)
  return `team-stream:${normalized}:${next}`
}

export const flushTeamStreamDeltaBuffers = (params: {
  messages: AgentMessage[]
  teamActiveStreamIds: Set<string>
  teamStreamDeltaBufferByStreamId: Map<string, string>
  teamStreamTempMessageByStreamId: Map<string, string>
  streamId?: string
}): void => {
  const targetIds = params.streamId
    ? [params.streamId]
    : [...params.teamStreamDeltaBufferByStreamId.keys()]
  for (const sid of targetIds) {
    const buffered = params.teamStreamDeltaBufferByStreamId.get(sid)
    if (!buffered) continue
    const tempMessageId =
      params.teamStreamTempMessageByStreamId.get(sid) || `team-stream:${sid}`
    const message = params.messages.find((item) => item.id === tempMessageId)
    if (!message) continue
    message.content = `${message.content || ''}${buffered}`
    message.metadata = {
      ...(message.metadata || {}),
      team_streaming: params.teamActiveStreamIds.has(sid),
    }
    params.teamStreamDeltaBufferByStreamId.delete(sid)
  }
}

export const upsertTeamStreamTempMessage = (params: {
  messages: AgentMessage[]
  activeTeamSessionId?: string | null
  teamActiveStreamIds: Set<string>
  teamStreamTempMessageByStreamId: Map<string, string>
  teamStreamSegmentSeqByStreamId: Map<string, number>
  streamId: string
  memberId?: string
  memberName?: string
  options?: { streaming?: boolean }
  insertMainFlowMessageAtPreferredIndex: (
    message: AgentMessage,
    preferredIndex?: number | null
  ) => void
  createId: () => string
}): string => {
  const isStreaming =
    params.options?.streaming ?? params.teamActiveStreamIds.has(params.streamId)
  const tempMessageId =
    params.teamStreamTempMessageByStreamId.get(params.streamId) ||
    allocateTeamStreamSegmentMessageId({
      streamId: params.streamId,
      teamStreamSegmentSeqByStreamId: params.teamStreamSegmentSeqByStreamId,
      createId: params.createId,
    })
  const fallbackStreamIndex = resolveTeamStreamTransientInsertIndex({
    messages: params.messages,
    teamStreamTempMessageByStreamId: params.teamStreamTempMessageByStreamId,
    streamId: params.streamId,
  })

  if (!params.teamStreamTempMessageByStreamId.has(params.streamId)) {
    params.teamStreamTempMessageByStreamId.set(params.streamId, tempMessageId)
    params.insertMainFlowMessageAtPreferredIndex(
      {
        id: tempMessageId,
        type: 'final',
        content: '',
        timestamp: Date.now(),
        metadata: {
          kind: 'team_member_output',
          team_member_id: params.memberId,
          team_member_name: params.memberName,
          team_member_role: 'assistant',
          team_streaming: isStreaming,
          team_stream_id: params.streamId,
          team_session_id: params.activeTeamSessionId || undefined,
        },
      },
      fallbackStreamIndex,
    )
    return tempMessageId
  }

  const existing = params.messages.find((item) => item.id === tempMessageId)
  if (!existing) {
    params.insertMainFlowMessageAtPreferredIndex(
      {
        id: tempMessageId,
        type: 'final',
        content: '',
        timestamp: Date.now(),
        metadata: {
          kind: 'team_member_output',
          team_member_id: params.memberId,
          team_member_name: params.memberName,
          team_member_role: 'assistant',
          team_streaming: isStreaming,
          team_stream_id: params.streamId,
          team_session_id: params.activeTeamSessionId || undefined,
        },
      },
      fallbackStreamIndex,
    )
  } else {
    existing.metadata = {
      ...(existing.metadata || {}),
      kind: 'team_member_output',
      team_member_id: params.memberId,
      team_member_name: params.memberName,
      team_member_role: 'assistant',
      team_streaming: isStreaming,
      team_stream_id: params.streamId,
      team_session_id:
        params.activeTeamSessionId || existing.metadata?.team_session_id,
    }
  }

  return tempMessageId
}

export const markTeamStreamDoneForReconcile = (params: {
  teamStreamDoneIdsBySignature: Map<string, string[]>
  tempMessageId: string
  memberName?: string
  content: string
}): void => {
  const signature = buildTeamMessageSignature(
    'assistant',
    params.memberName,
    params.content,
  )
  const queue = params.teamStreamDoneIdsBySignature.get(signature) || []
  queue.push(params.tempMessageId)
  params.teamStreamDoneIdsBySignature.set(signature, queue)
}

export const splitTeamStreamAssistantSegmentAtToolBoundary = (params: {
  messages: AgentMessage[]
  teamActiveStreamIds: Set<string>
  teamStreamDeltaBufferByStreamId: Map<string, string>
  teamStreamDoneIdsBySignature: Map<string, string[]>
  teamStreamTempMessageByStreamId: Map<string, string>
  streamId: string
  memberName?: string
}): void => {
  const normalizedStreamId = String(params.streamId || '').trim()
  if (!normalizedStreamId) return

  flushTeamStreamDeltaBuffers({
    messages: params.messages,
    teamActiveStreamIds: params.teamActiveStreamIds,
    teamStreamDeltaBufferByStreamId: params.teamStreamDeltaBufferByStreamId,
    teamStreamTempMessageByStreamId: params.teamStreamTempMessageByStreamId,
    streamId: normalizedStreamId,
  })

  const tempMessageId =
    params.teamStreamTempMessageByStreamId.get(normalizedStreamId)
  if (!tempMessageId) return
  const message = params.messages.find((item) => item.id === tempMessageId)

  params.teamStreamTempMessageByStreamId.delete(normalizedStreamId)
  if (!message) return

  const finalizedContent = (message.content || '').trim()
  if (!finalizedContent) {
    const idx = params.messages.findIndex((item) => item.id === tempMessageId)
    if (idx >= 0) {
      params.messages.splice(idx, 1)
    }
    return
  }

  message.metadata = {
    ...(message.metadata || {}),
    team_streaming: false,
  }
  markTeamStreamDoneForReconcile({
    teamStreamDoneIdsBySignature: params.teamStreamDoneIdsBySignature,
    tempMessageId,
    memberName: params.memberName,
    content: finalizedContent,
  })
}

export const consumeTeamStreamTempForPersistedMessage = (params: {
  messages: AgentMessage[]
  teamStreamDoneIdsBySignature: Map<string, string[]>
  teamStreamTempMessageByStreamId: Map<string, string>
  msg: AgentTeamMessage
}): number | null => {
  const signature = buildTeamMessageSignature(
    params.msg.role,
    params.msg.member_name,
    params.msg.content || '',
  )
  const queue = params.teamStreamDoneIdsBySignature.get(signature)
  if (!queue || queue.length === 0) return null
  let removedIndex: number | null = null

  while (queue.length > 0) {
    const tempId = queue.shift()
    if (!tempId) break
    const idx = params.messages.findIndex((item) => item.id === tempId)
    if (idx >= 0) {
      params.messages.splice(idx, 1)
      removedIndex = idx
      for (const [streamId, mappedTempId] of params.teamStreamTempMessageByStreamId.entries()) {
        if (mappedTempId === tempId) {
          params.teamStreamTempMessageByStreamId.delete(streamId)
          break
        }
      }
      break
    }
  }

  if (queue.length === 0) {
    params.teamStreamDoneIdsBySignature.delete(signature)
  } else {
    params.teamStreamDoneIdsBySignature.set(signature, queue)
  }

  return removedIndex
}
