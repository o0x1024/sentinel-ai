/**
 * Runtime task state composable.
 * Listens for backend task updates and maintains shared reactive state.
 */

import { ref, computed, onMounted, onUnmounted, type Ref, type ComputedRef } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useTerminal } from '@/composables/useTerminal'
import type {
  AgentTasksUpdatePayload,
  TaskRuntimeItem,
  TaskRuntimeStats,
  TaskRuntimeStatus,
} from '@/types/taskRuntime'
import {
  calculateTaskRuntimeProgress,
  getChildTaskRuntimeItems,
  getRootTaskRuntimeItems,
  getTaskRuntimeIndicator,
} from '@/types/taskRuntime'

export interface UseTaskRuntimeReturn {
  tasks: ComputedRef<TaskRuntimeItem[]>
  tasksByExecutionId: ComputedRef<Record<string, TaskRuntimeItem[]>>
  executionIds: ComputedRef<string[]>
  lastExecutionId: ComputedRef<string | undefined>
  rootTasks: ComputedRef<TaskRuntimeItem[]>
  stats: ComputedRef<TaskRuntimeStats>
  progress: ComputedRef<number>
  hasTasks: ComputedRef<boolean>
  hasHistory: ComputedRef<boolean>
  isTaskPanelActive: Ref<boolean>
  currentTask: ComputedRef<TaskRuntimeItem | undefined>
  getTasksForExecution: (executionId: string) => TaskRuntimeItem[]
  getChildren: (parentId: string) => TaskRuntimeItem[]
  getIndicator: (status: TaskRuntimeStatus) => string
  clearTasks: () => void
  clearTasksForExecution: (executionId: string) => void
  setTasksForExecution: (executionId: string, tasks: TaskRuntimeItem[]) => void
  open: () => void
  close: () => void
  toggle: () => void
  startListening: () => Promise<void>
  stopListening: () => void
}

type AgentExecutionOutcome = 'succeeded' | 'failed' | 'cancelled'

interface AgentExecutionFinishedEvent {
  execution_id: string
  outcome: AgentExecutionOutcome
  success: boolean
  error?: string | null
  response?: string | null
  message?: string | null
}

const globalTaskRuntimeState = ref<{
  tasksByExecutionId: Record<string, TaskRuntimeItem[]>
  isTaskPanelActive: boolean
  lastExecutionId: string | undefined
}>({
  tasksByExecutionId: {},
  isTaskPanelActive: false,
  lastExecutionId: undefined,
})

let globalUnlisten: UnlistenFn | null = null
let listenerCount = 0

