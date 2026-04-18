import type { TeamTask } from '@/types/agentTeam'
import { getTaskToneBadgeClass, getTeamTaskStatusTone } from './taskStatusPresentation'

export type TeamTaskStatusKey =
  | 'pending'
  | 'ready_for_claim'
  | 'claimed'
  | 'running'
  | 'blocked'
  | 'waiting_review'
  | 'waiting_handoff_ack'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'unknown'

export type TeamTaskSectionKey =
  | 'executable'
  | 'active'
  | 'blocked'
  | 'review'
  | 'completed'
  | 'attention'

export const normalizeTeamTaskStatus = (value: unknown): TeamTaskStatusKey => {
  const normalized = String(value || '').trim().toLowerCase()
  if (!normalized) return 'unknown'
  if (normalized === 'pending') return 'pending'
  if (normalized === 'ready_for_claim') return 'ready_for_claim'
  if (normalized === 'claimed') return 'claimed'
  if (normalized === 'running' || normalized === 'in_progress') return 'running'
  if (normalized === 'blocked') return 'blocked'
  if (normalized === 'waiting_review') return 'waiting_review'
  if (normalized === 'waiting_handoff_ack') return 'waiting_handoff_ack'
  if (normalized === 'completed') return 'completed'
  if (normalized === 'failed') return 'failed'
  if (normalized === 'cancelled') return 'cancelled'
  return 'unknown'
}

export const teamTaskStatusBadgeClass = (status: unknown): string => {
  return getTaskToneBadgeClass(getTeamTaskStatusTone(normalizeTeamTaskStatus(status)))
}

export const teamTaskStatusI18nKey = (status: unknown): string => {
  switch (normalizeTeamTaskStatus(status)) {
    case 'pending':
      return 'agent.teamTaskStatus.pending'
    case 'ready_for_claim':
      return 'agent.teamTaskStatus.readyForClaim'
    case 'claimed':
      return 'agent.teamTaskStatus.claimed'
    case 'running':
      return 'agent.teamTaskStatus.running'
    case 'blocked':
      return 'agent.teamTaskStatus.blocked'
    case 'waiting_review':
      return 'agent.teamTaskStatus.waitingReview'
    case 'waiting_handoff_ack':
      return 'agent.teamTaskStatus.waitingHandoffAck'
    case 'completed':
      return 'agent.teamTaskStatus.completed'
    case 'failed':
      return 'agent.teamTaskStatus.failed'
    case 'cancelled':
      return 'agent.teamTaskStatus.cancelled'
    default:
      return 'agent.teamTaskStatus.unknown'
  }
}

export const teamWorkspaceAgentStatusI18nKey = (status: unknown): string => {
  const normalized = String(status || '').trim().toLowerCase()
  switch (normalized) {
    case 'running':
      return 'agent.teamAgentStatus.running'
    case 'failed':
      return 'agent.teamAgentStatus.failed'
    case 'blocked':
      return 'agent.teamAgentStatus.blocked'
    case 'completed':
      return 'agent.teamAgentStatus.completed'
    case 'pending':
      return 'agent.teamAgentStatus.pending'
    default:
      return 'agent.teamAgentStatus.idle'
  }
}

export const teamBlackboardEntryTypeI18nKey = (entryType: unknown): string => {
  const normalized = String(entryType || '').trim().toLowerCase()
  switch (normalized) {
    case 'task_output':
      return 'agent.teamBlackboardEntryType.taskOutput'
    case 'artifact_ref':
      return 'agent.teamBlackboardEntryType.artifactRef'
    case 'task_error':
      return 'agent.teamBlackboardEntryType.taskError'
    case 'task_start':
      return 'agent.teamBlackboardEntryType.taskStart'
    case 'plan':
      return 'agent.teamBlackboardEntryType.plan'
    case 'plan_fallback':
      return 'agent.teamBlackboardEntryType.planFallback'
    case 'goal':
      return 'agent.teamBlackboardEntryType.goal'
    default:
      return 'agent.teamBlackboardEntryType.generic'
  }
}

export const resolveTeamTaskDependencyTitles = (
  task: TeamTask,
  tasks: TeamTask[],
): string[] => {
  if (!Array.isArray(task.depends_on) || task.depends_on.length === 0) return []
  const titleByTaskId = new Map<string, string>()
  const titleByRecordId = new Map<string, string>()

  for (const item of tasks) {
    const title = String(item.title || item.task_id || item.id || '').trim()
    if (!title) continue
    const taskId = String(item.task_id || '').trim()
    const recordId = String(item.id || '').trim()
    if (taskId) titleByTaskId.set(taskId, title)
    if (recordId) titleByRecordId.set(recordId, title)
  }

  return task.depends_on.map((dependency) => {
    const key = String(dependency || '').trim()
    return titleByTaskId.get(key) || titleByRecordId.get(key) || key
  })
}

export const resolveUnresolvedTeamTaskDependencies = (
  task: TeamTask,
  tasks: TeamTask[],
): string[] => {
  if (!Array.isArray(task.depends_on) || task.depends_on.length === 0) return []

  const taskByKey = new Map<string, TeamTask>()
  const taskByRecordId = new Map<string, TeamTask>()

  for (const item of tasks) {
    const taskKey = String(item.task_id || '').trim()
    const recordId = String(item.id || '').trim()
    if (taskKey) taskByKey.set(taskKey, item)
    if (recordId) taskByRecordId.set(recordId, item)
  }

  return task.depends_on
    .map((dependency) => {
      const key = String(dependency || '').trim()
      if (!key) return ''
      const matched = taskByKey.get(key) || taskByRecordId.get(key)
      if (!matched) return key
      if (normalizeTeamTaskStatus(matched.status) === 'completed') return ''
      return String(matched.title || matched.task_id || matched.id || key).trim()
    })
    .filter(Boolean)
}

