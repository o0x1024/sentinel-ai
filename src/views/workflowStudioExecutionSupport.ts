import type { Ref } from 'vue'

export interface ExecutionRecord {
  id: string
  start_time: string
  end_time?: string
  duration?: number
  status: 'pending' | 'running' | 'completed' | 'failed'
  step_results: Record<string, any>
}

export interface HistoryItem {
  execution_id: string
  workflow_id: string
  workflow_name: string
  version: string
  status: string
  started_at: string
  completed_at?: string
  duration_ms?: number
  progress: number
  total_steps: number
  completed_steps: number
  error_message?: string
}

export interface DetailData {
  execution_id: string
  workflow_id: string
  workflow_name: string
  version: string
  status: string
  started_at: string
  completed_at?: string
  duration_ms?: number
  progress: number
  total_steps: number
  completed_steps: number
  error_message?: string
  steps: Array<{
    step_id: string
    step_name?: string
    step_order?: number
    status: string
    started_at?: string
    completed_at?: string
    duration_ms?: number
    result?: any
    error_message?: string
  }>
}

export interface ExecutionLog {
  timestamp: Date
  level: 'INFO' | 'WARN' | 'ERROR' | 'SUCCESS'
  message: string
  node_id?: string
  details?: string
}

const stringifyForLog = (value: any): string => {
  try {
    if (typeof value === 'string') return value
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

export const truncateWorkflowLogDetails = (value: any, maxLength: number): string => {
  const text = stringifyForLog(value)
  if (text.length <= maxLength) return text
  return `${text.slice(0, maxLength)}\n... (truncated)`
}

export const addWorkflowExecutionLog = ({
  executionLogs,
  expandedLogs,
  logsContainerRef,
  maxExecutionLogs,
  maxLogDetailsLength,
  level,
  message,
  nodeId,
  details,
}: {
  executionLogs: Ref<ExecutionLog[]>
  expandedLogs: Ref<Set<number>>
  logsContainerRef: Ref<HTMLElement | null>
  maxExecutionLogs: number
  maxLogDetailsLength: number
  level: ExecutionLog['level']
  message: string
  nodeId?: string
  details?: string
}) => {
  executionLogs.value.push({
    timestamp: new Date(),
    level,
    message,
    node_id: nodeId,
    details: details ? truncateWorkflowLogDetails(details, maxLogDetailsLength) : undefined,
  })

  if (executionLogs.value.length > maxExecutionLogs) {
    const overflow = executionLogs.value.length - maxExecutionLogs
    executionLogs.value.splice(0, overflow)
    const nextExpanded = new Set<number>()
    expandedLogs.value.forEach((idx) => {
      const mapped = idx - overflow
      if (mapped >= 0) nextExpanded.add(mapped)
    })
    expandedLogs.value = nextExpanded
  }

  requestAnimationFrame(() => {
    const logContainer = logsContainerRef.value
    if (logContainer) {
      logContainer.scrollTop = logContainer.scrollHeight
    }
  })
}

export const clearWorkflowExecutionLogs = (
  executionLogs: Ref<ExecutionLog[]>,
  expandedLogs: Ref<Set<number>>,
) => {
  executionLogs.value = []
  expandedLogs.value.clear()
}

export const toggleWorkflowLogDetails = (expandedLogs: Ref<Set<number>>, idx: number) => {
  if (expandedLogs.value.has(idx)) {
    expandedLogs.value.delete(idx)
  } else {
    expandedLogs.value.add(idx)
  }
}

export const formatWorkflowResult = (result: any, noResultLabel: string) => {
  if (result === undefined || result === null) return noResultLabel
  if (typeof result === 'object') {
    return JSON.stringify(result, null, 2)
  }
  return String(result)
}

export const formatWorkflowDatetime = (dateStr?: string) => {
  if (!dateStr) return '-'
  return new Date(dateStr).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

export const formatWorkflowDuration = (ms?: number) => {
  if (ms === undefined || ms === null) return '-'
  if (ms < 1000) return `${ms}ms`
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
  return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`
}

export const getWorkflowStatusBadgeClass = (status: string) => {
  switch (status) {
    case 'completed': return 'badge-success'
    case 'failed': return 'badge-error'
    case 'running': return 'badge-warning'
    case 'pending': return 'badge-ghost'
    case 'cancelled': return 'badge-neutral'
    default: return 'badge-ghost'
  }
}

export const getWorkflowLogClass = (level: string) => {
  switch (level) {
    case 'ERROR': return 'text-error'
    case 'WARN': return 'text-warning'
    case 'SUCCESS': return 'text-success'
    default: return 'text-base-content'
  }
}

export const formatWorkflowLogTime = (date: Date) => date.toLocaleTimeString('zh-CN', { hour12: false })

export const formatWorkflowShortDate = (dateStr: string) =>
  new Date(dateStr).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })

export const startWorkflowExecutionRecord = (
  executionHistory: Ref<ExecutionRecord[]>,
  currentExecutionId: Ref<string | null>,
) => {
  const id = `exec_${Date.now()}`
  const now = new Date().toLocaleString('zh-CN')

  executionHistory.value.unshift({
    id,
    start_time: now,
    status: 'running',
    step_results: {},
  })
  currentExecutionId.value = id

  if (executionHistory.value.length > 20) {
    executionHistory.value = executionHistory.value.slice(0, 20)
  }

  return id
}

export const updateWorkflowExecutionStepResult = (
  executionHistory: Ref<ExecutionRecord[]>,
  currentExecutionId: Ref<string | null>,
  stepId: string,
  result: any,
) => {
  const execution = executionHistory.value.find((entry) => entry.id === currentExecutionId.value)
  if (execution) {
    execution.step_results[stepId] = result
  }
}

export const completeWorkflowExecutionRecord = (
  executionHistory: Ref<ExecutionRecord[]>,
  currentExecutionId: Ref<string | null>,
  success: boolean,
) => {
  const execution = executionHistory.value.find((entry) => entry.id === currentExecutionId.value)
  if (!execution) return

  execution.status = success ? 'completed' : 'failed'
  execution.end_time = new Date().toLocaleString('zh-CN')
  execution.duration = Date.now() - new Date(execution.start_time).getTime()
}

export const saveWorkflowExecutionHistory = (workflowId: string, executionHistory: ExecutionRecord[]) => {
  localStorage.setItem(
    `workflow_execution_history_${workflowId}`,
    JSON.stringify(executionHistory.slice(0, 10)),
  )
}

export const loadWorkflowExecutionHistory = (workflowId: string): ExecutionRecord[] => {
  const saved = localStorage.getItem(`workflow_execution_history_${workflowId}`)
  return saved ? JSON.parse(saved) : []
}

export const clearWorkflowExecutionHistoryState = (
  workflowId: string,
  executionHistory: Ref<ExecutionRecord[]>,
  selectedExecution: Ref<ExecutionRecord | null>,
) => {
  executionHistory.value = []
  selectedExecution.value = null
  localStorage.removeItem(`workflow_execution_history_${workflowId}`)
}

export const deleteWorkflowExecutionRecord = (
  executionHistory: Ref<ExecutionRecord[]>,
  selectedExecution: Ref<ExecutionRecord | null>,
  executionId: string,
) => {
  const idx = executionHistory.value.findIndex((entry) => entry.id === executionId)
  if (idx === -1) return
  if (selectedExecution.value?.id === executionId) {
    selectedExecution.value = null
  }
  executionHistory.value.splice(idx, 1)
}
