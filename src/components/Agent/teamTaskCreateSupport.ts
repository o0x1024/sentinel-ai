import type { TeamTaskCreateInput } from '@/types/agentTeam'

const normalize = (value: unknown): string => String(value || '').trim()

const slugify = (value: string): string => {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .replace(/-{2,}/g, '-')
}

export const buildTeamTaskKey = (
  input: Pick<TeamTaskCreateInput, 'title'>,
  nowMs = Date.now(),
): string => {
  const base = slugify(normalize(input.title))
  if (base) return base
  return `task-${Math.max(0, Math.floor(nowMs)).toString(36)}`
}

export const buildTeamTaskCreateRequest = (
  input: TeamTaskCreateInput,
  nowMs = Date.now(),
) => {
  const title = normalize(input.title)
  const instruction = normalize(input.instruction)
  const dependsOn = Array.isArray(input.depends_on)
    ? input.depends_on
        .map((item) => normalize(item))
        .filter(Boolean)
    : []
  const ownerAgentId = normalize(input.owner_agent_id)
  const acceptanceCriteria = normalize(input.acceptance_criteria)

  return {
    task_key: buildTeamTaskKey({ title }, nowMs),
    title,
    instruction,
    owner_agent_id: ownerAgentId || null,
    acceptance_criteria: acceptanceCriteria || null,
    priority: null,
    metadata: dependsOn.length > 0 ? { depends_on: dependsOn } : null,
  }
}
