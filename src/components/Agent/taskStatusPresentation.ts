import type { AgentTaskStatus } from '@/types/agentTask'

export type TaskStatusTone =
  | 'neutral'
  | 'active'
  | 'accent'
  | 'success'
  | 'warning'
  | 'error'
  | 'muted'

export const getAgentTaskStatusTone = (status: AgentTaskStatus): TaskStatusTone => {
  switch (status) {
    case 'in_progress':
      return 'active'
    case 'completed':
      return 'success'
    case 'failed':
      return 'error'
    case 'blocked':
      return 'warning'
    case 'cancelled':
      return 'muted'
    default:
      return 'neutral'
  }
}

export const getTeamTaskStatusTone = (status: string): TaskStatusTone => {
  switch (status) {
    case 'completed':
      return 'success'
    case 'running':
      return 'active'
    case 'ready_for_claim':
      return 'accent'
    case 'claimed':
    case 'waiting_review':
    case 'waiting_handoff_ack':
      return 'accent'
    case 'failed':
      return 'error'
    case 'blocked':
      return 'warning'
    case 'cancelled':
      return 'muted'
    default:
      return 'neutral'
  }
}

export const getTaskToneBadgeClass = (tone: TaskStatusTone): string => {
  switch (tone) {
    case 'success':
      return 'badge-success'
    case 'active':
      return 'badge-info'
    case 'accent':
      return 'badge-secondary'
    case 'error':
      return 'badge-error'
    case 'warning':
      return 'badge-warning'
    case 'muted':
      return 'badge-ghost opacity-70'
    case 'neutral':
    default:
      return 'badge-ghost'
  }
}

export const getTaskToneIndicatorClass = (tone: TaskStatusTone): string => {
  switch (tone) {
    case 'active':
      return 'text-primary'
    case 'success':
      return 'text-success'
    case 'error':
      return 'text-error'
    case 'warning':
      return 'text-warning'
    case 'accent':
      return 'text-secondary'
    case 'muted':
      return 'text-base-content/50'
    case 'neutral':
    default:
      return 'text-base-content/60'
  }
}

export const getTaskToneContentClass = (
  tone: TaskStatusTone,
  options?: { struckThrough?: boolean },
): string => {
  const suffix = options?.struckThrough ? ' line-through' : ''
  switch (tone) {
    case 'success':
      return `text-base-content/60${suffix}`
    case 'error':
      return `text-error${suffix}`
    case 'warning':
      return `text-warning${suffix}`
    case 'muted':
      return `text-base-content/50${suffix}`
    case 'accent':
      return `text-secondary${suffix}`
    case 'active':
    case 'neutral':
    default:
      return `text-base-content${suffix}`
  }
}

export const getTaskToneRowClass = (tone: TaskStatusTone): string => {
  switch (tone) {
    case 'active':
      return 'text-primary'
    case 'success':
      return 'text-base-content/70'
    case 'error':
      return 'text-error'
    case 'warning':
      return 'text-warning'
    case 'accent':
      return 'text-secondary'
    case 'muted':
      return 'text-base-content/50'
    case 'neutral':
    default:
      return 'text-base-content/80'
  }
}
