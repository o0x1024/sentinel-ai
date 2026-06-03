import type { TaskRuntimeItem, TaskRuntimeStatus } from './taskRuntime'

export type AgentTaskStatus = TaskRuntimeStatus | 'blocked' | 'failed' | 'cancelled'

export interface AgentTaskMetadata {
  parent_id?: string
  source_label?: string
  source_execution_id?: string
  tool_name?: string
  step_index?: number
  tags?: string[]
  reason?: string | null
}

export interface AgentTask {
  id: string
  title: string
  active_title?: string
  status: AgentTaskStatus
  created_at: number
  updated_at: number
  metadata?: AgentTaskMetadata
}

export const mapTaskRuntimeStatusToAgentTaskStatus = (status: TaskRuntimeStatus): AgentTaskStatus => status

export const mapTaskRuntimeItemToAgentTask = (task: TaskRuntimeItem, sourceExecutionId?: string): AgentTask => ({
  id: task.id,
  title: task.content,
  active_title: task.active_form,
  status: mapTaskRuntimeStatusToAgentTaskStatus(task.status),
  created_at: Number(task.created_at || 0),
  updated_at: Number(task.updated_at || 0),
  metadata: {
    parent_id: task.metadata?.parent_id,
    tool_name: task.metadata?.tool_name,
    step_index: task.metadata?.step_index,
    tags: task.metadata?.tags,
    reason: task.metadata?.error || null,
    source_execution_id: sourceExecutionId,
  },
})

export const mapTaskRuntimeItemsToAgentTasks = (tasks: TaskRuntimeItem[], sourceExecutionId?: string): AgentTask[] => {
  return tasks.map((task) => mapTaskRuntimeItemToAgentTask(task, sourceExecutionId))
}

export const getAgentTaskDisplayText = (task: AgentTask): string => {
  if (task.status === 'in_progress' && task.active_title) {
    return task.active_title
  }
  return task.title
}

export const getChildAgentTasks = (tasks: AgentTask[], parentId: string): AgentTask[] => {
  return tasks.filter((task) => task.metadata?.parent_id === parentId)
}

export const getRootAgentTasks = (tasks: AgentTask[]): AgentTask[] => {
  return tasks.filter((task) => !task.metadata?.parent_id)
}

export const calculateAgentTaskProgress = (tasks: AgentTask[]): number => {
  if (tasks.length === 0) return 0
  const completed = tasks.filter((task) => task.status === 'completed').length
  return Math.round((completed / tasks.length) * 100)
}
