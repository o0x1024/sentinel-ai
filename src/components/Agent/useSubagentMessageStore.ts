import { computed, onMounted, onUnmounted, reactive, watch, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { TaskRuntimeItem } from '@/types/taskRuntime'

export interface SubagentMessageRecord {
  id: string
  subagent_run_id: string
  role: string
  content?: string | null
  metadata?: string | null
  tool_calls?: string | null
  attachments?: string | null
  reasoning_content?: string | null
  timestamp: string
  structured_data?: string | null
}

export interface SubagentMessageState {
  messages: SubagentMessageRecord[]
  messagesLoading: boolean
  streamingContent: string
  streamingReasoningContent: string
  taskItems: TaskRuntimeItem[]
  loaded: boolean
  loadId: number
}

interface KnownSubagent {
  id: string
}

const createState = (): SubagentMessageState => ({
  messages: [],
  messagesLoading: false,
  streamingContent: '',
  streamingReasoningContent: '',
  taskItems: [],
  loaded: false,
  loadId: 0,
})

const tryParseJson = (str: string): any => {
  try {
    return JSON.parse(str)
  } catch {
    return {}
  }
}

export const useSubagentMessageStore = (params: {
  parentExecutionId: Ref<string | null>
  subagents: Ref<KnownSubagent[]>
}) => {
  const states = reactive<Record<string, SubagentMessageState>>({})
  const unlisteners: UnlistenFn[] = []

  const getState = (subagentId: string): SubagentMessageState => {
    states[subagentId] ??= createState()
    return states[subagentId]
  }

  const hasState = (subagentId: string): boolean => Boolean(states[subagentId])

  const isKnownSubagent = (subagentId: string): boolean =>
    hasState(subagentId) || params.subagents.value.some(subagent => subagent.id === subagentId)

  const selectedState = (subagentId: Ref<string | null>) =>
    computed(() => (subagentId.value ? getState(subagentId.value) : null))

  const upsertPersistedMessage = (message: SubagentMessageRecord): void => {
    const state = getState(message.subagent_run_id)
    const existingIndex = state.messages.findIndex(item => item.id === message.id)
    if (existingIndex >= 0) {
      state.messages[existingIndex] = message
      return
    }

    if (message.role === 'assistant') {
      state.streamingContent = ''
      state.streamingReasoningContent = ''
      state.messages = state.messages.filter(
        item => !item.id.startsWith('stream-') && !item.id.startsWith('final-')
      )
    }

    state.messages.push(message)
  }

  const mergePersistedMessages = (
    subagentId: string,
    persistedMessages: SubagentMessageRecord[]
  ): void => {
    const state = getState(subagentId)
    persistedMessages.forEach(message => upsertPersistedMessage(message))
    state.loaded = true
  }

  const loadMessages = async (subagentId: string): Promise<void> => {
    const state = getState(subagentId)
    const loadId = state.loadId + 1
    state.loadId = loadId
    state.messagesLoading = true

    try {
      const result = await invoke<SubagentMessageRecord[]>('get_subagent_messages', {
        subagentRunId: subagentId,
      })
      if (state.loadId !== loadId) return
      mergePersistedMessages(subagentId, result || [])
    } catch (error) {
      console.error('[useSubagentMessageStore] Failed to load messages:', error)
    } finally {
      if (state.loadId === loadId) {
        state.messagesLoading = false
      }
    }
  }

  const clearAll = (): void => {
    Object.keys(states).forEach(key => {
      delete states[key]
    })
  }

  const startListening = async (): Promise<void> => {
    const unlistenChunk = await listen<{
      execution_id: string
      chunk_type: string
      content?: string
    }>('agent:chunk', event => {
      const payload = event.payload
      if (!isKnownSubagent(payload.execution_id)) return

      const state = getState(payload.execution_id)
      if (payload.chunk_type === 'text' && payload.content) {
        state.streamingContent += payload.content
      } else if (payload.chunk_type === 'reasoning' && payload.content) {
        state.streamingReasoningContent += payload.content
      }
    })
    unlisteners.push(unlistenChunk)

    const unlistenMessage = await listen<{
      subagent_run_id: string
      message_id: string
      role: string
      content?: string | null
      metadata?: string | null
      tool_calls?: string | null
      reasoning_content?: string | null
      timestamp: string
    }>('subagent:message', event => {
      const payload = event.payload
      upsertPersistedMessage({
        id: payload.message_id,
        subagent_run_id: payload.subagent_run_id,
        role: payload.role,
        content: payload.content || null,
        metadata: payload.metadata || null,
        tool_calls: payload.tool_calls || null,
        attachments: null,
        reasoning_content: payload.reasoning_content || null,
        timestamp: payload.timestamp,
        structured_data: null,
      })
    })
    unlisteners.push(unlistenMessage)

    const unlistenToolCall = await listen<{
      execution_id: string
      tool_call_id: string
      tool_name: string
      arguments?: string
    }>('agent:tool_call_complete', event => {
      const payload = event.payload
      if (!isKnownSubagent(payload.execution_id)) return

      const state = getState(payload.execution_id)
      if (state.streamingContent || state.streamingReasoningContent) {
        state.messages.push({
          id: 'stream-' + Date.now(),
          subagent_run_id: payload.execution_id,
          role: 'assistant',
          content: state.streamingContent || null,
          tool_calls: null,
          reasoning_content: state.streamingReasoningContent || null,
          timestamp: new Date().toISOString(),
          metadata: null,
          attachments: null,
          structured_data: null,
        })
        state.streamingContent = ''
        state.streamingReasoningContent = ''
      }

      const toolCallId = payload.tool_call_id
      if (state.messages.some(message => message.id === toolCallId)) return

      state.messages.push({
        id: toolCallId,
        subagent_run_id: payload.execution_id,
        role: 'tool',
        content: `Calling: ${payload.tool_name}`,
        tool_calls: null,
        reasoning_content: null,
        timestamp: new Date().toISOString(),
        metadata: JSON.stringify({
          tool_name: payload.tool_name,
          tool_args: payload.arguments ? tryParseJson(payload.arguments) : {},
          tool_call_id: payload.tool_call_id,
          status: 'running',
        }),
        attachments: null,
        structured_data: null,
      })
    })
    unlisteners.push(unlistenToolCall)

    const unlistenToolResult = await listen<{
      execution_id: string
      tool_call_id: string
      result: string
    }>('agent:tool_result', event => {
      const payload = event.payload
      if (!isKnownSubagent(payload.execution_id)) return

      const state = getState(payload.execution_id)
      const toolCallId = payload.tool_call_id
      const existingMsg = state.messages.find(message => message.id === toolCallId)
      if (!existingMsg) return

      const meta = existingMsg.metadata ? tryParseJson(existingMsg.metadata) : {}
      meta.status = 'completed'
      meta.tool_result = payload.result
      existingMsg.metadata = JSON.stringify(meta)
      existingMsg.content = `Completed: ${meta.tool_name || 'tool'}`
    })
    unlisteners.push(unlistenToolResult)

    const unlistenDone = await listen<{
      execution_id: string
      parent_execution_id: string
      success: boolean
      output?: string
    }>('subagent:done', event => {
      const payload = event.payload
      if (!isKnownSubagent(payload.execution_id)) return

      const state = getState(payload.execution_id)
      if (!state.streamingContent && !state.streamingReasoningContent) return

      state.messages.push({
        id: 'final-' + Date.now(),
        subagent_run_id: payload.execution_id,
        role: 'assistant',
        content: state.streamingContent || payload.output || null,
        tool_calls: null,
        reasoning_content: state.streamingReasoningContent || null,
        timestamp: new Date().toISOString(),
        metadata: null,
        attachments: null,
        structured_data: null,
      })
      state.streamingContent = ''
      state.streamingReasoningContent = ''
    })
    unlisteners.push(unlistenDone)

    const unlistenTasks = await listen<{
      execution_id: string
      tasks: TaskRuntimeItem[]
    }>('agent-tasks-update', event => {
      const payload = event.payload
      if (!isKnownSubagent(payload.execution_id)) return
      getState(payload.execution_id).taskItems = payload.tasks
    })
    unlisteners.push(unlistenTasks)
  }

  const stopListening = (): void => {
    unlisteners.forEach(unlisten => unlisten())
    unlisteners.length = 0
  }

  watch(
    () => params.parentExecutionId.value,
    () => {
      clearAll()
    }
  )

  watch(
    () => params.subagents.value.map(subagent => subagent.id),
    subagentIds => {
      subagentIds.forEach(id => getState(id))
    },
    { immediate: true }
  )

  onMounted(() => {
    void startListening()
  })

  onUnmounted(() => {
    stopListening()
  })

  return {
    clearAll,
    getState,
    loadMessages,
    selectedState,
    startListening,
    stopListening,
  }
}
