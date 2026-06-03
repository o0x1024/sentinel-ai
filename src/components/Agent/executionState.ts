export type AgentExecutionOutcome = 'succeeded' | 'failed' | 'cancelled'

export interface AgentExecutionFinishedEvent {
  execution_id: string
  conversation_id?: string | null
  generation?: number | null
  outcome: AgentExecutionOutcome
  success: boolean
  error?: string | null
  response?: string | null
  message?: string | null
}

export interface PersistedAgentExecutionState extends AgentExecutionFinishedEvent {
  completed_at?: string | null
}

export const parseConversationExecutionState = (
  conversationData?: string | null
): PersistedAgentExecutionState | null => {
  if (!conversationData) return null

  try {
    const parsed = JSON.parse(conversationData)
    const executionState = parsed?.execution_state
    if (!executionState || typeof executionState !== 'object') {
      return null
    }
    return executionState as PersistedAgentExecutionState
  } catch {
    return null
  }
}

export const getExecutionStateBadgeClass = (outcome?: AgentExecutionOutcome | null): string => {
  switch (outcome) {
    case 'succeeded':
      return 'badge-success'
    case 'failed':
      return 'badge-error'
    case 'cancelled':
      return 'badge-warning'
    default:
      return 'badge-ghost'
  }
}

export const getExecutionStateLabelKey = (outcome?: AgentExecutionOutcome | null): string => {
  switch (outcome) {
    case 'succeeded':
      return 'agent.executionOutcomeSucceeded'
    case 'failed':
      return 'agent.executionOutcomeFailed'
    case 'cancelled':
      return 'agent.executionOutcomeCancelled'
    default:
      return ''
  }
}
