import type {
  AgentMessage,
  PendingDocumentAttachment,
  ProcessedDocumentResult,
} from '@/types/agent'

export interface ReplaySubagentLike {
  startedAt?: number
}

export interface RestoredMessageDraftArtifacts {
  pendingAttachments: any[]
  pendingDocuments: PendingDocumentAttachment[]
  processedDocuments: ProcessedDocumentResult[]
}

export interface MessageReplaySnapshot<TSubagent extends ReplaySubagentLike> {
  messageIndex: number
  messageTimestamp: number
  messagesToKeep: AgentMessage[]
  subagentsToKeep: TSubagent[]
}

export const deleteConversationTailForReplay = async (params: {
  conversationId?: string | null
  deleteAiMessage: (messageId: string) => Promise<void>
  deleteAiMessagesAfter: (conversationId: string, messageId: string) => Promise<number>
  deleteSubagentRunsAfter: (parentExecutionId: string, afterTimestampMs: number) => Promise<number>
  logLabel: string
  messageId: string
  messageTimestamp: number
}): Promise<void> => {
  if (!params.conversationId) {
    return
  }

  try {
    const deletedCount = await params.deleteAiMessagesAfter(params.conversationId, params.messageId)
    console.log(`[AgentView] Deleted ${deletedCount} messages after the ${params.logLabel} message from database`)

    await params.deleteAiMessage(params.messageId)
    console.log(`[AgentView] Deleted the ${params.logLabel} message from database`)

    const deletedSubagents = await params.deleteSubagentRunsAfter(
      params.conversationId,
      params.messageTimestamp,
    )
    console.log(`[AgentView] Deleted ${deletedSubagents} subagent runs from database`)
  } catch (e) {
    console.error('[AgentView] Failed to delete messages from database:', e)
  }
}

const parseMetadataArray = (value: unknown): any[] => {
  if (Array.isArray(value)) return value
  if (typeof value === 'string') {
    try {
      const parsed = JSON.parse(value)
      return Array.isArray(parsed) ? parsed : []
    } catch {
      return []
    }
  }
  return []
}

export const restoreDraftArtifactsFromMessage = (
  message: AgentMessage,
): RestoredMessageDraftArtifacts => {
  const metadata = (message.metadata as any) || {}
  const pendingAttachments = parseMetadataArray(metadata.image_attachments)
  const rawDocumentAttachments = parseMetadataArray(metadata.document_attachments)
  const processedDocuments = rawDocumentAttachments.map((doc: any) => {
    const status = doc?.status === 'failed' || doc?.status === 'processing' || doc?.status === 'pending'
      ? doc.status
      : 'ready'
    return {
      ...doc,
      status,
    } as ProcessedDocumentResult
  })
  const pendingDocuments = processedDocuments.map((doc) => ({
    id: doc.id,
    file_id: doc.file_id,
    original_path: doc.file_path || doc.original_filename || '',
    original_filename: doc.original_filename,
    file_size: doc.file_size,
    mime_type: doc.mime_type,
    status: doc.status,
    file_path: doc.file_path,
    error_message: doc.error_message,
  }))

  return {
    pendingAttachments,
    pendingDocuments,
    processedDocuments,
  }
}

export const buildMessageReplaySnapshot = <TSubagent extends ReplaySubagentLike>(params: {
  message: AgentMessage
  messages: AgentMessage[]
  subagents: TSubagent[]
}): MessageReplaySnapshot<TSubagent> | null => {
  const messageIndex = params.messages.findIndex((item) => item.id === params.message.id)
  if (messageIndex === -1) {
    return null
  }

  const messageTimestamp = params.message.timestamp || Date.now()
  return {
    messageIndex,
    messageTimestamp,
    messagesToKeep: params.messages.slice(0, messageIndex),
    subagentsToKeep: params.subagents.filter((item) => {
      return !item.startedAt || item.startedAt <= messageTimestamp
    }),
  }
}
