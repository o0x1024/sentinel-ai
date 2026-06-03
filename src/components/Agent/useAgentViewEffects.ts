import { watch, type Ref } from 'vue'
import type { PersistedAgentExecutionState } from './executionState'

export const useAgentViewEffects = (params: {
  conversationId: Ref<string | null>
  conversationExecutionState: Ref<PersistedAgentExecutionState | null>
  currentConversationTitle: Ref<string>
  getNewConversationTitle: () => string
  updateSessionTitle: (id: string, title: string) => void
}) => {
  watch(params.conversationId, (newId) => {
    if (!newId) {
      params.currentConversationTitle.value = params.getNewConversationTitle()
      params.conversationExecutionState.value = null
    }
  })

  watch(params.currentConversationTitle, (newTitle) => {
    if (params.conversationId.value && newTitle) {
      params.updateSessionTitle(params.conversationId.value, newTitle)
    }
  })
}
