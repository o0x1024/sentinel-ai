import { computed, type ComputedRef, type Ref } from 'vue'
import { useTaskRuntime, type UseTaskRuntimeReturn } from '@/composables/useTaskRuntime'
import type { AgentTask } from '@/types/agentTask'
import { mapTaskRuntimeItemsToAgentTasks } from '@/types/agentTask'
import type { TaskRuntimeItem } from '@/types/taskRuntime'

export interface AgentTaskStats {
  total: number
  pending: number
  in_progress: number
  completed: number
}

export interface UseAgentTasksReturn {
  tasks: ComputedRef<AgentTask[]>
  tasksByExecutionId: ComputedRef<Record<string, AgentTask[]>>
  executionIds: ComputedRef<string[]>
  lastExecutionId: ComputedRef<string | undefined>
  rootTasks: ComputedRef<AgentTask[]>
  stats: ComputedRef<AgentTaskStats>
  progress: ComputedRef<number>
  hasTasks: ComputedRef<boolean>
  hasHistory: ComputedRef<boolean>
  isTaskPanelActive: Ref<boolean>
  currentTask: ComputedRef<AgentTask | undefined>
  getTasksForExecution: (executionId: string) => AgentTask[]
  getChildren: (parentId: string) => AgentTask[]
  clearTasks: () => void
  clearTasksForExecution: (executionId: string) => void
  setTasksForExecution: (executionId: string, tasks: AgentTask[]) => void
  open: () => void
  close: () => void
  toggle: () => void
  startListening: () => Promise<void>
  stopListening: () => void
}

const buildTasksByExecutionId = (tasksByExecutionId: Record<string, ReturnType<UseTaskRuntimeReturn['getTasksForExecution']>>): Record<string, AgentTask[]> => {
  return Object.fromEntries(
    Object.entries(tasksByExecutionId).map(([executionId, tasks]) => [
      executionId,
      mapTaskRuntimeItemsToAgentTasks(tasks, executionId),
    ]),
  )
}

const mapAgentTaskToRuntimeTask = (task: AgentTask): TaskRuntimeItem => ({
  id: task.id,
  content: task.title,
  active_form: task.active_title,
  status: task.status === 'blocked' ? 'pending' : task.status,
  created_at: task.created_at,
  updated_at: task.updated_at,
  metadata: {
    parent_id: task.metadata?.parent_id,
    tool_name: task.metadata?.tool_name,
    step_index: task.metadata?.step_index,
    tags: task.metadata?.tags,
    error: task.metadata?.reason || undefined,
  },
})

export function useAgentTasks(executionId?: Ref<string> | string): UseAgentTasksReturn {
  const taskRuntime = useTaskRuntime(executionId)

  const tasks = computed(() => mapTaskRuntimeItemsToAgentTasks(taskRuntime.tasks.value, taskRuntime.lastExecutionId.value))
  const tasksByExecutionId = computed(() => buildTasksByExecutionId(taskRuntime.tasksByExecutionId.value))
  const rootTasks = computed(() => mapTaskRuntimeItemsToAgentTasks(taskRuntime.rootTasks.value, taskRuntime.lastExecutionId.value))
  const currentTask = computed(() => (
    taskRuntime.currentTask.value
      ? mapTaskRuntimeItemsToAgentTasks([taskRuntime.currentTask.value], taskRuntime.lastExecutionId.value)[0]
      : undefined
  ))

  return {
    tasks,
    tasksByExecutionId,
    executionIds: taskRuntime.executionIds,
    lastExecutionId: taskRuntime.lastExecutionId,
    rootTasks,
    stats: taskRuntime.stats,
    progress: taskRuntime.progress,
    hasTasks: taskRuntime.hasTasks,
    hasHistory: taskRuntime.hasHistory,
    isTaskPanelActive: taskRuntime.isTaskPanelActive,
    currentTask,
    getTasksForExecution: (targetExecutionId: string) => (
      mapTaskRuntimeItemsToAgentTasks(taskRuntime.getTasksForExecution(targetExecutionId), targetExecutionId)
    ),
    getChildren: (parentId: string) => (
      mapTaskRuntimeItemsToAgentTasks(taskRuntime.getChildren(parentId), taskRuntime.lastExecutionId.value)
    ),
    clearTasks: taskRuntime.clearTasks,
    clearTasksForExecution: taskRuntime.clearTasksForExecution,
    setTasksForExecution: (targetExecutionId: string, nextTasks: AgentTask[]) => {
      taskRuntime.setTasksForExecution(
        targetExecutionId,
        nextTasks.map((task) => mapAgentTaskToRuntimeTask(task)),
      )
    },
    open: taskRuntime.open,
    close: taskRuntime.close,
    toggle: taskRuntime.toggle,
    startListening: taskRuntime.startListening,
    stopListening: taskRuntime.stopListening,
  }
}

export function useGlobalAgentTasks(): UseAgentTasksReturn {
  return useAgentTasks()
}
