/**
 * Runtime task state definitions for single-agent execution tracking.
 */

export type TaskRuntimeStatus =
  | 'pending'
  | 'in_progress'
  | 'completed'
  | 'failed'
  | 'cancelled'

export interface TaskRuntimeMetadata {
  tool_name?: string
  step_index?: number
  parent_id?: string
  tags?: string[]
  error?: string
}

export interface TaskRuntimeItem {
  id: string
  content: string
  active_form?: string
  status: TaskRuntimeStatus
  created_at: number
  updated_at: number
  metadata?: TaskRuntimeMetadata
}

export interface TaskWriteInput {
  tasks: TaskWriteItem[]
}

export interface TaskWriteItem {
  content: string
  status: TaskRuntimeStatus
  activeForm: string
}

export interface TaskRuntimeList {
  execution_id: string
  tasks: TaskRuntimeItem[]
  created_at: number
  updated_at: number
}

export interface AgentTasksUpdatePayload {
  execution_id: string
  tasks: TaskRuntimeItem[]
  timestamp: number
}

export interface TaskRuntimeStats {
  total: number
  pending: number
  in_progress: number
  completed: number
}

export const TaskRuntimeIndicators: Record<TaskRuntimeStatus, string> = {
  pending: '○',
  in_progress: '→',
  completed: '✓',
  failed: '!',
  cancelled: '×',
}

export const TaskRuntimeStatusColors: Record<TaskRuntimeStatus, string> = {
  pending: 'text-base-content/60',
  in_progress: 'text-primary',
  completed: 'text-success',
  failed: 'text-error',
  cancelled: 'text-base-content/50',
}

export function getTaskRuntimeDisplayText(task: TaskRuntimeItem): string {
  if (task.status === 'in_progress' && task.active_form) {
    return task.active_form
  }
  return task.content
}

export function getTaskRuntimeIndicator(status: TaskRuntimeStatus): string {
  return TaskRuntimeIndicators[status]
}

export function isSubTaskRuntimeItem(task: TaskRuntimeItem): boolean {
  return !!task.metadata?.parent_id
}

export function getChildTaskRuntimeItems(tasks: TaskRuntimeItem[], parentId: string): TaskRuntimeItem[] {
  return tasks.filter((task) => task.metadata?.parent_id === parentId)
}

export function getRootTaskRuntimeItems(tasks: TaskRuntimeItem[]): TaskRuntimeItem[] {
  return tasks.filter((task) => !task.metadata?.parent_id)
}

export function calculateTaskRuntimeProgress(tasks: TaskRuntimeItem[]): number {
  if (tasks.length === 0) return 0
  const completed = tasks.filter((task) => task.status === 'completed').length
  return Math.round((completed / tasks.length) * 100)
}

export function calculateTaskRuntimeChildrenProgress(tasks: TaskRuntimeItem[], parentId: string): { completed: number; total: number } {
  const children = getChildTaskRuntimeItems(tasks, parentId)
  return {
    completed: children.filter((child) => child.status === 'completed').length,
    total: children.length,
  }
}
