import { computed, nextTick, ref } from 'vue'
import { describe, expect, it } from 'vitest'
import { useAgentPanels } from './useAgentPanels'

const createPanelController = () => {
  const activeTeamSessionId = ref<string | null>(null)
  const isTaskPanelActive = ref(false)
  const isTeamWorkspaceActive = ref(false)
  const terminalIsActive = ref(false)
  const terminalHasHistory = ref(false)
  let tasksCloseCount = 0
  let terminalCloseCount = 0

  const controller = useAgentPanels({
    activeTeamSessionId,
    agentError: computed(() => null),
    clearTasksForExecution: () => {},
    conversationId: ref('conversation-1'),
    getTasksForExecution: () => [],
    isTeamWorkspaceActive,
    isTaskPanelActive: computed(() => isTaskPanelActive.value),
    localError: ref(null),
    parseTeamTaskExecutionId: () => null,
    propsShowTasks: true,
    parallelTaskSources: computed(() => []),
    resetAgentError: () => {},
    resolveAgentName: () => 'Agent',
    selectedTeamTaskAssigneeId: computed(() => null),
    teamWorkspaceAvailable: computed(() => false),
    terminalClose: () => {
      terminalCloseCount += 1
      terminalIsActive.value = false
    },
    terminalHasHistory: computed(() => terminalHasHistory.value),
    terminalIsActive: computed(() => terminalIsActive.value),
    terminalOpen: () => {
      terminalIsActive.value = true
    },
    tasksByExecutionId: computed(() => ({})),
    taskExecutionIds: computed(() => []),
    tasksClose: () => {
      tasksCloseCount += 1
      isTaskPanelActive.value = false
    },
    tasksOpen: () => {
      isTaskPanelActive.value = true
    },
  })

  return {
    controller,
    counts: {
      get tasksClose() {
        return tasksCloseCount
      },
      get terminalClose() {
        return terminalCloseCount
      },
    },
    state: {
      isTaskPanelActive,
      terminalIsActive,
    },
  }
}

describe('useAgentPanels', () => {
  it('closes the previous panel when syncing a newly active terminal panel', async () => {
    const { controller, counts, state } = createPanelController()

    state.isTaskPanelActive.value = true
    await nextTick()
    expect(controller.activeRightPanel.value).toBe('tasks')
    const tasksCloseBeforeTerminalSync = counts.tasksClose
    const terminalCloseBeforeTerminalSync = counts.terminalClose

    state.terminalIsActive.value = true
    await nextTick()

    expect(controller.activeRightPanel.value).toBe('terminal')
    expect(counts.tasksClose).toBe(tasksCloseBeforeTerminalSync + 1)
    expect(counts.terminalClose).toBe(terminalCloseBeforeTerminalSync)
  })
})