export const resolveSatisfiedTeamTaskDependencies = (
  task: TeamTask,
  tasks: TeamTask[],
): string[] => {
  if (!Array.isArray(task.depends_on) || task.depends_on.length === 0) return []

  const taskByKey = new Map<string, TeamTask>()
  const taskByRecordId = new Map<string, TeamTask>()

  for (const item of tasks) {
    const taskKey = String(item.task_id || '').trim()
    const recordId = String(item.id || '').trim()
    if (taskKey) taskByKey.set(taskKey, item)
    if (recordId) taskByRecordId.set(recordId, item)
  }

  return task.depends_on
    .map((dependency) => {
      const key = String(dependency || '').trim()
      if (!key) return ''
      const matched = taskByKey.get(key) || taskByRecordId.get(key)
      if (!matched) return ''
      if (normalizeTeamTaskStatus(matched.status) !== 'completed') return ''
      return String(matched.title || matched.task_id || matched.id || key).trim()
    })
    .filter(Boolean)
}

export const hasUnresolvedTeamTaskDependencies = (
  task: TeamTask,
  tasks: TeamTask[],
): boolean => {
  return resolveUnresolvedTeamTaskDependencies(task, tasks).length > 0
}

export const teamTaskSectionKey = (
  task: TeamTask,
  tasks: TeamTask[] = [],
): TeamTaskSectionKey => {
  switch (normalizeTeamTaskStatus(task.status)) {
    case 'ready_for_claim':
    case 'pending':
      if (hasUnresolvedTeamTaskDependencies(task, tasks)) return 'blocked'
      return 'executable'
    case 'claimed':
    case 'running':
      return 'active'
    case 'blocked':
      return 'blocked'
    case 'waiting_review':
    case 'waiting_handoff_ack':
      return 'review'
    case 'completed':
      return 'completed'
    case 'failed':
    case 'cancelled':
    case 'unknown':
    default:
      return 'attention'
  }
}

export const teamTaskSectionI18nKey = (section: TeamTaskSectionKey): string => {
  switch (section) {
    case 'executable':
      return 'agent.teamTaskSection.executable'
    case 'active':
      return 'agent.teamTaskSection.active'
    case 'blocked':
      return 'agent.teamTaskSection.blocked'
    case 'review':
      return 'agent.teamTaskSection.review'
    case 'completed':
      return 'agent.teamTaskSection.completed'
    case 'attention':
    default:
      return 'agent.teamTaskSection.attention'
  }
}

export const teamTaskSectionDescriptionI18nKey = (
  section: TeamTaskSectionKey,
): string => {
  switch (section) {
    case 'executable':
      return 'agent.teamTaskSectionDescription.executable'
    case 'active':
      return 'agent.teamTaskSectionDescription.active'
    case 'blocked':
      return 'agent.teamTaskSectionDescription.blocked'
    case 'review':
      return 'agent.teamTaskSectionDescription.review'
    case 'completed':
      return 'agent.teamTaskSectionDescription.completed'
    case 'attention':
    default:
      return 'agent.teamTaskSectionDescription.attention'
  }
}

export const teamTaskSectionEmptyI18nKey = (
  section: TeamTaskSectionKey,
): string => {
  switch (section) {
    case 'executable':
      return 'agent.teamTaskSectionEmpty.executable'
    case 'active':
      return 'agent.teamTaskSectionEmpty.active'
    case 'blocked':
      return 'agent.teamTaskSectionEmpty.blocked'
    case 'review':
      return 'agent.teamTaskSectionEmpty.review'
    case 'completed':
      return 'agent.teamTaskSectionEmpty.completed'
    case 'attention':
    default:
      return 'agent.teamTaskSectionEmpty.attention'
  }
}

export const teamTaskSectionActionI18nKey = (
  section: TeamTaskSectionKey,
): string => {
  switch (section) {
    case 'executable':
      return 'agent.teamTaskSectionAction.executable'
    case 'active':
      return 'agent.teamTaskSectionAction.active'
    case 'blocked':
      return 'agent.teamTaskSectionAction.blocked'
    case 'review':
      return 'agent.teamTaskSectionAction.review'
    case 'completed':
      return 'agent.teamTaskSectionAction.completed'
    case 'attention':
    default:
      return 'agent.teamTaskSectionAction.attention'
  }
}

export const buildTeamTaskSections = (tasks: TeamTask[]) => {
  const order: TeamTaskSectionKey[] = [
    'executable',
    'active',
    'blocked',
    'review',
    'completed',
    'attention',
  ]

  return order
    .map((key) => ({
      key,
      titleKey: teamTaskSectionI18nKey(key),
      descriptionKey: teamTaskSectionDescriptionI18nKey(key),
      emptyKey: teamTaskSectionEmptyI18nKey(key),
      actionKey: teamTaskSectionActionI18nKey(key),
      tasks: tasks.filter((task) => teamTaskSectionKey(task, tasks) === key),
    }))
}
