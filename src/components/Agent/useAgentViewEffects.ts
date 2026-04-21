import { watch, type Ref } from 'vue'
import type { PersistedAgentExecutionState } from './executionState'
import type { TeamTask } from '@/types/agentTeam'

export const useAgentViewEffects = (params: {
  conversationId: Ref<string | null>
  conversationExecutionState: Ref<PersistedAgentExecutionState | null>
  currentConversationTitle: Ref<string>
  getNewConversationTitle: () => string
  selectedTeamTaskId: Ref<string | null>
  setMirroredConversationMessageIds: (ids: Set<string>) => void
  syncActiveTeamSession: () => Promise<void>
  teamTasks: Ref<TeamTask[]>
  updateSessionTitle: (id: string, title: string) => void
}) => {
  watch(params.conversationId, async (newId) => {
    params.setMirroredConversationMessageIds(new Set())
    if (!newId) {
      params.currentConversationTitle.value = params.getNewConversationTitle()
      params.conversationExecutionState.value = null
    }
    await params.syncActiveTeamSession()
  })

  watch(params.teamTasks, (tasks) => {
    if (!params.selectedTeamTaskId.value) return
    if (tasks.some((task) => task.id === params.selectedTeamTaskId.value)) return
    params.selectedTeamTaskId.value = null
  }, { deep: true })

  watch(params.currentConversationTitle, (newTitle) => {
    if (params.conversationId.value && newTitle) {
      params.updateSessionTitle(params.conversationId.value, newTitle)
    }
  })
}
