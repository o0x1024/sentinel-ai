import type { Ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { teamRuntimeApi } from '@/api/teamRuntime'
import type { AgentMessage } from '@/types/agent'
import type {
  AgentChunkEvent,
  AgentToolCallCompleteEvent,
  AgentToolResultEvent,
  AgentToolResultNewEvent,
} from '@/composables/useAgentEventTypes'

type SolverActivityStatus = 'running' | 'completed' | 'failed'

interface TeamV4SolverActivityProgressParams {
  executionId: string
  messages: Ref<AgentMessage[]>
  runId: string
  solverId: string
  solverName: string
  taskId: string
  taskKey: string
  taskTitle: string
  scrollToBottom: () => void
}

const summarizeToolResult = (payload: AgentToolResultEvent | AgentToolResultNewEvent) => {
  const raw = 'result' in payload ? payload.result : payload.tool_result
  const source = typeof raw === 'string' ? raw.trim() : JSON.stringify(raw ?? '')
  if (!source) return ''
  const compact = source.replace(/\s+/g, ' ')
  return compact.length > 220 ? `${compact.slice(0, 220)}...` : compact
}

export const startTeamV4SolverActivityProgress = (params: TeamV4SolverActivityProgressParams) => {
  const messageId = crypto.randomUUID()
  const toolNamesByCallId = new Map<string, string>()
  const startedAt = Date.now()
  const unlisteners: UnlistenFn[] = []
  let disposed = false
  let lastActivity = 'Waiting for solver output.'
  let textChunkCount = 0
  let toolCallCount = 0
  let toolResultCount = 0
  let recordedTextStarted = false

  const recordEvent = (eventType: string, payload: Record<string, unknown>) => {
    void teamRuntimeApi.appendEvent(params.runId, {
      actorId: params.solverId,
      taskId: params.taskId,
      eventType,
      visibility: 'workspace',
      payload,
    }).catch((error) => {
      console.warn('[teamV4SolverActivityProgress] Failed to record observable event:', error)
    })
  }

  const writeMessage = (status: SolverActivityStatus = 'running') => {
    if (disposed) return
    const elapsedSeconds = Math.max(0, Math.round((Date.now() - startedAt) / 1000))
    const header = status === 'running'
      ? `Solver activity: ${params.solverName} -> ${params.taskTitle} (${elapsedSeconds}s)`
      : `Solver activity ${status}: ${params.solverName} -> ${params.taskTitle} (${elapsedSeconds}s)`
    const content = [
      header,
      lastActivity,
      `Text chunks: ${textChunkCount} · Tool calls: ${toolCallCount} · Tool results: ${toolResultCount}`,
    ].join('\n')
    const existing = params.messages.value.find((item) => item.id === messageId)
    if (existing) {
      existing.content = content
      existing.timestamp = Date.now()
      existing.metadata = {
        ...existing.metadata,
        status,
        duration_ms: elapsedSeconds * 1000,
      }
    } else {
      params.messages.value.push({
        id: messageId,
        type: 'progress',
        content,
        timestamp: Date.now(),
        metadata: {
          kind: 'team_v4_solver_activity',
          status,
          duration_ms: elapsedSeconds * 1000,
          team_member_id: params.solverId,
          team_member_name: params.solverName,
          team_member_role: 'solver',
          team_session_id: params.runId,
          team_task_record_id: params.taskId,
          team_task_key: params.taskKey,
        },
      })
    }
    params.scrollToBottom()
  }

  const register = async <T>(eventName: string, handler: (payload: T) => void) => {
    const unlisten = await listen<T>(eventName, (event) => {
      const payload = event.payload as T & { execution_id?: string }
      if (payload.execution_id !== params.executionId) return
      handler(payload)
    })
    if (disposed) {
      unlisten()
      return
    }
    unlisteners.push(unlisten)
  }

  void register<AgentChunkEvent>('agent:chunk', (payload) => {
    if (payload.chunk_type !== 'text' && payload.chunk_type !== 'reasoning') return
    textChunkCount += 1
    if (!recordedTextStarted) {
      recordedTextStarted = true
      recordEvent('solver_text_started', {
        executionId: params.executionId,
        chunkType: payload.chunk_type,
      })
    }
    lastActivity = payload.chunk_type === 'reasoning'
      ? 'Solver is reasoning.'
      : 'Solver is writing an answer.'
    writeMessage()
  })

  void register<AgentToolCallCompleteEvent>('agent:tool_call_complete', (payload) => {
    toolCallCount += 1
    toolNamesByCallId.set(payload.tool_call_id, payload.tool_name)
    lastActivity = `Calling tool: ${payload.tool_name}`
    recordEvent('solver_tool_started', {
      executionId: params.executionId,
      toolCallId: payload.tool_call_id,
      toolName: payload.tool_name,
    })
    writeMessage()
  })

  void register<AgentToolResultEvent | AgentToolResultNewEvent>('agent:tool_result', (payload) => {
    toolResultCount += 1
    const toolName = 'tool_name' in payload
      ? payload.tool_name
      : toolNamesByCallId.get(payload.tool_call_id) || payload.tool_call_id
    const summary = summarizeToolResult(payload)
    lastActivity = summary
      ? `Tool result: ${toolName} -> ${summary}`
      : `Tool result: ${toolName}`
    recordEvent('solver_tool_result', {
      executionId: params.executionId,
      toolCallId: 'tool_call_id' in payload ? payload.tool_call_id : null,
      toolName,
      summary,
      success: 'success' in payload ? payload.success === true : undefined,
    })
    writeMessage()
  })

  writeMessage()

  return {
    complete() {
      lastActivity = 'Solver execution completed.'
      writeMessage('completed')
    },
    fail(error: string) {
      lastActivity = `Solver execution failed: ${error}`
      writeMessage('failed')
    },
    dispose() {
      disposed = true
      while (unlisteners.length > 0) {
        const unlisten = unlisteners.pop()
        try {
          unlisten?.()
        } catch {
          // ignore listener cleanup failures
        }
      }
    },
  }
}