export function useTaskRuntime(executionId?: Ref<string> | string): UseTaskRuntimeReturn {
  const getExecutionId = (): string | undefined => {
    if (!executionId) return undefined
    return typeof executionId === 'string' ? executionId : executionId.value
  }

  const tasks = computed<TaskRuntimeItem[]>(() => {
    const targetId = getExecutionId()
    if (targetId) return globalTaskRuntimeState.value.tasksByExecutionId[targetId] || []
    const latestId = globalTaskRuntimeState.value.lastExecutionId
    if (latestId) return globalTaskRuntimeState.value.tasksByExecutionId[latestId] || []
    return []
  })

  const tasksByExecutionId = computed(() => globalTaskRuntimeState.value.tasksByExecutionId)
  const executionIds = computed(() => Object.keys(globalTaskRuntimeState.value.tasksByExecutionId))
  const lastExecutionId = computed(() => globalTaskRuntimeState.value.lastExecutionId)
  const rootTasks = computed(() => getRootTaskRuntimeItems(tasks.value))

  const stats = computed<TaskRuntimeStats>(() => ({
    total: tasks.value.length,
    pending: tasks.value.filter((task) => task.status === 'pending').length,
    in_progress: tasks.value.filter((task) => task.status === 'in_progress').length,
    completed: tasks.value.filter((task) => task.status === 'completed').length,
  }))

  const progress = computed(() => calculateTaskRuntimeProgress(tasks.value))
  const hasTasks = computed(() => tasks.value.length > 0)
  const hasHistory = computed(() => {
    return Object.values(globalTaskRuntimeState.value.tasksByExecutionId).some((items) => items.length > 0)
  })
  const currentTask = computed(() => tasks.value.find((task) => task.status === 'in_progress'))

  const getTasksForExecution = (id: string): TaskRuntimeItem[] => {
    if (!id) return []
    return globalTaskRuntimeState.value.tasksByExecutionId[id] || []
  }

  const getChildren = (parentId: string): TaskRuntimeItem[] => {
    return getChildTaskRuntimeItems(tasks.value, parentId)
  }

  const getIndicator = (status: TaskRuntimeStatus): string => {
    return getTaskRuntimeIndicator(status)
  }

  const clearTasks = (): void => {
    const targetId = getExecutionId()
    if (targetId) {
      delete globalTaskRuntimeState.value.tasksByExecutionId[targetId]
      if (globalTaskRuntimeState.value.lastExecutionId === targetId) {
        globalTaskRuntimeState.value.lastExecutionId = undefined
      }
      return
    }
    globalTaskRuntimeState.value.tasksByExecutionId = {}
    globalTaskRuntimeState.value.lastExecutionId = undefined
  }

  const clearTasksForExecution = (id: string): void => {
    if (!id) return
    if (!(id in globalTaskRuntimeState.value.tasksByExecutionId)) return
    delete globalTaskRuntimeState.value.tasksByExecutionId[id]
    if (globalTaskRuntimeState.value.lastExecutionId === id) {
      globalTaskRuntimeState.value.lastExecutionId = undefined
    }
  }

  const setTasksForExecution = (id: string, items: TaskRuntimeItem[]): void => {
    if (!id) return
    globalTaskRuntimeState.value.tasksByExecutionId = {
      ...globalTaskRuntimeState.value.tasksByExecutionId,
      [id]: items,
    }
    globalTaskRuntimeState.value.lastExecutionId = id
  }

  const open = (): void => {
    globalTaskRuntimeState.value.isTaskPanelActive = true
  }

  const close = (): void => {
    globalTaskRuntimeState.value.isTaskPanelActive = false
  }

  const toggle = (): void => {
    globalTaskRuntimeState.value.isTaskPanelActive = !globalTaskRuntimeState.value.isTaskPanelActive
  }

  const startListening = async (): Promise<void> => {
    listenerCount += 1
    if (globalUnlisten) return

    const unlistenTasks = await listen<AgentTasksUpdatePayload>('agent-tasks-update', (event) => {
      const targetId = getExecutionId()
      if (targetId && event.payload.execution_id !== targetId) {
        return
      }

      globalTaskRuntimeState.value.tasksByExecutionId = {
        ...globalTaskRuntimeState.value.tasksByExecutionId,
        [event.payload.execution_id]: event.payload.tasks,
      }
      globalTaskRuntimeState.value.lastExecutionId = event.payload.execution_id

      if (event.payload.tasks.length > 0) {
        globalTaskRuntimeState.value.isTaskPanelActive = true
        useTerminal().closeTerminal()
      }
    })

    const unlistenFinished = await listen<AgentExecutionFinishedEvent>('agent:execution_finished', (event) => {
      const targetId = getExecutionId()
      if (targetId && event.payload.execution_id !== targetId) {
        return
      }
    })

    globalUnlisten = () => {
      unlistenTasks()
      unlistenFinished()
    }
  }

  const stopListening = (): void => {
    listenerCount -= 1
    if (listenerCount <= 0 && globalUnlisten) {
      globalUnlisten()
      globalUnlisten = null
      listenerCount = 0
    }
  }

  onMounted(() => {
    void startListening()
  })

  onUnmounted(() => {
    stopListening()
  })

  return {
    tasks,
    tasksByExecutionId,
    executionIds,
    lastExecutionId,
    rootTasks,
    stats,
    progress,
    hasTasks,
    hasHistory,
    isTaskPanelActive: computed(() => globalTaskRuntimeState.value.isTaskPanelActive),
    currentTask,
    getTasksForExecution,
    getChildren,
    getIndicator,
    clearTasks,
    clearTasksForExecution,
    setTasksForExecution,
    open,
    close,
    toggle,
    startListening,
    stopListening,
  }
}

export function useGlobalTaskRuntime(): UseTaskRuntimeReturn {
  return useTaskRuntime()
}
