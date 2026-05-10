import type { AgentTask } from '@/types/agentTask'

export type TaskMetaTone = 'default' | 'success' | 'warning' | 'error'

export interface TaskMetaItem {
  key: string
  labelKey: string
  value: string
  tone?: TaskMetaTone
}

const normalize = (value: unknown): string => String(value || '').trim()

export const buildAgentTaskMetaItems = (
  task: AgentTask,
): TaskMetaItem[] => {
  const items: TaskMetaItem[] = []
  const source = normalize(task.metadata?.source_label)
  const reason = normalize(task.metadata?.reason)

  if (source) {
    items.push({
      key: 'source',
      labelKey: 'agent.taskMetaSourceLabel',
      value: source,
    })
  }

  if (reason) {
    items.push({
      key: 'reason',
      labelKey: 'agent.taskMetaReasonLabel',
      value: reason,
      tone: task.status === 'failed' ? 'error' : task.status === 'blocked' ? 'warning' : 'default',
    })
  }

  return items
}

export const taskMetaToneClass = (tone?: TaskMetaTone): string => {
  switch (tone) {
    case 'success':
      return 'text-success'
    case 'warning':
      return 'text-warning'
    case 'error':
      return 'text-error'
    default:
      return 'text-base-content/60'
  }
}
