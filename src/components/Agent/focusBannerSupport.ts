export interface FocusBannerInput {
  focusedMemoryId?: string | null
  focusedMessageId?: string | null
  resolvedMessageId?: string | null
  lastFocusedMemoryId?: string | null
}

export interface FocusBannerState {
  visible: boolean
  memoryId: string
  messageId: string
  nextLastFocusedMemoryId: string | null
}

const normalizeText = (value: unknown): string => {
  if (typeof value !== 'string') return ''
  return value.trim()
}

export const deriveFocusBannerState = (input: FocusBannerInput): FocusBannerState => {
  const focusedMemoryId = normalizeText(input.focusedMemoryId)
  const focusedMessageId = normalizeText(input.focusedMessageId)
  const resolvedMessageId = normalizeText(input.resolvedMessageId)
  const lastFocusedMemoryId = normalizeText(input.lastFocusedMemoryId)

  const memoryId = focusedMemoryId || lastFocusedMemoryId
  const messageId = resolvedMessageId || focusedMessageId

  return {
    visible: Boolean(messageId || focusedMemoryId || focusedMessageId),
    memoryId,
    messageId,
    nextLastFocusedMemoryId: memoryId || null,
  }
}

export const buildFocusedMemoryToolsRoute = (memoryId: string) => {
  const normalized = normalizeText(memoryId)
  if (!normalized) return null

  return {
    name: 'McpTools',
    query: {
      tab: 'builtin_tools',
      memoryId: normalized,
    },
  }
}
