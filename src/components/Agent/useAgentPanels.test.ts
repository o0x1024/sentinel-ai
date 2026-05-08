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
    getConversationIdForExecution: () => undefined,
    getTasksForExecution: () => [],
    isTeamWorkspaceActive,
    isTaskPanelActive: computed(() => isTaskPanelActive.value),
    localError: ref(null),
    parseTeamTaskExecutionId: () => null,
    propsShowTasks: true,
    parallelTaskSources: computed(() => []),
    pruneTasksForExecutionAfter: async () => [],
    resetAgentError: () => {},
    resolveAgentName: () => 'Agent',
    selectedTeamTaskAssigneeId: computed(() => null),
    setTasksForExecution: () => {},
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

  it('prunes only current-context tasks after the replay boundary', async () => {
    const calls: Array<{ executionId: string; timestampMs: number }> = []
    const remainingByExecution: Record<string, any[]> = {
      'conversation-1': [
        { id: 'conversation-1_0', title: 'kept', status: 'completed', created_at: 100, updated_at: 100 },
      ],
      'parallel-1': [],
    }
    const applied: Record<string, any[]> = {}
    const cleared: string[] = []

    const controller = useAgentPanels({
      activeTeamSessionId: ref(null),
      agentError: computed(() => null),
      clearTasksForExecution: (executionId) => {
        cleared.push(executionId)
      },
      conversationId: ref('conversation-1'),
      getConversationIdForExecution: () => undefined,
      getTasksForExecution: () => [],
      isTeamWorkspaceActive: ref(false),
      isTaskPanelActive: computed(() => false),
      localError: ref(null),
      parseTeamTaskExecutionId: () => null,
      parallelTaskSources: computed(() => [
        { executionId: 'parallel-1', parentConversationId: 'conversation-1', label: 'parallel', task: 'task', status: 'running' },
      ]),
      propsShowTasks: true,
      pruneTasksForExecutionAfter: async (executionId, timestampMs) => {
        calls.push({ executionId, timestampMs })
        return remainingByExecution[executionId] || []
      },
      resetAgentError: () => {},
      resolveAgentName: () => 'Agent',
      selectedTeamTaskAssigneeId: computed(() => null),
      setTasksForExecution: (executionId, tasks) => {
        applied[executionId] = tasks
      },
      teamWorkspaceAvailable: computed(() => false),
      terminalClose: () => {},
      terminalHasHistory: computed(() => false),
      terminalIsActive: computed(() => false),
      terminalOpen: () => {},
      tasksByExecutionId: computed(() => ({})),
      taskExecutionIds: computed(() => []),
      tasksClose: () => {},
      tasksOpen: () => {},
    })

    await controller.pruneTasksForCurrentContextAfter(500)

    expect(calls).toEqual([
      { executionId: 'conversation-1', timestampMs: 500 },
      { executionId: 'parallel-1', timestampMs: 500 },
    ])
    expect(applied['conversation-1']).toEqual(remainingByExecution['conversation-1'])
    expect(cleared).toEqual(['parallel-1'])
  })

  it('shows execution task buckets that belong to the current conversation', () => {
    const controller = useAgentPanels({
      activeTeamSessionId: ref(null),
      agentError: computed(() => null),
      clearTasksForExecution: () => {},
      conversationId: ref('conversation-1'),
      getConversationIdForExecution: (executionId) => (
        executionId === 'execution-1' ? 'conversation-1' : undefined
      ),
      getTasksForExecution: () => [],
      isTeamWorkspaceActive: ref(false),
      isTaskPanelActive: computed(() => false),
      localError: ref(null),
      parseTeamTaskExecutionId: () => null,
      parallelTaskSources: computed(() => []),
      propsShowTasks: true,
      pruneTasksForExecutionAfter: async () => [],
      resetAgentError: () => {},
      resolveAgentName: () => 'Agent',
      selectedTeamTaskAssigneeId: computed(() => null),
      setTasksForExecution: () => {},
      teamWorkspaceAvailable: computed(() => false),
      terminalClose: () => {},
      terminalHasHistory: computed(() => false),
      terminalIsActive: computed(() => false),
      terminalOpen: () => {},
      tasksByExecutionId: computed(() => ({
        'execution-1': [
          { id: 'execution-1_0', title: 'mapped task', status: 'pending', created_at: 100, updated_at: 100 },
        ],
        'other-execution': [
          { id: 'other_0', title: 'other task', status: 'pending', created_at: 200, updated_at: 200 },
        ],
      })),
      taskExecutionIds: computed(() => ['execution-1', 'other-execution']),
      tasksClose: () => {},
      tasksOpen: () => {},
    })

    expect(controller.tasks.value.map((task) => task.title)).toEqual(['mapped task'])
    expect(controller.taskBadgeCount.value).toBe(1)
  })
})
