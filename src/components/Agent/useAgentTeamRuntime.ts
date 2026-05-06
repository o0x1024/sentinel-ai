import { computed, onUnmounted, ref, watch, type ComputedRef, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AgentMessage } from '@/types/agent'
import type { AssistantConversationBinding } from './agentDraftTypes'
import type {
  AgentTeamMessage,
  AgentTeamMessageStreamDeltaEvent,
  AgentTeamMessageStreamDoneEvent,
  AgentTeamMessageStreamStartEvent,
  AgentTeamSession,
  AgentTeamToolCallEvent,
  AgentTeamToolResultEvent,
  TeamBlackboardEntry,
  TeamTask,
} from '@/types/agentTeam'
import { agentTeamApi } from '@/api/agentTeam'
import { appendTeamMessagesToMainFlow } from './agentTeamMainFlowSupport'
import {
  buildTeamPersistedAssistantSuppressionKey,
  parseTeamMessageTimestamp,
  shouldSuppressTeamMirrorNoiseMessage,
} from './agentTeamMessageSupport'
import {
  createAndStartTeamSession as createAndStartTeamSessionSupport,
  ensureConversationForTeamSession as ensureConversationForTeamSessionSupport,
  routeTeamMessage as routeTeamMessageSupport,
  startTeamExecutionRun as startTeamExecutionRunSupport,
} from './agentTeamEntrySupport'
import { resolveTeamSequenceInsertIndex, upsertTeamToolCallInMainFlow } from './agentTeamToolCallSupport'
import {
  consumeTeamStreamTempForPersistedMessage as consumeTeamStreamTempForPersistedMessageSupport,
  flushTeamStreamDeltaBuffers as flushTeamStreamDeltaBuffersSupport,
  markTeamStreamDoneForReconcile as markTeamStreamDoneForReconcileSupport,
  splitTeamStreamAssistantSegmentAtToolBoundary as splitTeamStreamAssistantSegmentAtToolBoundarySupport,
  upsertTeamStreamTempMessage as upsertTeamStreamTempMessageSupport,
} from './agentTeamStreamSupport'
import {
  mirrorTeamMessageToConversation as mirrorTeamMessageToConversationSupport,
  persistTeamToolEvent as persistTeamToolEventSupport,
} from './agentTeamPersistenceSupport'
import {
  collectTeamMirroredSourceIds as collectTeamMirroredSourceIdsForAssistantSupport,
  mirrorAssistantOutputToTeamSession as mirrorAssistantOutputToTeamSessionSupport,
  syncLatestAssistantOutputToTeamSession as syncLatestAssistantOutputToTeamSessionSupport,
} from './agentTeamAssistantBridgeSupport'
import { applyTeamStateChange, pickActiveTeamSession } from './agentTeamSessionSupport'
import {
  pollTeamRunStatusOnce as pollTeamRunStatusOnceSupport,
  refreshTeamRuntimeData as refreshTeamRuntimeDataSupport,
  syncTeamMessagesToMainFlow as syncTeamMessagesToMainFlowSupport,
} from './agentTeamRuntimeSupport'
import { loadTeamWorkspaceData as loadTeamWorkspaceDataSupport } from './agentTeamWorkspaceSupport'
import type { AgentExecutionFinishedEvent } from './executionState'
import type { TeamOrchestrationPresetId, TeamRecoveryPresetId } from './teamOrchestrationTypes'
import { buildRuntimeToolConfigForExecution, type UiToolConfigPayload } from './toolConfigRuntime'

const TEAM_BLACKBOARD_FETCH_LIMIT = 1000
const TEAM_RUN_STATUS_POLL_INTERVAL_MS = 2000
const TEAM_STREAM_DELTA_FLUSH_INTERVAL_MS = 80
const TEAM_ACTIVE_SESSION_MAP_STORAGE_KEY = 'sentinel:agent:team-active-session-map'
const TEAM_RUNNING_STATES = new Set([
  'EXECUTING',
  'INITIALIZING',
  'PROPOSING',
  'CHALLENGING',
  'CONVERGENCE_CHECK',
  'REVISING',
  'DECIDING',
  'ARTIFACT_GENERATION',
])

