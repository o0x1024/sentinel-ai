import type { AgentMessage } from '@/types/agent'

export const parseToolCallArguments = (value: unknown): Record<string, any> => {
  if (typeof value === 'string') {
    try {
      const parsed = JSON.parse(value)
      return parsed && typeof parsed === 'object' ? parsed : { raw: value }
    } catch {
      return { raw: value }
    }
  }
  if (value && typeof value === 'object') {
    return value as Record<string, any>
  }
  return {}
}

export const normalizeToolResult = (value: unknown): string | undefined => {
  if (value === undefined || value === null) return undefined
  if (typeof value === 'string') return value
  try {
    return JSON.stringify(value)
  } catch {
    return String(value)
  }
}

export const parseToolResultObject = (value: unknown): Record<string, any> | null => {
  if (!value) return null
  if (typeof value === 'object' && !Array.isArray(value)) {
    return value as Record<string, any>
  }
  if (typeof value !== 'string') return null
  try {
    const parsed = JSON.parse(value)
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      return parsed as Record<string, any>
    }
  } catch {
    return null
  }
  return null
}

export const buildShellFallbackNoticeFromResult = (resultValue: unknown): string | null => {
  const parsed = parseToolResultObject(resultValue)
  if (!parsed) return null
  const fallbackFrom = String(parsed.fallback_from || '').trim().toLowerCase()
  const executionMode = String(parsed.execution_mode || '').trim().toLowerCase()
  if (fallbackFrom !== 'docker' || executionMode !== 'host') return null
  const reason = String(parsed.fallback_reason || '').trim()
  if (reason) {
    return `系统提示: shell 工具在 Docker 中执行失败，已自动回退到宿主机。原因: ${reason}`
  }
  return '系统提示: shell 工具在 Docker 中执行失败，已自动回退到宿主机。'
}

export const inferTeamToolSuccess = (result: unknown): boolean => {
  if (typeof result !== 'string') return true
  const normalized = result.trim().toLowerCase()
  if (!normalized) return true
  if (normalized.startsWith('error:')) return false
  if (normalized.includes('"success":false') || normalized.includes('"ok":false')) return false
  return true
}

export const normalizeTeamSequence = (value: unknown): number | undefined => {
  const num = Number(value)
  if (!Number.isFinite(num)) return undefined
  const int = Math.floor(num)
  return int > 0 ? int : undefined
}

export const getTeamMessageSequence = (message: AgentMessage): number | undefined => {
  return normalizeTeamSequence(message.metadata?.team_sequence)
}

export const buildTeamMessageSignature = (
  role: string | undefined,
  memberName: string | undefined,
  content: string,
) => {
  return `${(role || '').trim().toLowerCase()}\u0001${(memberName || '').trim()}\u0001${content.trim()}`
}

export const buildTeamPersistedAssistantSuppressionKey = (
  sessionId: string | undefined,
  memberName: string | undefined,
  content: string,
) => {
  return `${(sessionId || '').trim()}\u0001${(memberName || '').trim()}\u0001${content.trim()}`
}

export const buildTeamPersistedToolEventKey = (
  sessionId: string | undefined,
  streamId: string | undefined,
  toolCallId: string | undefined,
  messageType: 'tool_call' | 'tool_result',
) => {
  return `${String(sessionId || '').trim()}\u0001${String(streamId || '').trim()}\u0001${String(toolCallId || '').trim()}\u0001${messageType}`
}

export const normalizeTeamHumanInputContent = (content: unknown): string => {
  return typeof content === 'string' ? content : ''
}

export const parseConversationMessageTimestamp = (raw: unknown): number => {
  if (typeof raw === 'number' && Number.isFinite(raw)) return raw
  const parsed = Date.parse(typeof raw === 'string' ? raw : '')
  return Number.isFinite(parsed) ? parsed : 0
}
