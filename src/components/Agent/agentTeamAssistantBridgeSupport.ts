import type { AgentMessage } from '@/types/agent'

import { parseConversationMessageTimestamp } from './agentTeamMessageSupport'

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

export const syncLatestAssistantOutputToTeamSession = async (params: {
  conversationId: string
  loadConversationMessages: (conversationId: string) => Promise<any[]>
  mirrorAssistantOutputToTeamSession: (
    sessionId: string,
    sourceMessageId: string,
    content: string,
  ) => Promise<void>
  sessionId: string
}): Promise<void> => {
  const conversationMessages = await params.loadConversationMessages(params.conversationId)
  const latestAssistant = (conversationMessages || [])
    .filter((row) => row?.role === 'assistant' && typeof row?.id === 'string')
    .sort(
      (a, b) =>
        parseConversationMessageTimestamp(b?.timestamp) -
        parseConversationMessageTimestamp(a?.timestamp),
    )[0]
  if (!latestAssistant) return

  const content = typeof latestAssistant.content === 'string' ? latestAssistant.content : ''
  await params.mirrorAssistantOutputToTeamSession(params.sessionId, latestAssistant.id, content)
}

export const mirrorAssistantOutputToTeamSession = async (params: {
  collectPersistedSourceIds: (sessionId: string) => Promise<Set<string>>
  loadTeamWorkspaceData?: () => Promise<void>
  mirroredAssistantSourceIds: Set<string>
  normalizedContent: string
  normalizedSourceMessageId: string
  sendTeamMessage: (sessionId: string, request: Record<string, unknown>) => Promise<void>
  sessionId: string
  syncTeamMessagesToMainFlow: (sessionId: string) => Promise<void>
  teamWorkspaceActive: boolean
}): Promise<Set<string>> => {
  if (!params.normalizedSourceMessageId || !params.normalizedContent) {
    return params.mirroredAssistantSourceIds
  }

  let nextMirrored = params.mirroredAssistantSourceIds
  if (!nextMirrored.has(params.normalizedSourceMessageId)) {
    if (nextMirrored.size === 0) {
      nextMirrored = await params.collectPersistedSourceIds(params.sessionId)
    }
    if (nextMirrored.has(params.normalizedSourceMessageId)) {
      return nextMirrored
    }
  } else {
    return nextMirrored
  }

  await params.sendTeamMessage(params.sessionId, {
    thread_id: params.sessionId,
    from_agent_id: 'assistant',
    to_agent_id: null,
    message_type: 'assistant',
    payload: {
      content: params.normalizedContent,
      source_message_id: params.normalizedSourceMessageId,
    },
  })
  nextMirrored.add(params.normalizedSourceMessageId)
  await params.syncTeamMessagesToMainFlow(params.sessionId)
  if (params.teamWorkspaceActive && params.loadTeamWorkspaceData) {
    await params.loadTeamWorkspaceData()
  }
  return nextMirrored
}
