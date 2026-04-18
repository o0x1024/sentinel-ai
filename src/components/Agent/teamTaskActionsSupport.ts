import type { TeamTask, TeamTaskActionResult } from '@/types/agentTeam'

const normalize = (value: unknown): string => String(value || '').trim()

export const getTeamTaskClaimAgentId = (task: TeamTask): string | null => {
  const ownerId = normalize(task.owner_agent_id)
  if (ownerId) return ownerId
  const assigneeId = normalize(task.assignee_agent_id)
  return assigneeId || null
}

export const getTeamTaskReleaseAgentId = (task: TeamTask): string | null => {
  const claimedById = normalize(task.claimed_by_agent_id)
  if (claimedById) return claimedById
  const assigneeId = normalize(task.assignee_agent_id)
  return assigneeId || null
}

export const canTeamTaskClaim = (task: TeamTask): boolean => {
  const status = normalize(task.status).toLowerCase()
  if (!['pending', 'ready_for_claim'].includes(status)) return false
  return Boolean(getTeamTaskClaimAgentId(task))
}

export const canTeamTaskRelease = (task: TeamTask): boolean => {
  const status = normalize(task.status).toLowerCase()
  if (!['claimed', 'running'].includes(status)) return false
  return Boolean(getTeamTaskReleaseAgentId(task))
}

export const canTeamTaskComplete = (task: TeamTask): boolean => {
  const status = normalize(task.status).toLowerCase()
  return ['claimed', 'running', 'waiting_review'].includes(status)
}

export const canTeamTaskFail = (task: TeamTask): boolean => {
  const status = normalize(task.status).toLowerCase()
  return ['claimed', 'running', 'waiting_review'].includes(status)
}

export const canTeamTaskBlock = (task: TeamTask): boolean => {
  const status = normalize(task.status).toLowerCase()
  return ['pending', 'ready_for_claim', 'claimed', 'running', 'waiting_review'].includes(status)
}

export const buildTeamTaskActionToastMessage = (
  result: Pick<TeamTaskActionResult, 'message' | 'reason' | 'next_step'>,
): string => {
  return [result.message, result.reason, result.next_step]
    .map((part) => normalize(part))
    .filter(Boolean)
    .join(' ')
}
