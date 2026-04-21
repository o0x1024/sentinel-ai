import type { AgentMessage } from '@/types/agent'
import {
  buildToolsActivatedMessage,
  buildToolsPreview,
} from '@/utils/agentToolActivation'
import { applyFileVerificationStatuses } from './fileVerificationSupport'

export interface PersistedConversationMessageRow {
  id: string
  role?: string
  content?: string
  metadata?: unknown
  reasoning_content?: unknown
  structured_data?: unknown
  timestamp?: unknown
  tool_calls?: unknown
}

const parseMaybeJson = (value: unknown): any => {
  if (typeof value !== 'string') return value
  try {
    return JSON.parse(value)
  } catch {
    return value
  }
}

const toMillis = (value: unknown): number => {
  const ms = new Date(value as any).getTime()
  return Number.isFinite(ms) ? ms : Date.now()
}

const normalizeText = (value: unknown): string => {
  if (typeof value !== 'string') return ''
  return value.trim()
}

const collectExistingToolCallIds = (messages: PersistedConversationMessageRow[]): Set<string> => {
  const ids = new Set<string>()
  messages.forEach((row) => {
    if (row.role !== 'tool') return
    ids.add(row.id)
    const metadata = parseMaybeJson(row.metadata)
    if (metadata?.tool_call_id) {
      ids.add(String(metadata.tool_call_id))
    }
  })
  return ids
}

export const collectTeamMirroredConversationMessageIds = (
  messages: PersistedConversationMessageRow[]
): Set<string> => {
  return new Set(
    messages
      .map((row) => String(row?.id || ''))
      .filter((id) => id.startsWith('teamv3:'))
  )
}

