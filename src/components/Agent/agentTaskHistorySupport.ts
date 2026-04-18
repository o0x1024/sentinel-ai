import type { AgentTask } from '@/types/agentTask'

export interface PersistedAgentTaskRow {
  id: string
  execution_id: string
  item_index: number
  content: string
  status: string
  result?: string | null
  created_at_ms: number
  updated_at_ms: number
}

const normalizeStatus = (value: string): AgentTask['status'] => {
  switch ((value || '').trim().toLowerCase()) {
    case 'in_progress':
      return 'in_progress'
    case 'completed':
      return 'completed'
    case 'failed':
      return 'failed'
    case 'cancelled':
      return 'cancelled'
    default:
      return 'pending'
  }
}

export const mapPersistedAgentTasks = (rows: PersistedAgentTaskRow[]): AgentTask[] => {
  return (rows || []).map((row) => ({
    id: row.id,
    title: row.content,
    status: normalizeStatus(row.status),
    created_at: Number(row.created_at_ms || 0),
    updated_at: Number(row.updated_at_ms || 0),
    metadata: {
      reason: row.result || null,
      source_execution_id: row.execution_id,
      step_index: Number.isFinite(row.item_index) ? row.item_index + 1 : undefined,
    },
  }))
}
