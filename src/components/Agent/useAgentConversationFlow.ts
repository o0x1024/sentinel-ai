import { nextTick, type ComputedRef, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { agentTeamApi } from '@/api/agentTeam'
import type { AgentMessage } from '@/types/agent'
import { maybeAutoRenameConversationByFirstMessage } from './agentConversationTitleSupport'
import type { PersistedConversationMessageRow } from './agentConversationHistorySupport'
import { loadConversationHistory as loadConversationHistorySupport } from './agentConversationLoadSupport'
import {
  ensureConversationForExecution as ensureConversationForExecutionSupport,
  executeConversationTask as executeConversationTaskSupport,
  takeOverConversationExecution as takeOverConversationExecutionSupport,
} from './agentConversationExecutionSupport'
import {
  clearConversationSession,
  createConversationSession,
  loadLatestConversationSession,
  selectConversationSession,
} from './agentConversationSessionSupport'
import {
  buildMessageReplaySnapshot,
  deleteConversationTailForReplay,
} from './agentMessageReplaySupport'
import type { AiConversationDetail, AiConversationSummary } from './conversationTypes'
import type { AgentExecutionFinishedEvent, PersistedAgentExecutionState } from './executionState'
import { normalizeTeamHumanInputContent, shouldSuppressTeamMirrorNoiseMessage } from './agentTeamMessageSupport'
import { prepareSubmission } from './agentSubmissionSupport'
import { buildRuntimeToolConfigForExecution, type UiToolConfigPayload } from './toolConfigRuntime'

export const useAgentConversationFlow = (params: {
  activeTeamSessionId: Ref<string | null>
  agentMessages: Ref<AgentMessage[]>
  agentStreamingContent: Ref<string>
  agentSubagents: Ref<any[]>
  assistantContextMode: Ref<'claude-like' | 'codex-like'>
  assistantSelectedModel: Ref<string>
  buildToolConfig: () => UiToolConfigPayload
  clearAgentMessages: () => void
  clearDraftArtifacts: () => void
  clearTodosForCurrentContext: () => void
  closeConversationDrawer: () => void
  conversationExecutionState: Ref<PersistedAgentExecutionState | null>
  conversationId: Ref<string | null>
  currentConversationTitle: Ref<string>
  emitComplete: (payload: any) => void
  emitError: (message: string) => void
  emitSubmit: (task: string) => void
  ensureConversationForTeamSession: () => Promise<any>
  executionIdProp?: string | null
  forceTodos: boolean
  getFailedToClearConversationLabel: () => string
  getFailedToStopExecutionLabel: () => string
  getNewConversationTitle: () => string
  getToolCallCompletedLabel: () => string
  getUnnamedConversationTitle: () => string
  handleStopTeamState: (nextState: string) => void
  historyLoadToken: Ref<number>
  inputValue: Ref<string>
  isHistoryLoading: Ref<boolean>
  isExecuting: ComputedRef<boolean>
  isTeamModeEnabled: Ref<boolean>
  isToolConfigEnabled: Ref<boolean>
  loadConversationList: () => void
  loadSubagentRuns: (parentExecutionId: string, loadToken?: number) => Promise<void>
  localError: Ref<string | null>
  pendingAttachments: Ref<any[]>
  processedDocuments: Ref<any[]>
  ragEnabled: Ref<boolean>
  referencedAssets: Ref<any[]>
  referencedFiles: Ref<any[]>
  referencedTraffic: Ref<any[]>
  resetTerminal: () => void
  restoreArtifactsFromMessage: (message: AgentMessage) => void
  routeTeamMessage: (content: string) => Promise<any>
  scrollMessageViewportToBottom: () => void
  setMirroredConversationMessageIds: (ids: Set<string>) => void
  setPendingDocumentAttachments: (documents: any[]) => void
  startTeamExecutionRun: (bridgeMessage?: string) => Promise<void>
  stopAgentExecutionState: () => void
  submitInFlight: Ref<boolean>
  syncActiveTeamSession: () => Promise<void>
  syncTeamMessagesToMainFlow: (sessionId?: string | null) => Promise<void>
  teamModeEnabled: Ref<boolean>
  tenthManEnabled: Ref<boolean>
  webSearchEnabled: Ref<boolean>
}) => {
  const handleConversationExecutionStateUpdate = (payload: AgentExecutionFinishedEvent) => {
    if (payload.execution_id !== params.conversationId.value) return
    params.conversationExecutionState.value = {
      ...payload,
      completed_at: new Date().toISOString(),
    }
  }

  const applyReplaySnapshot = (message: AgentMessage) => {
    const snapshot = buildMessageReplaySnapshot({
      message,
      messages: params.agentMessages.value,
      subagents: params.agentSubagents.value,
    })
    if (!snapshot) {
      console.error('[useAgentConversationFlow] Message not found')
      return null
    }
    params.agentMessages.value = snapshot.messagesToKeep
    params.agentSubagents.value = snapshot.subagentsToKeep
    return snapshot
  }

  const deleteConversationTailForMessageReplay = async (
    message: AgentMessage,
    messageTimestamp: number,
    logLabel: string,
  ) => {
    await deleteConversationTailForReplay({
      conversationId: params.conversationId.value,
      deleteAiMessage: async (messageId) => {
        await invoke('delete_ai_message', { messageId })
      },
      deleteAiMessagesAfter: async (conversationId, messageId) =>
        invoke<number>('delete_ai_messages_after', { conversationId, messageId }),
      deleteSubagentRunsAfter: async (parentExecutionId, afterTimestampMs) =>
        invoke<number>('delete_subagent_runs_after', { parentExecutionId, afterTimestampMs }),
      logLabel,
      messageId: message.id,
      messageTimestamp,
    })
  }

  const handleClearConversation = async () => {
    try {
      if (!params.conversationId.value) {
        console.log('[useAgentConversationFlow] No conversation to clear')
        return
      }
      const cleared = await clearConversationSession({
        clearConversationMessages: async (conversationId) => {
          await invoke('clear_conversation_messages', { conversationId })
        },
        conversationId: params.conversationId.value,
        loadConversationList: params.loadConversationList,
        onConversationCleared: () => {
          params.clearAgentMessages()
          params.conversationExecutionState.value = null
          params.clearDraftArtifacts()
          params.inputValue.value = ''
        },
      })
      if (cleared) {
        console.log('[useAgentConversationFlow] Conversation cleared successfully')
      }
    } catch (error) {
      console.error('[useAgentConversationFlow] Failed to clear conversation:', error)
      params.localError.value = `${params.getFailedToClearConversationLabel()}: ${error}`
    }
  }

  const handleStop = async () => {
    console.log('[useAgentConversationFlow] Stop requested for conversation:', params.conversationId.value)

    if (params.teamModeEnabled.value && params.activeTeamSessionId.value) {
      try {
        await agentTeamApi.stopRun(params.activeTeamSessionId.value)
        params.handleStopTeamState('FAILED')
      } catch (error) {
        console.error('[useAgentConversationFlow] Failed to stop team execution:', error)
        params.localError.value = `${params.getFailedToStopExecutionLabel()}: ${error}`
      }
      return
    }

    if (!params.conversationId.value) {
      console.warn('[useAgentConversationFlow] No conversation ID to stop')
      return
    }

    try {
      await invoke('cancel_ai_stream', {
        conversationId: params.conversationId.value,
      })
      params.stopAgentExecutionState()
    } catch (error) {
      console.error('[useAgentConversationFlow] Failed to stop execution:', error)
      params.localError.value = `${params.getFailedToStopExecutionLabel()}: ${error}`
    }
  }

  const handleResendMessage = async (message: AgentMessage) => {
    if (params.isExecuting.value) return
    const snapshot = applyReplaySnapshot(message)
    if (!snapshot) return
    await deleteConversationTailForMessageReplay(message, snapshot.messageTimestamp, 'original')
    params.restoreArtifactsFromMessage(message)
    params.clearTodosForCurrentContext()
    params.inputValue.value = params.teamModeEnabled.value
      ? normalizeTeamHumanInputContent(message.content)
      : message.content
    await handleSubmit()
  }

  const handleEditMessage = async (message: AgentMessage, newContent: string) => {
    if (params.isExecuting.value) return
    const snapshot = applyReplaySnapshot(message)
    if (!snapshot) return
    await deleteConversationTailForMessageReplay(message, snapshot.messageTimestamp, 'edited')
    params.restoreArtifactsFromMessage(message)
    params.clearTodosForCurrentContext()
    params.inputValue.value = params.teamModeEnabled.value
      ? normalizeTeamHumanInputContent(newContent)
      : newContent
    await handleSubmit()
  }

  const loadConversationHistory = async (conversationId: string) => {
    const currentLoadToken = ++params.historyLoadToken.value
    params.isHistoryLoading.value = true

    try {
      await loadConversationHistorySupport({
        buildToolCallCompletedLabel: params.getToolCallCompletedLabel,
        clearMessages: params.clearAgentMessages,
        conversationId,
        getConversation: (targetConversationId) =>
          invoke<AiConversationDetail | null>('get_ai_conversation', {
            conversationId: targetConversationId,
          }),
        getMessages: (targetConversationId) =>
          invoke<PersistedConversationMessageRow[]>('get_ai_messages_by_conversation', {
            conversationId: targetConversationId,
          }),
        isStale: () => currentLoadToken !== params.historyLoadToken.value || params.conversationId.value !== conversationId,
        loadSubagentRuns: async () => {
          await params.loadSubagentRuns(conversationId, currentLoadToken)
        },
        log: (message, ...args) => {
          console.log(message, ...args)
        },
        onConversationLoaded: ({ executionState, title }) => {
          params.currentConversationTitle.value = title
          params.conversationExecutionState.value = executionState || null
        },
        onEmptyHistoryLoaded: async () => {
          params.setMirroredConversationMessageIds(new Set())
          await params.syncActiveTeamSession()
          if (currentLoadToken !== params.historyLoadToken.value || params.conversationId.value !== conversationId) return
          if (params.activeTeamSessionId.value) {
            await params.syncTeamMessagesToMainFlow(params.activeTeamSessionId.value)
            if (currentLoadToken !== params.historyLoadToken.value || params.conversationId.value !== conversationId) return
          }
        },
        onMessagesLoaded: async ({ messageCount, mirroredConversationMessageIds, timeline }) => {
          params.setMirroredConversationMessageIds(mirroredConversationMessageIds)
          params.agentMessages.value = timeline
          await params.syncActiveTeamSession()
          if (currentLoadToken !== params.historyLoadToken.value || params.conversationId.value !== conversationId) return
          if (params.activeTeamSessionId.value) {
            await params.syncTeamMessagesToMainFlow(params.activeTeamSessionId.value)
            if (currentLoadToken !== params.historyLoadToken.value || params.conversationId.value !== conversationId) return
          }
          console.log('[useAgentConversationFlow] Loaded', messageCount, 'messages from conversation:', conversationId)
          nextTick(() => {
            params.scrollMessageViewportToBottom()
          })
        },
        onLoadFailed: (error) => {
          console.error('[useAgentConversationFlow] Failed to load conversation history:', error)
        },
        shouldSuppressTeamMirrorNoiseMessage,
        unnamedConversationTitle: params.getUnnamedConversationTitle(),
      })
    } finally {
      if (currentLoadToken === params.historyLoadToken.value) {
        params.isHistoryLoading.value = false
      }
    }
  }

  const handleSelectConversation = async (conversationId: string) => {
    await selectConversationSession({
      closeConversationDrawer: params.closeConversationDrawer,
      conversationId,
      isStillActive: (currentId) => params.conversationId.value === currentId,
      loadConversationHistory,
      onConversationSelected: (selectedId) => {
        params.conversationId.value = selectedId
      },
      resetTerminal: params.resetTerminal,
    })
  }

  const handleCreateConversation = async (newConversationId?: string) => {
    if (newConversationId) {
      await handleSelectConversation(newConversationId)
      return
    }

    try {
      await createConversationSession({
        createConversation: async (request) =>
          invoke<string>('create_ai_conversation', { request }),
        getConversationTitle: params.getNewConversationTitle,
        getDisplayTitle: params.getUnnamedConversationTitle,
        loadConversationList: params.loadConversationList,
        onConversationCreated: (conversationId, title) => {
          params.conversationId.value = conversationId
          params.currentConversationTitle.value = title
          params.conversationExecutionState.value = null
          params.clearAgentMessages()
          params.resetTerminal()
        },
      })
    } catch (error) {
      console.error('[useAgentConversationFlow] Failed to create conversation:', error)
    }
  }

  const handleSubmit = async () => {
    if (params.submitInFlight.value) return
    const task = params.inputValue.value.trim()
    if (!task) return

    params.submitInFlight.value = true
    params.localError.value = null

    try {
      if (params.teamModeEnabled.value) {
        try {
          if (params.isExecuting.value && params.conversationId.value) {
            await handleStop()
            await new Promise((resolve) => setTimeout(resolve, 300))
          }

          await params.ensureConversationForTeamSession()
          const { fullTask } = prepareSubmission({
            clearDraftState: () => {
              params.inputValue.value = ''
              params.clearDraftArtifacts()
            },
            pendingAttachments: params.pendingAttachments.value,
            processedDocuments: params.processedDocuments.value,
            referencedAssets: params.referencedAssets.value,
            referencedFiles: params.referencedFiles.value,
            referencedTraffic: params.referencedTraffic.value,
            setPendingDocumentAttachments: (documents) => {
              params.setPendingDocumentAttachments(documents)
            },
            task,
            toAssetContextItems: (assets) => assets,
            toFileContextItems: (files) => files,
            toTrafficContextItems: (traffic) => traffic,
          })

          nextTick(() => {
            params.scrollMessageViewportToBottom()
          })

          params.emitSubmit(fullTask)
          await params.routeTeamMessage(fullTask)
          if (params.activeTeamSessionId.value) {
            await params.startTeamExecutionRun()
          }
          params.emitComplete({
            mode: 'team',
            session_id: params.activeTeamSessionId.value,
            execution_id: params.conversationId.value,
          })
        } catch (error: any) {
          const errorMsg = error?.toString?.() || String(error)
          params.localError.value = errorMsg
          params.emitError(errorMsg)
        }
        return
      }

      await takeOverConversationExecutionSupport({
        appendPartialAssistantMessage: (message) => {
          params.agentMessages.value.push({
            id: message.id,
            type: 'final' as any,
            content: message.content,
            timestamp: message.timestamp,
          })
        },
        conversationId: params.conversationId.value,
        createMessageId: () => crypto.randomUUID(),
        currentTime: () => Date.now(),
        isExecuting: params.isExecuting.value,
        savePartialAssistantMessage: async (request) => {
          await invoke('save_ai_message', { request })
        },
        stopExecution: async () => {
          await handleStop()
        },
        streamingContent: params.agentStreamingContent.value,
        waitForStop: async () => {
          await new Promise((resolve) => setTimeout(resolve, 500))
        },
      })

      const {
        displayContent,
        fullTask,
        usedAssets,
        usedAttachments,
        usedDocuments,
        usedFiles,
        usedTraffic,
      } = prepareSubmission({
        clearDraftState: () => {
          params.inputValue.value = ''
          params.clearDraftArtifacts()
        },
        pendingAttachments: params.pendingAttachments.value,
        processedDocuments: params.processedDocuments.value,
        referencedAssets: params.referencedAssets.value,
        referencedFiles: params.referencedFiles.value,
        referencedTraffic: params.referencedTraffic.value,
        setPendingDocumentAttachments: (documents) => {
          params.setPendingDocumentAttachments(documents)
        },
        task,
        toAssetContextItems: (assets) => assets,
        toFileContextItems: (files) => files,
        toTrafficContextItems: (traffic) => traffic,
      })

      nextTick(() => {
        params.scrollMessageViewportToBottom()
      })
      params.emitSubmit(fullTask)

      try {
        const ensuredConversationId = await ensureConversationForExecutionSupport({
          conversationId: params.conversationId.value,
          createConversation: async (request) =>
            invoke<string>('create_ai_conversation', { request }),
          getConversationTitle: params.getNewConversationTitle,
          getDisplayTitle: params.getUnnamedConversationTitle,
          loadConversationList: params.loadConversationList,
          onConversationReady: (conversationId, title) => {
            params.conversationId.value = conversationId
            params.currentConversationTitle.value = title
            params.conversationExecutionState.value = null
          },
        })
        if (!ensuredConversationId) {
          throw new Error('Conversation ID is required for agent execution.')
        }

        const result = await executeConversationTaskSupport({
          assistantContextMode: params.assistantContextMode.value,
          assistantSelectedModel: params.assistantSelectedModel.value,
          conversationId: ensuredConversationId,
          defaultConversationTitle: params.getUnnamedConversationTitle(),
          displayContent,
          enableRag: params.ragEnabled.value,
          enableTenthManRule: params.tenthManEnabled.value,
          firstMessage: task,
          forceTodos: params.forceTodos,
          fullTask,
          maybeAutoRenameConversation: (renameParams) => {
            void maybeAutoRenameConversationByFirstMessage(renameParams)
          },
          onConversationListRefresh: params.loadConversationList,
          onCurrentConversationTitleChange: (title) => {
            params.currentConversationTitle.value = title
          },
          runAgentExecute: (request) => invoke('agent_execute', request),
          runtimeToolConfig: buildRuntimeToolConfigForExecution(params.buildToolConfig(), {
            webSearchEnabled: params.webSearchEnabled.value,
          }),
          usedAssets,
          usedAttachments,
          usedDocuments,
          usedFiles,
          usedTraffic,
        })

        params.emitComplete(result)
      } catch (error: any) {
        const errorMsg = error.toString()
        params.localError.value = errorMsg
        params.emitError(errorMsg)
      }
    } finally {
      params.submitInFlight.value = false
    }
  }

  const loadLatestConversation = async () => {
    try {
      const latest = await loadLatestConversationSession({
        currentConversationId: params.conversationId.value,
        getConversations: () => invoke<AiConversationSummary[]>('get_ai_conversations'),
        loadConversationHistory,
        onConversationLoaded: (conversation) => {
          params.conversationId.value = conversation.id
          params.currentConversationTitle.value = conversation.title || params.getUnnamedConversationTitle()
        },
      })
      if (latest) {
        console.log('[useAgentConversationFlow] Loaded latest conversation:', latest.id)
      }
    } catch (error) {
      console.error('[useAgentConversationFlow] Failed to load latest conversation:', error)
    }
  }

  return {
    handleClearConversation,
    handleConversationExecutionStateUpdate,
    handleCreateConversation,
    handleEditMessage,
    handleResendMessage,
    handleSelectConversation,
    handleStop,
    handleSubmit,
    loadConversationHistory,
    loadLatestConversation,
  }
}
