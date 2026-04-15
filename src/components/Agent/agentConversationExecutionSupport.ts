import type { ProcessedDocumentResult } from '@/types/agent'
import type {
  ReferencedAsset,
  ReferencedConversationMessage,
  ReferencedFile,
  ReferencedTraffic,
} from '@/types/agentReferences'
import { useTerminal } from '@/composables/useTerminal'

export const takeOverConversationExecution = async (params: {
  appendPartialAssistantMessage: (message: {
    content: string
    id: string
    timestamp: number
  }) => void
  conversationId?: string | null
  createMessageId: () => string
  currentTime: () => number
  isExecuting: boolean
  savePartialAssistantMessage: (request: {
    content: string
    conversation_id: string
    id: string
    role: string
  }) => Promise<void>
  stopExecution: () => Promise<void>
  streamingContent?: string | null
  waitForStop: () => Promise<void>
}): Promise<void> => {
  if (!params.isExecuting || !params.conversationId) {
    return
  }

  console.log('[AgentView] Takeover: stopping current execution to handle new message')
  try {
    const partial = (params.streamingContent || '').trim()
    if (partial) {
      const partialMsgId = params.createMessageId()
      console.log('[AgentView] Takeover: saving partial response:', partial.substring(0, 100))
      params.appendPartialAssistantMessage({
        content: partial,
        id: partialMsgId,
        timestamp: params.currentTime(),
      })
      await params.savePartialAssistantMessage({
        content: partial,
        conversation_id: params.conversationId,
        id: partialMsgId,
        role: 'assistant',
      })
    }

    await params.stopExecution()
    await params.waitForStop()
    console.log('[AgentView] Takeover: previous execution stopped, proceeding with new message')
  } catch (e) {
    console.warn('[AgentView] Takeover stop failed, continuing:', e)
  }
}

export const ensureConversationForExecution = async (params: {
  conversationId?: string | null
  createConversation: (request: { service_name: string; title: string }) => Promise<string>
  getConversationTitle: () => string
  getDisplayTitle: () => string
  loadConversationList?: () => void
  onConversationReady: (conversationId: string, title: string) => void
}): Promise<string | null | undefined> => {
  if (params.conversationId) {
    return params.conversationId
  }

  console.log('[AgentView] No conversation ID, creating new conversation')
  const conversationId = await params.createConversation({
    service_name: 'default',
    title: params.getConversationTitle(),
  })
  params.onConversationReady(conversationId, params.getDisplayTitle())
  params.loadConversationList?.()
  console.log('[AgentView] Created new conversation:', conversationId)
  return conversationId
}

export const buildAssistantModelOverride = (
  assistantSelectedModel?: string | null,
): string | undefined => (
  assistantSelectedModel && assistantSelectedModel.includes('/')
    ? assistantSelectedModel
    : undefined
)

export const executeConversationTask = async (params: {
  assistantContextMode: 'claude-like' | 'codex-like'
  assistantSelectedModel?: string | null
  conversationId: string
  defaultConversationTitle: string
  displayContent?: string
  enableRag: boolean
  enableTenthManRule: boolean
  firstMessage: string
  forceTodos: boolean
  fullTask: string
  maybeAutoRenameConversation: (params: {
    convId: string
    currentConversationId: string
    defaultTitle: string
    firstMessage: string
    onConversationListRefresh: () => void
    onCurrentConversationTitleChange: (title: string) => void
  }) => void
  onConversationListRefresh: () => void
  onCurrentConversationTitleChange: (title: string) => void
  runAgentExecute: (request: {
    config: {
      attachments?: unknown[]
      conversation_id: string
      context_mode: 'claude-like' | 'codex-like'
      current_terminal_session_fingerprint?: string
      current_terminal_session_id?: string
      display_content?: string
      document_attachments?: ProcessedDocumentResult[]
      enable_rag: boolean
      enable_tenth_man_rule: boolean
      force_todos: boolean
      message_id: null
      model_override?: string
      referenced_assets?: ReferencedAsset[]
      referenced_files?: ReferencedFile[]
      referenced_messages?: ReferencedConversationMessage[]
      referenced_traffic?: ReferencedTraffic[]
      timeout_secs: number
      tool_config: unknown
    }
    task: string
  }) => Promise<any>
  runtimeToolConfig: unknown
  usedAssets: ReferencedAsset[]
  usedAttachments: unknown[]
  usedDocuments: ProcessedDocumentResult[]
  usedFiles: ReferencedFile[]
  usedMessages: ReferencedConversationMessage[]
  usedTraffic: ReferencedTraffic[]
}): Promise<any> => {
  const terminal = useTerminal()
  const currentTerminalSessionId = terminal.currentSessionId.value?.trim() || undefined
  const currentTerminalSessionFingerprint =
    terminal.currentSessionFingerprint.value?.trim() || undefined

  params.maybeAutoRenameConversation({
    convId: params.conversationId,
    currentConversationId: params.conversationId,
    defaultTitle: params.defaultConversationTitle,
    firstMessage: params.firstMessage,
    onConversationListRefresh: params.onConversationListRefresh,
    onCurrentConversationTitleChange: params.onCurrentConversationTitleChange,
  })

  return params.runAgentExecute({
    task: params.fullTask,
    config: {
      attachments: params.usedAttachments.length > 0 ? params.usedAttachments : undefined,
      conversation_id: params.conversationId,
      context_mode: params.assistantContextMode,
      current_terminal_session_fingerprint: currentTerminalSessionFingerprint,
      current_terminal_session_id: currentTerminalSessionId,
      display_content: params.displayContent,
      document_attachments: params.usedDocuments.length > 0 ? params.usedDocuments : undefined,
      enable_rag: params.enableRag,
      enable_tenth_man_rule: params.enableTenthManRule,
      force_todos: params.forceTodos,
      message_id: null,
      model_override: buildAssistantModelOverride(params.assistantSelectedModel),
      referenced_assets: params.usedAssets.length > 0 ? params.usedAssets : undefined,
      referenced_files: params.usedFiles.length > 0 ? params.usedFiles : undefined,
      referenced_messages: params.usedMessages.length > 0 ? params.usedMessages : undefined,
      referenced_traffic: params.usedTraffic.length > 0 ? params.usedTraffic : undefined,
      timeout_secs: 300,
      tool_config: params.runtimeToolConfig,
    },
  })
}
