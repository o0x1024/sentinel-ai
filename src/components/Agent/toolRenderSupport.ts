const SHELL_TOOL_NAMES = new Set([
  'shell',
  'interactive_shell',
  'exec_command',
  'write_stdin',
  'bash',
  'cmd',
  'powershell',
])
const FILE_TOOL_NAMES = new Set(['file_read', 'file_write', 'file_edit'])
const SEARCH_TOOL_NAMES = new Set(['glob', 'grep'])

const normalizeErrorText = (value: unknown): string => {
  if (typeof value === 'string') return value.trim().toLowerCase()
  if (value === null || value === undefined) return ''
  try {
    return JSON.stringify(value).trim().toLowerCase()
  } catch {
    return String(value).trim().toLowerCase()
  }
}

export const isShellLikeToolName = (toolName: unknown): boolean => {
  const normalized = String(toolName || '').trim().toLowerCase()
  return SHELL_TOOL_NAMES.has(normalized)
}

export const isFileToolName = (toolName: unknown): boolean => {
  const normalized = String(toolName || '').trim().toLowerCase()
  return FILE_TOOL_NAMES.has(normalized)
}

export const isSearchToolName = (toolName: unknown): boolean => {
  const normalized = String(toolName || '').trim().toLowerCase()
  return SEARCH_TOOL_NAMES.has(normalized)
}

export const shouldRenderSpecializedShellTool = (params: {
  toolName?: unknown
  result?: unknown
  error?: unknown
}): boolean => {
  if (!isShellLikeToolName(params.toolName)) {
    return false
  }

  const toolName = String(params.toolName || '').trim().toLowerCase()
  const combined = [
    normalizeErrorText(params.error),
    normalizeErrorText(params.result),
  ]
    .filter(Boolean)
    .join('\n')

  if (!combined.includes('toolnotfounderror')) {
    return true
  }

  if (combined.includes(`toolnotfounderror: ${toolName}`)) {
    return false
  }

  return toolName !== 'shell' || !combined.includes('toolnotfounderror: shell')
}

export const parseStructuredToolPayload = (raw: unknown): unknown => {
  if (typeof raw === 'string') {
    const trimmed = raw.trim()
    if (!trimmed) return ''
    try {
      return JSON.parse(trimmed)
    } catch {
      return raw
    }
  }

  if (Array.isArray(raw)) {
    const textPayload = raw
      .filter((item) => item && typeof item === 'object' && (item as Record<string, unknown>).type === 'text')
      .map((item) => (item as Record<string, unknown>).text)
      .find((value) => typeof value === 'string')
    if (typeof textPayload === 'string') {
      return parseStructuredToolPayload(textPayload)
    }
  }

  return raw
}
