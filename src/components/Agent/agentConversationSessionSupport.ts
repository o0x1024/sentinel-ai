import type { AssistantConversationBinding } from './agentDraftTypes'
import type { AiConversationSummary } from './conversationTypes'

type CreateConversationRequest = {
  service_name: string
  title: string
  conversation_binding?: AssistantConversationBinding | null
}

export const pickLatestConversation = (
  conversations: AiConversationSummary[],
): AiConversationSummary | null => {
  if (!Array.isArray(conversations) || conversations.length === 0) {
    return null
  }

  const sorted = [...conversations].sort((a, b) => (
    new Date(b.created_at || 0).getTime() - new Date(a.created_at || 0).getTime()
    || new Date(b.updated_at || 0).getTime() - new Date(a.updated_at || 0).getTime()
  ))
  return sorted[0] || null
}

export const createConversationSession = async (params: {
  conversationBinding?: AssistantConversationBinding | null
  createConversation: (request: CreateConversationRequest) => Promise<string>
  getConversationTitle: () => string
  getDisplayTitle: () => string
  loadConversationList?: () => void
  onConversationCreated: (conversationId: string, title: string) => void
}): Promise<string> => {
  const conversationId = await params.createConversation({
    conversation_binding: params.conversationBinding,
    service_name: 'default',
    title: params.getConversationTitle(),
  })
  params.onConversationCreated(conversationId, params.getDisplayTitle())
  params.loadConversationList?.()
  return conversationId
}

export const selectConversationSession = async (params: {
  closeConversationDrawer: () => void
  conversationId: string
  isStillActive: (conversationId: string) => boolean
  loadConversationHistory: (conversationId: string) => Promise<void>
  onConversationSelected: (conversationId: string) => void
  resetTerminal: () => void
}): Promise<void> => {
  params.onConversationSelected(params.conversationId)
  await params.loadConversationHistory(params.conversationId)
  if (!params.isStillActive(params.conversationId)) {
    return
  }

  params.resetTerminal()
  if (!params.isStillActive(params.conversationId)) {
    return
  }

  params.closeConversationDrawer()
}

export const clearConversationSession = async (params: {
  clearConversationMessages: (conversationId: string) => Promise<void>
  conversationId?: string | null
  loadConversationList?: () => void
  onConversationCleared: () => void
}): Promise<boolean> => {
  if (!params.conversationId) {
    return false
  }

  await params.clearConversationMessages(params.conversationId)
  params.onConversationCleared()
  params.loadConversationList?.()
  return true
}

export const loadLatestConversationSession = async (params: {
  currentConversationId?: string | null
  getConversations: () => Promise<AiConversationSummary[]>
  loadConversationHistory: (conversationId: string) => Promise<void>
  onConversationLoaded: (conversation: AiConversationSummary) => void
}): Promise<AiConversationSummary | null> => {
  if (params.currentConversationId) {
    return null
  }

  const conversations = await params.getConversations()
  if (params.currentConversationId) {
    return null
  }

  const latest = pickLatestConversation(conversations)
  if (!latest || params.currentConversationId) {
    return null
  }

  params.onConversationLoaded(latest)
  await params.loadConversationHistory(latest.id)
  return latest
}
