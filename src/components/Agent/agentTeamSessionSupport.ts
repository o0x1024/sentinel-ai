import type { AgentTeamSession } from '@/types/agentTeam'

export const applyTeamStateChange = (
  currentState: string,
  nextState: string,
): { changed: boolean; normalized: string; requiresHumanPrompt: boolean } => {
  const normalized = (nextState || '').trim()
  if (!normalized) {
    return {
      changed: false,
      normalized: currentState,
      requiresHumanPrompt: false,
    }
  }
  const changed = currentState !== normalized
  return {
    changed,
    normalized,
    requiresHumanPrompt: changed && normalized === 'SUSPENDED_FOR_HUMAN',
  }
}

export const buildTeamSessionName = (goal: string): string => {
  const normalized = (goal || '').replace(/\s+/g, ' ').trim()
  if (!normalized) return 'Team Session'
  const preview = normalized.length > 96 ? `${normalized.slice(0, 96)}...` : normalized
  return `Team: ${preview}`
}

export const pickActiveTeamSession = (
  sessions: AgentTeamSession[],
  preferredSessionId?: string | null,
): AgentTeamSession | null => {
  const preferred = preferredSessionId
    ? (sessions.find((item) => item.id === preferredSessionId && item.state !== 'ARCHIVED') || null)
    : null
  return preferred || sessions.find((item) => item.state !== 'ARCHIVED') || null
}
