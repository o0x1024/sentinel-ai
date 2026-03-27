import type { PersistedAgentExecutionState } from './executionState'

export interface AiConversationSummary {
  id: string
  title?: string | null
  model_name?: string
  total_messages?: number
  created_at?: string
  updated_at?: string
}

export interface AiConversationDetail extends AiConversationSummary {
  execution_state?: PersistedAgentExecutionState | null
}

export interface ConversationOptionSource {
  id: string
  title?: string | null
  updated_at?: string | null
}
