import type {
  LocationQuery,
  LocationQueryRaw,
  LocationQueryValue,
  LocationQueryValueRaw,
} from 'vue-router'

export interface FocusLocationState {
  conversationId: string | null
  memoryId: string | null
  focusedMessageId: string | null
}

export type QueryLike = LocationQuery | LocationQueryRaw

const normalizeQueryValue = (
  value: LocationQueryValue | LocationQueryValue[] | LocationQueryValueRaw | LocationQueryValueRaw[] | undefined,
): string | null => {
  if (Array.isArray(value)) {
    for (const entry of value) {
      const normalized = normalizeQueryValue(entry)
      if (normalized) return normalized
    }
    return null
  }

  if (typeof value !== 'string') return null
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : null
}

export const readFocusLocationState = (query: QueryLike): FocusLocationState => {
  const conversationId = normalizeQueryValue(query.conversationId) ?? normalizeQueryValue(query.conversation_id)
  return {
    conversationId,
    memoryId: normalizeQueryValue(query.memoryId),
    focusedMessageId: normalizeQueryValue(query.focusedMessageId),
  }
}

export const buildFocusedMessageQuery = (
  query: QueryLike,
  params: { memoryId: string; messageId: string },
): LocationQueryRaw => {
  const currentMemoryId = normalizeQueryValue(query.memoryId)
  const normalizedMemoryId = normalizeQueryValue(params.memoryId)
  const normalizedMessageId = normalizeQueryValue(params.messageId)

  if (!currentMemoryId || !normalizedMemoryId || !normalizedMessageId || currentMemoryId !== normalizedMemoryId) {
    return { ...query }
  }

  const nextQuery: LocationQueryRaw = { ...query }
  delete nextQuery.memoryId
  nextQuery.focusedMessageId = normalizedMessageId
  return nextQuery
}

export const clearFocusLocationQuery = (query: QueryLike): LocationQueryRaw => {
  const nextQuery: LocationQueryRaw = { ...query }
  delete nextQuery.memoryId
  delete nextQuery.focusedMessageId
  return nextQuery
}
