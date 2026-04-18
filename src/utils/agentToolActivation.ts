export interface AgentToolsActivatedLikePayload {
  tool_ids?: unknown
  tools?: unknown
  query?: unknown
  runtime_hint?: unknown
  tools_preview?: unknown
}

const normalizeStringList = (value: unknown): string[] => {
  if (!Array.isArray(value)) return []
  return value
    .map((item) => (typeof item === 'string' ? item.trim() : ''))
    .filter((item) => item.length > 0)
}

const normalizeOptionalText = (value: unknown): string | undefined => {
  if (typeof value !== 'string') return undefined
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : undefined
}

export const buildToolsPreview = (tools: string[]): string => {
  const preview = tools.slice(0, 6).join(', ')
  const suffix = tools.length > 6 ? ` +${tools.length - 6}` : ''
  return `${preview}${suffix}`.trim()
}

export const normalizeToolsActivatedPayload = (payload: AgentToolsActivatedLikePayload) => {
  const toolIds = normalizeStringList(payload.tool_ids)
  const tools = normalizeStringList(payload.tools)
  const query = normalizeOptionalText(payload.query)
  const runtimeHint = normalizeOptionalText(payload.runtime_hint)
  const toolsPreview =
    normalizeOptionalText(payload.tools_preview) ||
    buildToolsPreview(tools)

  return {
    toolIds,
    tools,
    query,
    runtimeHint,
    toolsPreview,
  }
}

export const buildToolsActivatedMessage = (payload: AgentToolsActivatedLikePayload): string => {
  const { toolIds, toolsPreview, query, runtimeHint } = normalizeToolsActivatedPayload(payload)
  const activatedPreview = buildToolsPreview(toolIds)
  const firstLine = activatedPreview
    ? `Deferred tools activated: ${activatedPreview}`
    : 'Deferred tools activated'

  const lines = [firstLine]
  if (query) {
    lines.push(`Query: ${query}`)
  }
  if (toolsPreview) {
    lines.push(`Active toolset: ${toolsPreview}`)
  }
  if (runtimeHint) {
    lines.push(`Reason: ${runtimeHint}`)
  }

  return lines.join('\n')
}
