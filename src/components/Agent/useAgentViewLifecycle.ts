import { nextTick, onActivated, onMounted, onUnmounted, type Ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  AgentTeamMessageStreamDeltaEvent,
  AgentTeamMessageStreamDoneEvent,
  AgentTeamMessageStreamStartEvent,
  AgentTeamStateChangedEvent,
  AgentTeamToolCallEvent,
  AgentTeamToolResultEvent,
} from '@/types/agentTeam'
import type { AgentExecutionFinishedEvent, PersistedAgentExecutionState } from './executionState'

interface AgentStartEvent {
  execution_id: string
  task: string
}

interface AgentAssistantMessageSavedEvent {
  execution_id: string
  message_id: string
  content: string
  metadata?: Record<string, unknown> | null
  reasoning_content?: string | null
  timestamp: number
}

export const useAgentViewLifecycle = (params: {
  activeTeamSessionId: Ref<string | null>
  assistantProfileRegistryReady: Ref<boolean>
  conversationExecutionState: Ref<PersistedAgentExecutionState | null>
  conversationId: Ref<string | null>
  executionId?: string
  isTeamWorkspaceActive: Ref<boolean>
  loadAssistantModelOptions: () => Promise<void>
  loadAssistantProfiles: () => Promise<void>
  loadDefaultAssistantProfile: () => Promise<void>
  loadConversationHistory: (conversationId: string) => Promise<void>
  loadLatestConversation: () => Promise<void>
  loadSidebarWidth: () => void
  loadToolConfigDrawerWidth: () => void
  loadTeamWorkspaceData: () => Promise<void>
  loadToolConfig: () => Promise<void>
  focusInput: () => void
  handleConversationExecutionStateUpdate: (payload: AgentExecutionFinishedEvent) => void
  handleTeamAssistantMessageSaved: (payload: AgentAssistantMessageSavedEvent) => Promise<void>
  handleTeamExecutionFinished: (payload: AgentExecutionFinishedEvent) => Promise<void>
  handleTeamMessageStreamDelta: (payload: AgentTeamMessageStreamDeltaEvent) => void
  handleTeamMessageStreamDone: (payload: AgentTeamMessageStreamDoneEvent) => void
  handleTeamMessageStreamStart: (payload: AgentTeamMessageStreamStartEvent) => void
  handleTeamToolCall: (payload: AgentTeamToolCallEvent) => void
  handleTeamToolResult: (payload: AgentTeamToolResultEvent) => void
  preconnectTerminal: () => void
  scrollMessageViewportToBottom: () => void
  syncTeamMessagesToMainFlow: (sessionId?: string | null) => Promise<void>
  applyTeamState: (state: string) => void
}) => {
  const unlistenFns: UnlistenFn[] = []

  const pushListener = async (factory: () => Promise<UnlistenFn>) => {
    const unlisten = await factory()
    unlistenFns.push(unlisten)
  }

  onMounted(async () => {
    await Promise.all([
      params.loadAssistantModelOptions(),
      params.loadAssistantProfiles(),
      params.loadDefaultAssistantProfile(),
    ])

    await pushListener(() => listen('ai_config_updated', async () => {
      await params.loadAssistantModelOptions()
    }))

    await pushListener(() => listen<AgentTeamStateChangedEvent>('agent_team:state_changed', (event) => {
      if (!params.activeTeamSessionId.value || event.payload.session_id !== params.activeTeamSessionId.value) {
        return
      }
      params.applyTeamState(event.payload.state)
      void params.syncTeamMessagesToMainFlow(event.payload.session_id)
      if (params.isTeamWorkspaceActive.value) {
        void params.loadTeamWorkspaceData()
      }
    }))

    await pushListener(() => listen<AgentTeamMessageStreamStartEvent>('agent_team:message_stream_start', (event) => {
      params.handleTeamMessageStreamStart(event.payload)
    }))
    await pushListener(() => listen<AgentTeamMessageStreamDeltaEvent>('agent_team:message_stream_delta', (event) => {
      params.handleTeamMessageStreamDelta(event.payload)
    }))
    await pushListener(() => listen<AgentTeamMessageStreamDoneEvent>('agent_team:message_stream_done', (event) => {
      params.handleTeamMessageStreamDone(event.payload)
    }))
    await pushListener(() => listen<AgentTeamToolCallEvent>('agent_team:tool_call', (event) => {
      params.handleTeamToolCall(event.payload)
    }))
    await pushListener(() => listen<AgentTeamToolResultEvent>('agent_team:tool_result', (event) => {
      params.handleTeamToolResult(event.payload)
    }))
    await pushListener(() => listen<AgentStartEvent>('agent:start', (event) => {
      if (event.payload.execution_id !== params.conversationId.value) return
      params.conversationExecutionState.value = null
    }))
    await pushListener(() => listen<AgentExecutionFinishedEvent>('agent:execution_finished', (event) => {
      params.handleConversationExecutionStateUpdate(event.payload)
      void params.handleTeamExecutionFinished(event.payload)
    }))
    await pushListener(() => listen<AgentAssistantMessageSavedEvent>('agent:assistant_message_saved', (event) => {
      void params.handleTeamAssistantMessageSaved(event.payload)
    }))

    params.loadSidebarWidth()
    params.loadToolConfigDrawerWidth()

    const startupTasks: Promise<unknown>[] = [params.loadToolConfig()]
    if (params.executionId) {
      params.conversationId.value = params.executionId
      startupTasks.push(params.loadConversationHistory(params.executionId))
    } else {
      startupTasks.push(params.loadLatestConversation())
    }

    await Promise.allSettled(startupTasks)
    params.assistantProfileRegistryReady.value = true
    params.preconnectTerminal()

    nextTick(() => {
      params.focusInput()
    })
  })

  onUnmounted(() => {
    while (unlistenFns.length > 0) {
      const unlisten = unlistenFns.pop()
      try {
        unlisten?.()
      } catch {
        // ignore cleanup failures
      }
    }
  })

  onActivated(() => {
    nextTick(() => {
      params.scrollMessageViewportToBottom()
    })
  })
}
