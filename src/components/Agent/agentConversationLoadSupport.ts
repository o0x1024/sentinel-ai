import type { AgentMessage } from '@/types/agent'
import {
  buildConversationTimeline,
  type PersistedConversationMessageRow,
} from './agentConversationHistorySupport'
import type { AiConversationDetail } from './conversationTypes'

export const loadConversationHistory = async (params: {
  buildToolCallCompletedLabel: () => string
  clearMessages: () => void
  conversationId: string
  getConversation: (conversationId: string) => Promise<AiConversationDetail | null>
  getMessages: (conversationId: string) => Promise<PersistedConversationMessageRow[]>
  isStale: () => boolean
  loadSubagentRuns: () => Promise<void>
  log: (message: string, ...args: unknown[]) => void
  onConversationLoaded: (payload: {
    executionState: AiConversationDetail['execution_state']
    title: string
  }) => void
  onEmptyHistoryLoaded: () => Promise<void>
  onMessagesLoaded: (payload: {
    messageCount: number
    timeline: AgentMessage[]
  }) => Promise<void>
  onLoadFailed: (error: unknown) => void
  unnamedConversationTitle: string
}): Promise<void> => {
  params.log('[AgentView] Loading conversation history for:', params.conversationId)

  try {
    const [conversationDetail, messages] = await Promise.all([
      params.getConversation(params.conversationId),
      params.getMessages(params.conversationId),
    ])
    if (params.isStale()) {
      params.log('[AgentView] Skip stale conversation history load for:', params.conversationId)
      return
    }

    params.onConversationLoaded({
      executionState: conversationDetail?.execution_state || null,
      title: conversationDetail?.title || params.unnamedConversationTitle,
    })
    params.log('[AgentView] Received messages:', messages)

    params.clearMessages()
    await params.loadSubagentRuns()
    if (params.isStale()) {
      params.log('[AgentView] Skip stale conversation history apply for:', params.conversationId)
      return
    }

    if (messages && messages.length > 0) {
      const timeline = buildConversationTimeline(messages, {
        toolCallCompletedLabel: params.buildToolCallCompletedLabel(),
      })

      if (params.isStale()) {
        params.log('[AgentView] Skip stale conversation timeline apply for:', params.conversationId)
        return
      }

      await params.onMessagesLoaded({
        messageCount: messages.length,
        timeline,
      })
      return
    }

    await params.onEmptyHistoryLoaded()
  } catch (e) {
    params.onLoadFailed(e)
  }
}