export const buildConversationTimeline = (
  messages: PersistedConversationMessageRow[],
  options: {
    toolCallCompletedLabel: string
    shouldSuppressTeamMirrorNoiseMessage: (content: unknown) => boolean
  }
): AgentMessage[] => {
  const timeline: AgentMessage[] = []
  const existingToolCallIds = collectExistingToolCallIds(messages)

  messages.forEach((row) => {
    const parsedMetadata = parseMaybeJson(row.metadata)
    const parsedStructured = parseMaybeJson(row.structured_data)
    const ts = toMillis(row.timestamp)
    const reasoningContent = normalizeText(row.reasoning_content)
    const isTeamMirrorNoise =
      parsedMetadata?.kind === 'team_v3_mirror' &&
      options.shouldSuppressTeamMirrorNoiseMessage(row.content)

    if (isTeamMirrorNoise) {
      return
    }

    if (row.role === 'tool') {
      const kind = parsedMetadata?.kind
      const type = kind === 'tool_result' ? 'tool_result' : 'tool_call'
      timeline.push({
        id: row.id,
        type: type as any,
        content: row.content || '',
        timestamp: ts,
        metadata: parsedMetadata,
      })
      return
    }

    if (row.role === 'system') {
      if (parsedMetadata?.kind === 'agent_task_update') {
        return
      }

      if (parsedMetadata?.kind === 'skill_loaded') {
        const content =
          row.content ||
          `Skill loaded: ${parsedMetadata?.skill_name || 'unknown'} (${parsedMetadata?.skill_id || 'unknown'})`
        const restMetadata = { ...(parsedMetadata || {}) }
        delete restMetadata.tools
        delete restMetadata.tools_preview
        timeline.push({
          id: row.id,
          type: 'system' as any,
          content,
          timestamp: ts,
          metadata: restMetadata,
        })
        return
      }

      if (parsedMetadata?.kind === 'tools_activated') {
        const tools = Array.isArray(parsedMetadata?.tools) ? parsedMetadata.tools : []
        const toolIds = Array.isArray(parsedMetadata?.tool_ids) ? parsedMetadata.tool_ids : []
        const toolsPreview =
          parsedMetadata?.tools_preview ||
          buildToolsPreview(tools)
        timeline.push({
          id: row.id,
          type: 'system' as any,
          content: buildToolsActivatedMessage({
            ...parsedMetadata,
            tool_ids: toolIds,
            tools,
            tools_preview: toolsPreview,
          }) || row.content || 'Deferred tools activated',
          timestamp: ts,
          metadata: {
            ...parsedMetadata,
            tool_ids: toolIds,
            tools,
            tools_preview: toolsPreview,
          },
        })
        return
      }

      timeline.push({
        id: row.id,
        type: 'system' as any,
        content: row.content || '',
        timestamp: ts,
        metadata: parsedMetadata,
      })
      return
    }

    if (row.role === 'assistant' && row.tool_calls) {
      try {
        const toolCalls = parseMaybeJson(row.tool_calls)
        if (Array.isArray(toolCalls) && toolCalls.length > 0) {
          toolCalls.forEach((toolCall: any, index: number) => {
            if (toolCall.id && existingToolCallIds.has(String(toolCall.id))) {
              return
            }
            let parsedArgs: any = {}
            try {
              parsedArgs =
                typeof toolCall.arguments === 'string'
                  ? JSON.parse(toolCall.arguments)
                  : (toolCall.arguments ?? {})
            } catch {
              parsedArgs = { raw: toolCall.arguments }
            }
            timeline.push({
              id: `toolcall:${row.id}:${toolCall.id || index}`,
              type: 'tool_call' as any,
              content: `${options.toolCallCompletedLabel}: ${toolCall.name || 'unknown'}`,
              timestamp: ts,
              metadata: {
                tool_name: toolCall.name,
                tool_args: parsedArgs,
                tool_result: toolCall.result,
                tool_call_id: toolCall.id,
                status: 'completed',
                success: toolCall.success !== false,
              },
            })
          })
        }
      } catch (error) {
        console.warn('[AgentView] Failed to parse legacy tool_calls:', error)
      }
    }

    if (row.role === 'assistant' && reasoningContent) {
      timeline.push({
        id: `thinking:${row.id}`,
        type: 'thinking',
        content: reasoningContent,
        timestamp: ts,
        metadata: parsedMetadata,
      })
    }

    const messageType = row.role === 'user' ? 'user' : 'final'
    const displayContent =
      row.role === 'user' && parsedStructured?.display_content
        ? parsedStructured.display_content
        : row.content
    const normalizedDisplayContent = normalizeText(displayContent)

    let finalMetadata = parsedMetadata || {}
    if (row.role === 'user') {
      const documentAttachments =
        parsedMetadata?.document_attachments || parsedStructured?.document_attachments
      if (documentAttachments && Array.isArray(documentAttachments) && documentAttachments.length > 0) {
        finalMetadata = { ...finalMetadata, document_attachments: documentAttachments }
      }
      const referencedFiles =
        parsedMetadata?.referenced_files || parsedStructured?.referenced_files
      if (referencedFiles && Array.isArray(referencedFiles) && referencedFiles.length > 0) {
        finalMetadata = { ...finalMetadata, referenced_files: referencedFiles }
      }
      const referencedMessages =
        parsedMetadata?.referenced_messages || parsedStructured?.referenced_messages
      if (referencedMessages && Array.isArray(referencedMessages) && referencedMessages.length > 0) {
        finalMetadata = { ...finalMetadata, referenced_messages: referencedMessages }
      }
      const referencedAssets =
        parsedMetadata?.referenced_assets || parsedStructured?.referenced_assets
      if (referencedAssets && Array.isArray(referencedAssets) && referencedAssets.length > 0) {
        finalMetadata = { ...finalMetadata, referenced_assets: referencedAssets }
      }
      const referencedTraffic =
        parsedMetadata?.referenced_traffic || parsedStructured?.referenced_traffic
      if (referencedTraffic && Array.isArray(referencedTraffic) && referencedTraffic.length > 0) {
        finalMetadata = { ...finalMetadata, referenced_traffic: referencedTraffic }
      }
    }

    if (row.role === 'assistant' && !normalizedDisplayContent) {
      return
    }

    timeline.push({
      id: row.id,
      type: messageType as any,
      content: normalizedDisplayContent,
      timestamp: ts,
      metadata: finalMetadata,
    })
  })

  return applyFileVerificationStatuses(timeline)
}
