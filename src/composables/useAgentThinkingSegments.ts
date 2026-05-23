import { invoke } from '@tauri-apps/api/core'
import type { Ref } from 'vue'
import type { AgentMessage } from '@/types/agent'
import type { AgentChunkEvent } from './useAgentEventTypes'
import { readGeneration } from './agentEventSupport'

export const useAgentThinkingSegments = (params: {
  currentThinkingMessageId: Ref<string | null>
  getTargetId: () => string | undefined
  messages: Ref<AgentMessage[]>
  thinkingBuffer: Ref<string>
  onAssistantSegmentBoundary: () => void
}) => {
  let segmentSequence = 0

  const persistThinkingSegment = (message: AgentMessage): void => {
    const metadata = message.metadata as any
    const conversationId = String(metadata?.conversation_id || params.getTargetId() || '').trim()
    const content = String(message.content || '').trim()
    if (!conversationId || !content) return

    void Promise.resolve(invoke('save_ai_message', {
      request: {
        id: message.id,
        conversation_id: conversationId,
        role: 'system',
        content,
        metadata: {
          ...(message.metadata || {}),
          kind: 'thinking_segment',
          status: 'complete',
        },
      },
    })).catch(error => {
      console.warn('[useAgentEvents] Failed to persist thinking segment:', error)
    })
  }

  const finalizeCurrentThinkingSegment = (): void => {
    const messageId = params.currentThinkingMessageId.value
    if (!messageId) return

    const existingMsg = params.messages.value.find(m => m.id === messageId)
    if (existingMsg) {
      existingMsg.metadata = {
        ...(existingMsg.metadata || {}),
        status: 'complete',
      } as any
      persistThinkingSegment(existingMsg)
    }

    params.currentThinkingMessageId.value = null
    params.thinkingBuffer.value = ''
  }

  const appendThinkingChunk = (payload: AgentChunkEvent): void => {
    params.onAssistantSegmentBoundary()
    params.thinkingBuffer.value += payload.content || ''

    if (params.currentThinkingMessageId.value) {
      const existingMsg = params.messages.value.find(
        m => m.id === params.currentThinkingMessageId.value
      )
      if (existingMsg) {
        existingMsg.content = params.thinkingBuffer.value
        existingMsg.metadata = {
          ...(existingMsg.metadata || {}),
          status: 'streaming',
        } as any
      }
      return
    }

    segmentSequence += 1
    const generation = readGeneration(payload)
    const msgId = crypto.randomUUID()
    params.currentThinkingMessageId.value = msgId
    params.messages.value.push({
      id: msgId,
      type: 'thinking',
      content: params.thinkingBuffer.value,
      timestamp: Date.now(),
      metadata: {
        kind: 'thinking_segment',
        status: 'streaming',
        execution_id: payload.execution_id,
        conversation_id: payload.conversation_id || params.getTargetId(),
        generation: generation || undefined,
        segment_id: `${payload.execution_id}:${generation || 0}:${segmentSequence}`,
      } as any,
    })
  }

  const resetThinkingSegmentSequence = (): void => {
    segmentSequence = 0
  }

  return {
    appendThinkingChunk,
    finalizeCurrentThinkingSegment,
    resetThinkingSegmentSequence,
  }
}
