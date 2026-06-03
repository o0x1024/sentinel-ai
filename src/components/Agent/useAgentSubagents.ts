import { ref, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface AgentSubagentItem {
  id: string
  role?: string
  status: 'running' | 'queued' | 'completed' | 'failed'
  progress?: number
  tools?: string[]
  parentId: string
  summary?: string
  task?: string
  error?: string
  startedAt?: number
  duration?: number
}

interface SubagentRunRecord {
  id: string
  parent_execution_id: string
  role?: string | null
  task: string
  status: 'running' | 'queued' | 'completed' | 'failed'
  output?: string | null
  error?: string | null
  started_at: string
  completed_at?: string | null
  created_at: string
  updated_at: string
}

export const useAgentSubagents = (params: {
  conversationId: Ref<string | null>
  historyLoadToken: Ref<number>
  subagents: Ref<AgentSubagentItem[]>
}) => {
  const showSubagentDetailModal = ref(false)
  const selectedSubagent = ref<AgentSubagentItem | null>(null)

  const loadSubagentRuns = async (parentExecutionId: string, loadToken?: number) => {
    try {
      const runs = await invoke<SubagentRunRecord[]>('get_subagent_runs', {
        parentExecutionId,
      })
      if (
        (typeof loadToken === 'number' && loadToken !== params.historyLoadToken.value) ||
        params.conversationId.value !== parentExecutionId
      ) {
        return
      }

      const toMillis = (value: unknown) => {
        const ms = new Date(value as any).getTime()
        return Number.isFinite(ms) ? ms : undefined
      }

      const mapped = (runs || []).map((run) => {
        const startedAt = toMillis(run.started_at)
        const completedAt = toMillis(run.completed_at)
        const duration = startedAt !== undefined && completedAt !== undefined
          ? Math.max(0, completedAt - startedAt)
          : undefined

        const summary = (run.output || '').trim()
        return {
          id: run.id,
          parentId: run.parent_execution_id,
          role: run.role || undefined,
          status: run.status,
          progress: run.status === 'running' || run.status === 'queued' ? 0 : 100,
          task: run.task,
          summary: summary.length > 0 ? summary.slice(0, 200) : undefined,
          error: run.error || undefined,
          startedAt,
          duration,
        } satisfies AgentSubagentItem
      })

      const byId = new Map<string, AgentSubagentItem>()
      params.subagents.value.forEach((subagent) => byId.set(subagent.id, subagent))
      mapped.forEach((subagent) => {
        const previous = byId.get(subagent.id)
        byId.set(subagent.id, previous ? { ...subagent, ...previous } : subagent)
      })

      params.subagents.value = [...byId.values()].sort((a, b) => {
        const at = a.startedAt ?? 0
        const bt = b.startedAt ?? 0
        if (bt !== at) return bt - at
        return String(b.id).localeCompare(String(a.id))
      })
    } catch (error) {
      console.error('[useAgentSubagents] Failed to load subagent runs:', error)
    }
  }

  const handleViewSubagentDetails = (subagentId: string) => {
    const subagent = params.subagents.value.find((item) => item.id === subagentId)
    if (!subagent) return
    selectedSubagent.value = subagent
    showSubagentDetailModal.value = true
  }

  return {
    handleViewSubagentDetails,
    loadSubagentRuns,
    selectedSubagent,
    showSubagentDetailModal,
  }
}
