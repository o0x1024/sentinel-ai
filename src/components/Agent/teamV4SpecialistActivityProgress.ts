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
import { appendTeamV4TimelineMessages, buildTeamV4TimelineMessages } from './teamV4MessageTimelineSupport'

interface TeamV4SpecialistActivityProgressParams {
  executionId: string
  messages: Ref<AgentMessage[]>
  runId: string
  specialistId: string
  specialistName: string
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

export const startTeamV4SpecialistActivityProgress = (params: TeamV4SpecialistActivityProgressParams) => {
  const toolArgumentsByCallId = new Map<string, string>()
  const toolNamesByCallId = new Map<string, string>()
  const unlisteners: UnlistenFn[] = []
  let disposed = false
  let recordedTextStarted = false

  const appendObservableEventMessage = (event: Awaited<ReturnType<typeof teamRuntimeApi.appendEvent>>) => {
    params.messages.value = appendTeamV4TimelineMessages(
      params.messages.value,
      buildTeamV4TimelineMessages({
        agents: [
          {
            id: params.specialistId,
            run_id: params.runId,
            profile_id: null,
            role_type: 'specialist',
            name: params.specialistName,
            status: 'running',
            model: null,
            context_mode: null,
            tool_policy_json: {},
            metadata: {},
            created_at: event.created_at,
            updated_at: event.created_at,
          },
        ],
        tasks: [
          {
            id: params.taskId,
            run_id: params.runId,
            parent_task_id: null,
            task_key: params.taskKey,
            title: params.taskTitle,
            instruction: '',
            status: 'running',
            priority: 0,
            assigned_agent_id: params.specialistId,
            depends_on: [],
            acceptance_criteria: null,
            context_snapshot_id: null,
            metadata: {},
            created_at: event.created_at,
            updated_at: event.created_at,
          },
        ],
        events: [event],
      }),
    )
    params.scrollToBottom()
  }

  const recordEvent = (eventType: string, payload: Record<string, unknown>) => {
    void teamRuntimeApi.appendEvent(params.runId, {
      actorId: params.specialistId,
      taskId: params.taskId,
      eventType,
      visibility: 'workspace',
      payload,
    })
      .then(appendObservableEventMessage)
      .catch((error) => {
        console.warn('[teamV4SpecialistActivityProgress] Failed to record observable event:', error)
      })
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
    if (!recordedTextStarted) {
      recordedTextStarted = true
      recordEvent('specialist_text_started', {
        executionId: params.executionId,
        chunkType: payload.chunk_type,
      })
    }
  })

  void register<AgentToolCallCompleteEvent>('agent:tool_call_complete', (payload) => {
    toolArgumentsByCallId.set(payload.tool_call_id, payload.arguments)
    toolNamesByCallId.set(payload.tool_call_id, payload.tool_name)
    recordEvent('specialist_tool_started', {
      executionId: params.executionId,
      toolCallId: payload.tool_call_id,
      arguments: payload.arguments,
      toolName: payload.tool_name,
    })
  })

  void register<AgentToolResultEvent | AgentToolResultNewEvent>('agent:tool_result', (payload) => {
    const toolCallId = 'tool_call_id' in payload ? payload.tool_call_id : null
    const toolName = 'tool_name' in payload
      ? payload.tool_name
      : toolNamesByCallId.get(payload.tool_call_id) || payload.tool_call_id
    const rawResult = 'result' in payload ? payload.result : payload.tool_result
    const summary = summarizeToolResult(payload)
    recordEvent('specialist_tool_result', {
      executionId: params.executionId,
      arguments: toolCallId ? toolArgumentsByCallId.get(toolCallId) || null : null,
      result: rawResult,
      toolCallId,
      toolName,
      summary,
      success: 'success' in payload ? payload.success === true : undefined,
      trackedArtifacts: payload.tracked_artifacts,
    })
  })

  return {
    complete() {},
    fail(_error: string) {},
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