export const useAgentTeamRuntime = (params: {
  activeTeamSessionId: Ref<string | null>
  activateRightPanel: (panel: 'team') => void
  activeRightPanel: Ref<string | null>
  agentMessages: Ref<AgentMessage[]>
  buildCurrentConversationBinding: () => AssistantConversationBinding
  buildToolPolicyFromUiConfig: (config: UiToolConfigPayload) => Record<string, unknown>
  clearLocalError: () => void
  conversationId: Ref<string | null>
  currentConversationTitle: Ref<string>
  deactivateRightPanel: (panel: 'team') => void
  flushPendingToolConfigSave: () => Promise<void>
  getDisplayConversationTitle: () => string
  getNewConversationTitle: () => string
  handleStopExecution: () => Promise<void>
  isExecuting: ComputedRef<boolean>
  isTeamScopedMainFlowMessage: (message: AgentMessage) => boolean
  isTeamWorkspaceActive: Ref<boolean>
  loadConversationList: () => void
  markConversationExecutionPending: () => void
  onTeamWorkspaceSnapshotLoaded?: () => void
  ragEnabled: Ref<boolean>
  selectedTeamTaskId: Ref<string | null>
  setLocalError: (message: string) => void
  teamBlackboardEntries: Ref<TeamBlackboardEntry[]>
  teamModeEnabled: Ref<boolean>
  teamWorkspaceAvailable: ComputedRef<boolean>
  teamSelectedOrchestrationPresetId: Ref<TeamOrchestrationPresetId | null>
  teamSelectedRecoveryPresetId: Ref<TeamRecoveryPresetId>
  teamSessionDetail: Ref<AgentTeamSession | null>
  teamSessionMessages: Ref<AgentTeamMessage[]>
  teamSessionState: Ref<string>
  teamTasks: Ref<TeamTask[]>
  teamWorkspaceLoading: Ref<boolean>
  teamWorkspaceTab: Ref<'tasks' | 'inbox' | 'blackboard' | 'agents'>
  toolConfig: Ref<UiToolConfigPayload>
  webSearchEnabled: Ref<boolean>
}) => {
  let teamRunStatusPollTimer: ReturnType<typeof setInterval> | null = null
  let isPollingTeamRunStatus = false
  let teamMainFlowMessageIds = new Set<string>()
  let teamMirroredAssistantSourceIds = new Set<string>()
  let teamMirroredConversationMessageIds = new Set<string>()
  const teamPersistedToolEventKeys = new Set<string>()
  const teamStreamTempMessageByStreamId = new Map<string, string>()
  const teamStreamSegmentSeqByStreamId = new Map<string, number>()
  const teamStreamDoneIdsBySignature = new Map<string, string[]>()
  const teamLocalHumanMessageIdsBySignature = new Map<string, string[]>()
  const teamStreamDeltaBufferByStreamId = new Map<string, string>()
  const teamActiveStreamIds = new Set<string>()
  const teamPersistedAssistantSuppressionKeys = new Set<string>()
  let teamStreamDeltaFlushTimer: ReturnType<typeof setTimeout> | null = null
  let isSyncingTeamMessages = false
  let pendingTeamMessageSyncSessionId: string | null = null

  const isTeamRunActive = computed(() => {
    if (!params.teamModeEnabled.value || !params.activeTeamSessionId.value) return false
    const normalized = String(params.teamSessionState.value || '').trim().toUpperCase()
    return TEAM_RUNNING_STATES.has(normalized)
  })

  const loadTeamActiveSessionMap = (): Record<string, string> => {
    if (typeof window === 'undefined') return {}
    try {
      const raw = window.localStorage.getItem(TEAM_ACTIVE_SESSION_MAP_STORAGE_KEY)
      if (!raw) return {}
      const parsed = JSON.parse(raw)
      if (!parsed || typeof parsed !== 'object') return {}
      return parsed as Record<string, string>
    } catch {
      return {}
    }
  }

  const persistTeamActiveSessionMap = (next: Record<string, string>) => {
    if (typeof window === 'undefined') return
    try {
      window.localStorage.setItem(TEAM_ACTIVE_SESSION_MAP_STORAGE_KEY, JSON.stringify(next))
    } catch (error) {
      console.warn('[useAgentTeamRuntime] Failed to persist active session map:', error)
    }
  }

  const getPersistedTeamSessionId = (conversationId?: string | null): string | null => {
    const normalized = String(conversationId || '').trim()
    if (!normalized) return null
    const sessionId = loadTeamActiveSessionMap()[normalized]
    return typeof sessionId === 'string' && sessionId.trim() ? sessionId.trim() : null
  }

  const setPersistedTeamSessionId = (conversationId?: string | null, sessionId?: string | null) => {
    const normalizedConversationId = String(conversationId || '').trim()
    if (!normalizedConversationId) return
    const next = loadTeamActiveSessionMap()
    const normalizedSessionId = String(sessionId || '').trim()
    if (normalizedSessionId) {
      next[normalizedConversationId] = normalizedSessionId
    } else {
      delete next[normalizedConversationId]
    }
    persistTeamActiveSessionMap(next)
  }

  const pruneTeamLocalHumanInputReconcileQueue = (sessionId?: string | null) => {
    const normalizedSessionId = String(sessionId || '').trim()
    if (!normalizedSessionId) {
      teamLocalHumanMessageIdsBySignature.clear()
      return
    }
    for (const key of [...teamLocalHumanMessageIdsBySignature.keys()]) {
      if (!key.startsWith(`${normalizedSessionId}\u0001`)) {
        teamLocalHumanMessageIdsBySignature.delete(key)
      }
    }
  }

  const appendTeamBridgeMessage = (content: string) => {
    const normalized = (content || '').replace(/^\s*\[Team\]\s*/i, '').trim()
    params.agentMessages.value.push({
      id: crypto.randomUUID(),
      type: 'system',
      content: normalized || content,
      timestamp: Date.now(),
      metadata: {
        kind: 'team_bridge',
      },
    })
  }

  const buildTeamHumanInputSignature = (sessionId: string | null | undefined, content: string) =>
    `${String(sessionId || '').trim()}\u0001${content.trim()}`

  const markTeamLocalHumanInputForReconcile = (sessionId: string, content: string, localMessageId: string) => {
    const signature = buildTeamHumanInputSignature(sessionId, content)
    const queue = teamLocalHumanMessageIdsBySignature.get(signature) || []
    queue.push(localMessageId)
    teamLocalHumanMessageIdsBySignature.set(signature, queue)
  }

  const consumeTeamLocalHumanInputForPersistedMessage = (
    sessionId: string | null | undefined,
    content: string,
  ): number | null => {
    const signature = buildTeamHumanInputSignature(sessionId, content)
    const queue = teamLocalHumanMessageIdsBySignature.get(signature)
    if (!queue || queue.length === 0) return null
    let removedIndex: number | null = null

    while (queue.length > 0) {
      const localId = queue.shift()
      if (!localId) break
      const idx = params.agentMessages.value.findIndex((item) => item.id === localId)
      if (idx >= 0) {
        params.agentMessages.value.splice(idx, 1)
        removedIndex = removedIndex === null ? idx : Math.min(removedIndex, idx)
      }
    }

    if (queue.length === 0) {
      teamLocalHumanMessageIdsBySignature.delete(signature)
    } else {
      teamLocalHumanMessageIdsBySignature.set(signature, queue)
    }

    return removedIndex
  }

  const pushTeamHumanInputLocalEcho = (sessionId: string, content: string) => {
    const normalizedContent = content.trim()
    if (!sessionId || !normalizedContent) return
    const localMessageId = `team-local-user:${crypto.randomUUID()}`
    params.agentMessages.value.push({
      id: localMessageId,
      type: 'user',
      content: normalizedContent,
      timestamp: Date.now(),
      metadata: {
        kind: 'team_human_input',
        team_member_id: 'human',
        team_member_name: 'human',
        team_member_role: 'user',
        team_session_id: sessionId,
      },
    })
    markTeamLocalHumanInputForReconcile(sessionId, normalizedContent, localMessageId)
  }

  const insertMainFlowMessageAtPreferredIndex = (message: AgentMessage, preferredIndex?: number | null) => {
    const sequenceDerivedIndex = resolveTeamSequenceInsertIndex({
      messages: params.agentMessages.value,
      teamSequence: message.metadata?.team_sequence,
      isTeamScopedMainFlowMessage: params.isTeamScopedMainFlowMessage,
    })
    const candidateIndex = preferredIndex ?? sequenceDerivedIndex
    const targetIndex = typeof candidateIndex === 'number' && Number.isFinite(candidateIndex)
      ? Math.min(Math.max(Math.floor(candidateIndex), 0), params.agentMessages.value.length)
      : -1
    if (targetIndex >= 0) {
      params.agentMessages.value.splice(targetIndex, 0, message)
      return
    }
    params.agentMessages.value.push(message)
  }

  const upsertTeamToolCallToMainFlow = (upsert: {
    toolCallId?: string
    toolName?: string
    toolArgs?: unknown
    toolResult?: unknown
    success?: boolean
    timestamp?: number
    teamSequence?: number
    memberId?: string
    memberName?: string
    streamId?: string
    phase?: string
    mode: 'start' | 'result'
  }) => {
    upsertTeamToolCallInMainFlow({
      messages: params.agentMessages.value,
      activeTeamSessionId: params.activeTeamSessionId.value,
      teamStreamTempMessageByStreamId,
      isTeamScopedMainFlowMessage: params.isTeamScopedMainFlowMessage,
      insertMainFlowMessageAtPreferredIndex,
      appendMainFlowMessage: (message) => {
        params.agentMessages.value.push(message)
      },
      createId: () => crypto.randomUUID(),
      upsert,
    })
  }

  const clearTeamStreamDeltaFlushTimer = () => {
    if (!teamStreamDeltaFlushTimer) return
    clearTimeout(teamStreamDeltaFlushTimer)
    teamStreamDeltaFlushTimer = null
  }

  const flushTeamStreamDeltaBuffers = (streamId?: string) => {
    flushTeamStreamDeltaBuffersSupport({
      messages: params.agentMessages.value,
      teamActiveStreamIds,
      teamStreamDeltaBufferByStreamId,
      teamStreamTempMessageByStreamId,
      streamId,
    })
    if (teamStreamDeltaBufferByStreamId.size === 0) {
      clearTeamStreamDeltaFlushTimer()
    }
  }

  const scheduleTeamStreamDeltaFlush = () => {
    if (teamStreamDeltaFlushTimer) return
    teamStreamDeltaFlushTimer = setTimeout(() => {
      teamStreamDeltaFlushTimer = null
      flushTeamStreamDeltaBuffers()
    }, TEAM_STREAM_DELTA_FLUSH_INTERVAL_MS)
  }

  const upsertTeamStreamTempMessage = (
    streamId: string,
    memberId?: string,
    memberName?: string,
    options?: { streaming?: boolean },
  ) => upsertTeamStreamTempMessageSupport({
    messages: params.agentMessages.value,
    activeTeamSessionId: params.activeTeamSessionId.value,
    teamActiveStreamIds,
    teamStreamTempMessageByStreamId,
    teamStreamSegmentSeqByStreamId,
    streamId,
    memberId,
    memberName,
    options,
    insertMainFlowMessageAtPreferredIndex,
    createId: () => crypto.randomUUID(),
  })

  const handleTeamMessageStreamStart = (payload: AgentTeamMessageStreamStartEvent) => {
    if (!payload?.session_id || payload.session_id !== params.activeTeamSessionId.value) return
    teamActiveStreamIds.add(payload.stream_id)
    upsertTeamStreamTempMessage(payload.stream_id, payload.member_id, payload.member_name, { streaming: true })
  }

  const handleTeamMessageStreamDelta = (payload: AgentTeamMessageStreamDeltaEvent) => {
    if (!payload?.session_id || payload.session_id !== params.activeTeamSessionId.value) return
    teamActiveStreamIds.add(payload.stream_id)
    upsertTeamStreamTempMessage(payload.stream_id, payload.member_id, payload.member_name, { streaming: true })
    if (!payload.delta) return
    const buffered = teamStreamDeltaBufferByStreamId.get(payload.stream_id) || ''
    teamStreamDeltaBufferByStreamId.set(payload.stream_id, `${buffered}${payload.delta}`)
    scheduleTeamStreamDeltaFlush()
  }

  const markTeamStreamDoneForReconcile = (tempMessageId: string, memberName: string | undefined, content: string) => {
    markTeamStreamDoneForReconcileSupport({
      teamStreamDoneIdsBySignature,
      tempMessageId,
      memberName,
      content,
    })
  }

  const splitTeamStreamAssistantSegmentAtToolBoundary = (payload: { stream_id: string; member_name?: string }) => {
    splitTeamStreamAssistantSegmentAtToolBoundarySupport({
      messages: params.agentMessages.value,
      teamActiveStreamIds,
      teamStreamDeltaBufferByStreamId,
      teamStreamDoneIdsBySignature,
      teamStreamTempMessageByStreamId,
      streamId: payload.stream_id,
      memberName: payload.member_name,
    })
  }

  const consumeTeamStreamTempForPersistedMessage = (msg: AgentTeamMessage): number | null =>
    consumeTeamStreamTempForPersistedMessageSupport({
      messages: params.agentMessages.value,
      teamStreamDoneIdsBySignature,
      teamStreamTempMessageByStreamId,
      msg,
    })

  const mirrorTeamMessageToConversation = async (msg: AgentTeamMessage) => {
    try {
      await mirrorTeamMessageToConversationSupport({
        msg,
        conversationId: params.conversationId.value,
        activeTeamSessionId: params.activeTeamSessionId.value,
        teamMirroredConversationMessageIds,
        shouldSuppressTeamMirrorNoiseMessage,
        persistMessage: async (request) => {
          await invoke('save_ai_message', { request })
        },
      })
    } catch (error) {
      console.warn('[useAgentTeamRuntime] Failed to mirror Team message to conversation:', error)
    }
  }

  const persistTeamToolEvent = async (
    messageType: 'tool_call' | 'tool_result',
    payload: AgentTeamToolCallEvent | AgentTeamToolResultEvent,
  ) => {
    try {
      await persistTeamToolEventSupport({
        messageType,
        payload,
        persistedToolEventKeys: teamPersistedToolEventKeys,
        messages: params.agentMessages.value,
        activeTeamSessionId: params.activeTeamSessionId.value,
        sendTeamMessage: async (sessionId, request) => {
          await invoke('team_v3_send_message', { sessionId, request })
        },
      })
    } catch (error) {
      console.warn(`[useAgentTeamRuntime] Failed to persist team ${messageType} event:`, error)
    }
  }

  const handleTeamMessageStreamDone = (payload: AgentTeamMessageStreamDoneEvent) => {
    if (!payload?.session_id || payload.session_id !== params.activeTeamSessionId.value) return
    flushTeamStreamDeltaBuffers(payload.stream_id)
    teamActiveStreamIds.delete(payload.stream_id)
    const tempMessageId = upsertTeamStreamTempMessage(
      payload.stream_id,
      payload.member_id,
      payload.member_name,
      { streaming: false },
    )
    const message = params.agentMessages.value.find((item) => item.id === tempMessageId)
    if (!message) return

    if (payload.error) {
      message.type = 'error'
      message.content = payload.error
    } else if ((payload.had_delta ?? false) === false && typeof payload.content === 'string') {
      message.content = payload.content
    }

    const finalContent = (message.content || '').trim()
    if (finalContent.length > 0 && message.type !== 'error') {
      const persistedCanonicalContent =
        typeof payload.content === 'string' && payload.content.trim().length > 0
          ? payload.content.trim()
          : finalContent
      const suppressionKey = buildTeamPersistedAssistantSuppressionKey(
        payload.session_id,
        payload.member_name,
        persistedCanonicalContent,
      )
      teamPersistedAssistantSuppressionKeys.add(suppressionKey)
      markTeamStreamDoneForReconcile(tempMessageId, payload.member_name, finalContent)
    } else {
      if (finalContent.length === 0 && message.type !== 'error') {
        const idx = params.agentMessages.value.findIndex((item) => item.id === tempMessageId)
        if (idx >= 0) params.agentMessages.value.splice(idx, 1)
      }
      teamStreamTempMessageByStreamId.delete(payload.stream_id)
    }

    void syncTeamMessagesToMainFlow(payload.session_id)
  }

  const handleTeamToolCall = (payload: AgentTeamToolCallEvent) => {
    if (!payload?.session_id || payload.session_id !== params.activeTeamSessionId.value) return
    splitTeamStreamAssistantSegmentAtToolBoundary(payload)
    upsertTeamToolCallToMainFlow({
      toolCallId: payload.tool_call_id,
      toolName: payload.name,
      toolArgs: payload.arguments,
      timestamp: parseTeamMessageTimestamp(payload.timestamp || ''),
      memberId: payload.member_id,
      memberName: payload.member_name,
      streamId: payload.stream_id,
      phase: payload.phase,
      mode: 'start',
    })
    void persistTeamToolEvent('tool_call', payload)
  }

  const handleTeamToolResult = (payload: AgentTeamToolResultEvent) => {
    if (!payload?.session_id || payload.session_id !== params.activeTeamSessionId.value) return
    upsertTeamToolCallToMainFlow({
      toolCallId: payload.tool_call_id,
      toolResult: payload.result,
      success: payload.success,
      timestamp: parseTeamMessageTimestamp(payload.timestamp || ''),
      memberId: payload.member_id,
      memberName: payload.member_name,
      streamId: payload.stream_id,
      phase: payload.phase,
      mode: 'result',
    })
    void persistTeamToolEvent('tool_result', payload)
  }

  const persistTeamSessionState = async (sessionId: string, nextState: string) => {
    try {
      await agentTeamApi.updateSession(sessionId, { state: nextState as any })
    } catch (error) {
      console.warn(`[useAgentTeamRuntime] Failed to persist team session state '${nextState}':`, error)
    }
  }

  const collectTeamMirroredSourceIds = async (sessionId: string): Promise<Set<string>> => {
    const rows = await invoke<any[]>('team_v3_list_messages', { sessionId })
    return collectTeamMirroredSourceIdsForAssistantSupport(rows)
  }

  const syncTeamMessagesToMainFlow = async (sessionId?: string | null) => {
    const sid = sessionId || params.activeTeamSessionId.value
    if (!sid) return
    if (isSyncingTeamMessages) {
      pendingTeamMessageSyncSessionId = sid
      return
    }
    isSyncingTeamMessages = true
    try {
      const result = await syncTeamMessagesToMainFlowSupport({
        activeTeamSessionId: params.activeTeamSessionId.value,
        sessionId: sid,
        getMessages: async (targetSessionId) => agentTeamApi.getMessages(targetSessionId),
      })
      if (!result.messages) return
      params.teamSessionMessages.value = result.messages
      appendTeamMessagesToMainFlow({
        messagesResp: result.messages,
        teamMainFlowMessageIds,
        teamMirroredConversationMessageIds,
        teamPersistedAssistantSuppressionKeys,
        pushMainFlowMessage: (message) => {
          params.agentMessages.value.push(message)
        },
        insertMainFlowMessageAtPreferredIndex,
        upsertTeamToolCallToMainFlow,
        mirrorTeamMessageToConversation,
        consumeTeamLocalHumanInputForPersistedMessage,
        consumeTeamStreamTempForPersistedMessage,
      })
    } catch (error) {
      console.warn('[useAgentTeamRuntime] Failed to sync team messages to main flow:', error)
    } finally {
      isSyncingTeamMessages = false
      const queued = pendingTeamMessageSyncSessionId
      pendingTeamMessageSyncSessionId = null
      if (queued && queued === params.activeTeamSessionId.value) {
        void syncTeamMessagesToMainFlow(queued)
      }
    }
  }

  const loadTeamWorkspaceData = async () => {
    await loadTeamWorkspaceDataSupport({
      activeTeamSessionId: params.activeTeamSessionId.value,
      blackboardFetchLimit: TEAM_BLACKBOARD_FETCH_LIMIT,
      ensureSchema: () => agentTeamApi.ensureSchema(),
      getMessages: (sessionId) => agentTeamApi.getMessages(sessionId),
      getSession: (sessionId) => agentTeamApi.getSession(sessionId),
      listBlackboardEntries: (sessionId, limit) => agentTeamApi.listBlackboardEntries(sessionId, limit),
      listTasks: (sessionId) => agentTeamApi.listTasks(sessionId),
      onEmptySession: () => {
        params.teamSessionMessages.value = []
        params.teamSessionDetail.value = null
        params.teamTasks.value = []
        params.selectedTeamTaskId.value = null
        params.teamBlackboardEntries.value = []
        params.teamSelectedOrchestrationPresetId.value = null
        params.teamSelectedRecoveryPresetId.value = 'balanced'
      },
      onSnapshotLoaded: (snapshot) => {
        params.teamSessionDetail.value = snapshot.sessionDetail
        params.teamSessionMessages.value = snapshot.sessionMessages
        params.teamTasks.value = snapshot.tasks
        params.teamBlackboardEntries.value = snapshot.blackboardEntries
        params.onTeamWorkspaceSnapshotLoaded?.()
      },
      setWorkspaceLoading: (loading) => {
        params.teamWorkspaceLoading.value = loading
      },
      syncTeamOrchestrationEditorFromSession: () => {
        params.onTeamWorkspaceSnapshotLoaded?.()
      },
    })
  }

  const refreshTeamRuntimeData = async (sessionId: string) => {
    const snapshot = await refreshTeamRuntimeDataSupport({
      activeTeamSessionId: params.activeTeamSessionId.value,
      includeBlackboard: params.isTeamWorkspaceActive.value,
      listBlackboardEntries: async () =>
        agentTeamApi.listBlackboardEntries(sessionId, TEAM_BLACKBOARD_FETCH_LIMIT),
      listTasks: async () => agentTeamApi.listTasks(sessionId),
      loadSession: async () => agentTeamApi.getSession(sessionId),
      sessionId,
    })
    if (!snapshot) return
    if (snapshot.sessionDetail) params.teamSessionDetail.value = snapshot.sessionDetail
    if (snapshot.tasks) {
      params.teamTasks.value = snapshot.tasks
    }
    if (snapshot.blackboardEntries) params.teamBlackboardEntries.value = snapshot.blackboardEntries
  }

  const applyTeamState = (nextState: string) => {
    const result = applyTeamStateChange(params.teamSessionState.value, nextState)
    if (!result.changed) return false
    params.teamSessionState.value = result.normalized
    if (result.requiresHumanPrompt) {
      appendTeamBridgeMessage('[Team] 需要人工介入，请继续输入指导意见。')
    }
    return true
  }

  const stopTeamRunStatusPolling = () => {
    if (teamRunStatusPollTimer) {
      clearInterval(teamRunStatusPollTimer)
      teamRunStatusPollTimer = null
    }
    isPollingTeamRunStatus = false
  }

  const pollTeamRunStatusOnce = async () => {
    if (isPollingTeamRunStatus) return
    isPollingTeamRunStatus = true
    try {
      await pollTeamRunStatusOnceSupport({
        activeTeamSessionId: params.activeTeamSessionId.value,
        applyTeamState,
        getRunStatus: async (sessionId) => agentTeamApi.getRunStatus(sessionId),
        loadTeamWorkspaceData: async () => {
          await loadTeamWorkspaceData()
        },
        refreshTeamRuntimeData: async (sessionId) => {
          await refreshTeamRuntimeData(sessionId)
        },
        syncTeamMessagesToMainFlow: async (sessionId) => {
          await syncTeamMessagesToMainFlow(sessionId)
        },
        teamModeEnabled: params.teamModeEnabled.value,
        teamWorkspaceActive: params.isTeamWorkspaceActive.value,
      })
    } catch (error) {
      console.warn('[useAgentTeamRuntime] Failed to poll team run status:', error)
    } finally {
      isPollingTeamRunStatus = false
    }
  }

  const ensureTeamRunStatusPolling = () => {
    if (!params.teamModeEnabled.value || !params.activeTeamSessionId.value) {
      stopTeamRunStatusPolling()
      return
    }
    const normalized = String(params.teamSessionState.value || '').trim().toUpperCase()
    if (!TEAM_RUNNING_STATES.has(normalized)) {
      stopTeamRunStatusPolling()
      return
    }
    if (teamRunStatusPollTimer) return
    void pollTeamRunStatusOnce()
    teamRunStatusPollTimer = setInterval(() => {
      void pollTeamRunStatusOnce()
    }, TEAM_RUN_STATUS_POLL_INTERVAL_MS)
  }

  const syncActiveTeamSession = async () => {
    if (!params.conversationId.value) {
      params.activeTeamSessionId.value = null
      params.teamSessionState.value = 'PENDING'
      return
    }
    try {
      await agentTeamApi.ensureSchema()
      const sessions = await agentTeamApi.listSessions(params.conversationId.value, 20, 0)
      const candidate = pickActiveTeamSession(
        sessions,
        getPersistedTeamSessionId(params.conversationId.value),
      )
      params.activeTeamSessionId.value = candidate?.id || null
      params.teamSessionState.value = candidate?.state || 'PENDING'
      ensureTeamRunStatusPolling()
      if (params.isTeamWorkspaceActive.value) {
        await loadTeamWorkspaceData()
      }
    } catch (error) {
      console.warn('[useAgentTeamRuntime] Failed to sync team session:', error)
      params.activeTeamSessionId.value = null
      params.teamSessionState.value = 'PENDING'
      stopTeamRunStatusPolling()
    }
  }

  const handleToggleTeamMode = async (enabled: boolean) => {
    if (!enabled && isTeamRunActive.value) {
      appendTeamBridgeMessage('[Team] 正在运行，停止后才能关闭 Team 模式。')
      return
    }
    params.teamModeEnabled.value = enabled
    if (enabled) {
      await syncActiveTeamSession()
      ensureTeamRunStatusPolling()
      if (params.isTeamWorkspaceActive.value) {
        await loadTeamWorkspaceData()
      }
      return
    }
    stopTeamRunStatusPolling()
    params.isTeamWorkspaceActive.value = false
    params.selectedTeamTaskId.value = null
  }

  const ensureConversationForTeamSession = async () =>
    ensureConversationForTeamSessionSupport({
      conversationId: params.conversationId.value,
      conversationBinding: params.buildCurrentConversationBinding(),
      createConversation: async (request) => invoke<string>('create_ai_conversation', { request }),
      getConversationTitle: params.getNewConversationTitle,
      getDisplayTitle: params.getDisplayConversationTitle,
      loadConversationList: params.loadConversationList,
      onConversationReady: (convId, title) => {
        params.conversationId.value = convId
        params.currentConversationTitle.value = title
        params.markConversationExecutionPending()
      },
      syncActiveTeamSession,
    })

  const createAndStartTeamSession = async (goal: string) =>
    createAndStartTeamSessionSupport({
      conversationId: params.conversationId.value,
      goal,
      createSession: (request) => agentTeamApi.createSession(request),
      loadTeamWorkspaceData: async () => {
        await loadTeamWorkspaceData()
      },
      onSessionReady: (session) => {
        params.activeTeamSessionId.value = session.id
        params.teamSessionState.value = session.state
      },
      pushTeamHumanInputLocalEcho,
      submitMessage: (request) => agentTeamApi.submitMessage(request),
      syncTeamMessagesToMainFlow: async (sessionId) => {
        await syncTeamMessagesToMainFlow(sessionId)
      },
      teamWorkspaceActive: params.isTeamWorkspaceActive.value,
    })

  const routeTeamMessage = async (content: string) =>
    routeTeamMessageSupport({
      activeTeamSessionId: params.activeTeamSessionId.value,
      content,
      createAndStartTeamSession: async (goal) => createAndStartTeamSession(goal),
      getSession: (sessionId) => agentTeamApi.getSession(sessionId),
      loadTeamWorkspaceData: async () => {
        await loadTeamWorkspaceData()
      },
      onSessionStateChange: (state) => {
        params.teamSessionState.value = state
      },
      pushTeamHumanInputLocalEcho,
      submitMessage: (request) => agentTeamApi.submitMessage(request),
      syncTeamMessagesToMainFlow: async (sessionId) => {
        await syncTeamMessagesToMainFlow(sessionId)
      },
      teamWorkspaceActive: params.isTeamWorkspaceActive.value,
    })

  const startTeamExecutionRun = async (bridgeMessage?: string) => {
    await startTeamExecutionRunSupport({
      appendTeamBridgeMessage,
      bridgeMessage,
      conversationId: params.conversationId.value,
      ensureTeamRunStatusPolling,
      flushPendingToolConfigSave: params.flushPendingToolConfigSave,
      loadTeamWorkspaceData: async () => {
        await loadTeamWorkspaceData()
      },
      persistTeamSessionState,
      ragEnabled: params.ragEnabled.value,
      runtimeToolConfig: buildRuntimeToolConfigForExecution(params.toolConfig.value, {
        webSearchEnabled: params.webSearchEnabled.value,
      }),
      sessionId: params.activeTeamSessionId.value,
      setTeamSessionState: (state) => {
        params.teamSessionState.value = state
      },
      startRun: (sessionId, convId, ragEnabledFlag, runtimeToolConfig) =>
        agentTeamApi.startRun(sessionId, convId, ragEnabledFlag, runtimeToolConfig),
      teamWorkspaceActive: params.isTeamWorkspaceActive.value,
    })
  }

  const runTeamExecutionFromWorkspace = async (bridgeMessage: string) => {
    if ((params.isExecuting.value || isTeamRunActive.value) && params.conversationId.value) {
      await params.handleStopExecution()
      await new Promise((resolve) => setTimeout(resolve, 300))
    }
    await ensureConversationForTeamSession()
    await startTeamExecutionRun(bridgeMessage)
  }

  const handleToggleTeamWorkspace = async () => {
    if (!params.teamWorkspaceAvailable.value) return
    if (params.activeRightPanel.value === 'team') {
      params.deactivateRightPanel('team')
      return
    }
    params.activateRightPanel('team')
    params.isTeamWorkspaceActive.value = true
    if (!params.activeTeamSessionId.value) {
      params.teamWorkspaceTab.value = 'tasks'
      return
    }
    await loadTeamWorkspaceData()
  }

  const mirrorAssistantOutputToTeamSession = async (
    sessionId: string,
    sourceMessageId: string,
    content: string,
  ) => {
    try {
      teamMirroredAssistantSourceIds = await mirrorAssistantOutputToTeamSessionSupport({
        collectPersistedSourceIds: collectTeamMirroredSourceIds,
        loadTeamWorkspaceData: async () => {
          await loadTeamWorkspaceData()
        },
        mirroredAssistantSourceIds: teamMirroredAssistantSourceIds,
        normalizedContent: (content || '').trim(),
        normalizedSourceMessageId: (sourceMessageId || '').trim(),
        sendTeamMessage: async (targetSessionId, request) => {
          await invoke('team_v3_send_message', { sessionId: targetSessionId, request })
        },
        sessionId,
        syncTeamMessagesToMainFlow: async (targetSessionId) => {
          await syncTeamMessagesToMainFlow(targetSessionId)
        },
        teamWorkspaceActive: params.isTeamWorkspaceActive.value,
      })
    } catch (error) {
      console.warn('[useAgentTeamRuntime] Failed to mirror assistant output to Team session:', error)
    }
  }

  const syncLatestAssistantOutputToTeamSession = async (sessionId: string, conversationId: string) => {
    try {
      await syncLatestAssistantOutputToTeamSessionSupport({
        conversationId,
        loadConversationMessages: async (targetConversationId) =>
          invoke<any[]>('get_ai_messages_by_conversation', { conversationId: targetConversationId }),
        mirrorAssistantOutputToTeamSession: async (targetSessionId, sourceMessageId, content) => {
          await mirrorAssistantOutputToTeamSession(targetSessionId, sourceMessageId, content)
        },
        sessionId,
      })
    } catch (error) {
      console.warn('[useAgentTeamRuntime] Failed to sync latest assistant output to Team session:', error)
    }
  }

  const handleTeamAssistantMessageSaved = async (payload: {
    execution_id: string
    message_id: string
    content: string
  }) => {
    const conversationId = params.conversationId.value
    const sessionId = params.activeTeamSessionId.value
    if (!params.teamModeEnabled.value || !conversationId || !sessionId) return
    if (payload.execution_id !== conversationId) return
    await mirrorAssistantOutputToTeamSession(sessionId, payload.message_id, payload.content)
  }

  const handleTeamExecutionFinished = async (payload: AgentExecutionFinishedEvent) => {
    const conversationId = params.conversationId.value
    const sessionId = params.activeTeamSessionId.value
    if (!params.teamModeEnabled.value || !conversationId || !sessionId) return
    if (payload.execution_id !== conversationId) return

    if (payload.outcome === 'succeeded') {
      await syncLatestAssistantOutputToTeamSession(sessionId, conversationId)
    }

    try {
      await agentTeamApi.finalizeRun(
        sessionId,
        payload.outcome === 'succeeded',
        payload.error || payload.message || undefined,
      )
    } catch (error) {
      console.warn('[useAgentTeamRuntime] Failed to finalize team run on execution finish:', error)
    }

    const nextState = payload.outcome === 'succeeded' ? 'PLAN_DRAFT' : 'FAILED'
    params.teamSessionState.value = nextState
    await persistTeamSessionState(sessionId, nextState)
    if (params.isTeamWorkspaceActive.value) {
      await loadTeamWorkspaceData()
    }
    stopTeamRunStatusPolling()
  }

  const resetActiveTeamRuntimeState = (nextSessionId?: string | null) => {
    params.selectedTeamTaskId.value = null
    teamMainFlowMessageIds = new Set<string>()
    teamMirroredAssistantSourceIds = new Set<string>()
    teamPersistedToolEventKeys.clear()
    teamStreamTempMessageByStreamId.clear()
    teamStreamSegmentSeqByStreamId.clear()
    teamStreamDoneIdsBySignature.clear()
    pruneTeamLocalHumanInputReconcileQueue(nextSessionId)
    teamStreamDeltaBufferByStreamId.clear()
    teamActiveStreamIds.clear()
    teamPersistedAssistantSuppressionKeys.clear()
    clearTeamStreamDeltaFlushTimer()
    pendingTeamMessageSyncSessionId = null
  }

  const setMirroredConversationMessageIds = (ids: Set<string>) => {
    teamMirroredConversationMessageIds = new Set(ids)
  }

  watch(params.activeTeamSessionId, async (newId, oldId) => {
    if (newId !== oldId) {
      resetActiveTeamRuntimeState(newId)
    }
    if (params.conversationId.value) {
      setPersistedTeamSessionId(params.conversationId.value, newId || null)
    }
    ensureTeamRunStatusPolling()
    if (newId) {
      await syncTeamMessagesToMainFlow(newId)
    }
    if (!params.isTeamWorkspaceActive.value) return
    await loadTeamWorkspaceData()
  })

  onUnmounted(() => {
    stopTeamRunStatusPolling()
    clearTeamStreamDeltaFlushTimer()
    resetActiveTeamRuntimeState(null)
  })

  return {
    activeTeamSessionId: params.activeTeamSessionId,
    appendTeamBridgeMessage,
    applyTeamState,
    ensureConversationForTeamSession,
    ensureTeamRunStatusPolling,
    handleTeamAssistantMessageSaved,
    handleTeamExecutionFinished,
    handleTeamMessageStreamDelta,
    handleTeamMessageStreamDone,
    handleTeamMessageStreamStart,
    handleTeamToolCall,
    handleTeamToolResult,
    handleToggleTeamMode,
    handleToggleTeamWorkspace,
    isTeamRunActive,
    isTeamWorkspaceActive: params.isTeamWorkspaceActive,
    loadTeamWorkspaceData,
    refreshTeamRuntimeData,
    routeTeamMessage,
    runTeamExecutionFromWorkspace,
    selectedTeamTaskId: params.selectedTeamTaskId,
    setMirroredConversationMessageIds,
    startTeamExecutionRun,
    stopTeamRunStatusPolling,
    syncActiveTeamSession,
    syncTeamMessagesToMainFlow,
    teamBlackboardEntries: params.teamBlackboardEntries,
    teamSessionDetail: params.teamSessionDetail,
    teamSessionMessages: params.teamSessionMessages,
    teamSessionState: params.teamSessionState,
    teamTasks: params.teamTasks,
    teamWorkspaceLoading: params.teamWorkspaceLoading,
  }
}
