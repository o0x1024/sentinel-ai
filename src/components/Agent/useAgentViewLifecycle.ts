import { nextTick, onActivated, onMounted, onUnmounted, type Ref } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AgentExecutionFinishedEvent, PersistedAgentExecutionState } from './executionState'

interface AgentStartEvent {
  execution_id: string
  conversation_id?: string | null
  generation?: number | null
  task: string
}

export const useAgentViewLifecycle = (params: {
  assistantProfileRegistryReady: Ref<boolean>
  conversationExecutionState: Ref<PersistedAgentExecutionState | null>
  conversationId: Ref<string | null>
  executionId?: string
  loadAssistantModelOptions: () => Promise<void>
  loadAssistantProfiles: () => Promise<void>
  loadDefaultAssistantProfile: () => Promise<void>
  loadDefaultTeamProfile: () => Promise<void>
  loadTeamProfiles: () => Promise<void>
  loadConversationHistory: (conversationId: string) => Promise<void>
  loadLatestConversation: () => Promise<void>
  loadSidebarWidth: () => void
  loadToolConfigDrawerWidth: () => void
  loadToolConfig: () => Promise<void>
  focusInput: () => void
  handleConversationExecutionStateUpdate: (payload: AgentExecutionFinishedEvent) => void
  preconnectTerminal: () => void
  scrollMessageViewportToBottom: () => void
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
      params.loadTeamProfiles(),
      params.loadDefaultTeamProfile(),
    ])

    await pushListener(() =>
      listen('ai_config_updated', async () => {
        await params.loadAssistantModelOptions()
      })
    )

    await pushListener(() =>
      listen<AgentStartEvent>('agent:start', event => {
        const payloadConversationId = event.payload.conversation_id || event.payload.execution_id
        if (payloadConversationId !== params.conversationId.value) return
        params.conversationExecutionState.value = null
      })
    )
    await pushListener(() =>
      listen<AgentExecutionFinishedEvent>('agent:execution_finished', event => {
        params.handleConversationExecutionStateUpdate(event.payload)
      })
    )

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
