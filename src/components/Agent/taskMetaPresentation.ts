import type { AgentTask } from '@/types/agentTask'
import type { TeamTask } from '@/types/agentTeam'

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

export const buildTeamTaskMetaItems = (
  task: TeamTask,
  params: {
    resolveAgentName: (agentId?: string | null) => string
    dependencyPreview: (task: TeamTask) => string
    dependencySatisfiedPreview: (task: TeamTask) => string
    dependencyBlockerPreview: (task: TeamTask) => string
  },
): TaskMetaItem[] => {
  const items: TaskMetaItem[] = []
  const ownerId = normalize(task.owner_agent_id)
  const claimedById = normalize(task.claimed_by_agent_id)
  const assigneeId = normalize(task.assignee_agent_id)
  const dependencyText = normalize(params.dependencyPreview(task))
  const satisfiedText = normalize(params.dependencySatisfiedPreview(task))
  const blockerText = normalize(params.dependencyBlockerPreview(task))
  const acceptance = normalize(task.acceptance_criteria)

  if (ownerId) {
    items.push({
      key: 'owner',
      labelKey: 'agent.teamTaskOwnerLabel',
      value: params.resolveAgentName(ownerId),
    })
  }

  if (claimedById && claimedById !== ownerId) {
    items.push({
      key: 'handler',
      labelKey: 'agent.teamTaskCurrentHandlerLabel',
      value: params.resolveAgentName(claimedById),
    })
  } else if (assigneeId) {
    items.push({
      key: 'handler',
      labelKey: 'agent.teamTaskCurrentHandlerLabel',
      value: params.resolveAgentName(assigneeId),
    })
  }

  items.push({
    key: 'dependencies',
    labelKey: 'agent.teamTaskDependencyLabel',
    value: dependencyText,
  })

  if (satisfiedText) {
    items.push({
      key: 'dependencies_satisfied',
      labelKey: 'agent.teamTaskSatisfiedDependencyLabel',
      value: satisfiedText,
      tone: 'success',
    })
  }

  if (blockerText) {
    items.push({
      key: 'dependencies_blocked',
      labelKey: 'agent.teamTaskBlockingDependencyLabel',
      value: blockerText,
      tone: 'warning',
    })
  }

  if (acceptance) {
    items.push({
      key: 'acceptance',
      labelKey: 'agent.teamTaskAcceptanceCriteriaLabel',
      value: acceptance,
    })
  }

  items.push({
    key: 'attempts',
    labelKey: 'agent.teamTaskAttemptsLabel',
    value: `${task.attempt}/${task.max_attempts}`,
  })

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
