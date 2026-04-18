import type { AgentMessage } from '@/types/agent'

const normalizeText = (value: unknown): string => {
  if (typeof value !== 'string') return ''
  return value.trim()
}

export const parseMemoryToolPayload = (value: unknown, depth = 0): Record<string, any> | null => {
  if (depth > 3 || value == null) return null

  if (typeof value === 'string') {
    try {
      return parseMemoryToolPayload(JSON.parse(value), depth + 1)
    } catch {
      return null
    }
  }

  if (Array.isArray(value)) {
    const textItem = value.find((item: any) => item?.type === 'text' && item?.text)
    if (textItem?.text) {
      return parseMemoryToolPayload(textItem.text, depth + 1)
    }
    return null
  }

  if (typeof value !== 'object') return null

  const record = value as Record<string, unknown>
  if (typeof record.text === 'string') {
    const nested = parseMemoryToolPayload(record.text, depth + 1)
    if (nested) return nested
  }

  return record as Record<string, any>
}

export const extractMemoryIdsFromMessage = (message: AgentMessage): string[] => {
  if ((message.metadata?.tool_name || '').toLowerCase() !== 'memory') {
    return []
  }

  const payload = parseMemoryToolPayload(message.metadata?.tool_result)
  if (!payload) return []

  const memoryIds = new Set<string>()
  const storeId = normalizeText(payload.store?.memory_id)
  if (storeId) {
    memoryIds.add(storeId)
  }

  const items = Array.isArray(payload.items) ? payload.items : []
  for (const item of items) {
    const itemId = normalizeText(item?.id)
    if (itemId) {
      memoryIds.add(itemId)
    }
  }

  return Array.from(memoryIds)
}

export const resolveFocusedMemoryMessageId = (
  messages: AgentMessage[],
  memoryId: string,
): string | null => {
  const normalizedMemoryId = normalizeText(memoryId)
  if (!normalizedMemoryId) return null

  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index]
    if (extractMemoryIdsFromMessage(message).includes(normalizedMemoryId)) {
      return message.id
    }
  }

  return null
}
