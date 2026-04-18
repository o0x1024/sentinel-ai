import type { AgentMessage } from '@/types/agent'

export interface PersistAgentTaskMessageRequest {
  id: string
  conversation_id: string
  role: string
  content: string
  metadata: Record<string, unknown>
}

export const buildPersistedAgentTaskMessageRequest = (params: {
  conversationId?: string | null
  message: AgentMessage
}): PersistAgentTaskMessageRequest | null => {
  const conversationId = String(params.conversationId || '').trim()
  if (!conversationId) return null
  if (params.message.type !== 'system') return null
  if (params.message.metadata?.kind !== 'agent_task_update') return null

  return {
    id: params.message.id,
    conversation_id: conversationId,
    role: 'system',
    content: params.message.content,
    metadata: {
      ...(params.message.metadata || {}),
      source: 'agent_tasks_update',
    },
  }
}

export const persistAgentTaskMessage = async (params: {
  conversationId?: string | null
  message: AgentMessage
  persistedIds: Set<string>
  persistMessage: (request: PersistAgentTaskMessageRequest) => Promise<void>
}): Promise<boolean> => {
  const request = buildPersistedAgentTaskMessageRequest({
    conversationId: params.conversationId,
    message: params.message,
  })
  if (!request) return false
  if (params.persistedIds.has(request.id)) return false

  try {
    await params.persistMessage(request)
    params.persistedIds.add(request.id)
    return true
  } catch (error: any) {
    const message = String(error || '')
    if (
      message.includes('UNIQUE constraint failed') ||
      message.toLowerCase().includes('duplicate key') ||
      message.toLowerCase().includes('already exists')
    ) {
      params.persistedIds.add(request.id)
      return false
    }
    throw error
  }
}
