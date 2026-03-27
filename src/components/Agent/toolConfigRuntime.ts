export const WEB_SEARCH_TOOL_ID = 'web_search'

export const LEGACY_SKILLS_TOOL_IDS = [
  'skills',
  'shell',
  'http_request',
  'subagent_execute',
  'subagent_await',
  'subagent_channel',
  'tenth_man_review',
  'memory',
  'todos',
]

export interface UiToolConfigPayload {
  enabled: boolean
  selection_strategy: any
  max_tools: number
  fixed_tools: string[]
  disabled_tools: string[]
  manual_tools?: string[]
  skills?: string[]
  allowed_tools?: string[]
}

const dedupeToolIds = (items: string[]) => {
  const seen = new Set<string>()
  const out: string[] = []

  for (const item of items) {
    const normalized = item.trim().replace(/::/g, '__')
    if (!normalized || seen.has(normalized)) continue
    seen.add(normalized)
    out.push(normalized)
  }

  return out
}

export const normalizeToolIdList = (items: unknown): string[] => {
  if (!Array.isArray(items)) return []
  return dedupeToolIds(items.filter((item): item is string => typeof item === 'string'))
}

export const parseToolSelectionStrategy = (
  strategyRaw: unknown,
  fallbackManualTools: string[],
) => {
  if (strategyRaw && typeof strategyRaw === 'object' && !Array.isArray(strategyRaw)) {
    const strategyObj = strategyRaw as Record<string, unknown>
    if (Array.isArray(strategyObj.Manual)) {
      return { mode: 'Manual', manualTools: normalizeToolIdList(strategyObj.Manual) }
    }
    if (Array.isArray(strategyObj.Skills)) {
      return { mode: 'Manual', manualTools: [...LEGACY_SKILLS_TOOL_IDS] }
    }
  }

  if (typeof strategyRaw === 'string') {
    const mode = strategyRaw.trim() || 'Keyword'
    if (mode === 'Skills') {
      return { mode: 'Manual', manualTools: [...LEGACY_SKILLS_TOOL_IDS] }
    }
    if (mode === 'Manual') {
      return { mode, manualTools: normalizeToolIdList(fallbackManualTools) }
    }
    return { mode, manualTools: [] as string[] }
  }

  return { mode: 'Keyword', manualTools: [] as string[] }
}

const unionToolIds = (...groups: Array<string[] | undefined>) => {
  return dedupeToolIds(groups.flatMap((group) => group || []))
}

export const buildRuntimeToolConfigForExecution = (
  config: UiToolConfigPayload,
  options?: {
    webSearchEnabled?: boolean
  },
) => {
  const webSearchEnabled = options?.webSearchEnabled === true
  const disabledTools = normalizeToolIdList(config.disabled_tools)
    .filter((toolId) => !(webSearchEnabled && toolId === WEB_SEARCH_TOOL_ID))
  const fixedTools = normalizeToolIdList(config.fixed_tools)
  const manualFallback = normalizeToolIdList(config.manual_tools)
  const allowedTools = normalizeToolIdList(config.allowed_tools)

  if (!webSearchEnabled) {
    return {
      enabled: config.enabled,
      selection_strategy: config.selection_strategy,
      max_tools: Math.max(1, Number(config.max_tools) || 1),
      fixed_tools: fixedTools,
      disabled_tools: disabledTools,
      allowed_tools: allowedTools,
    }
  }

  if (!config.enabled) {
    return {
      enabled: true,
      selection_strategy: { Manual: [WEB_SEARCH_TOOL_ID] },
      max_tools: 1,
      fixed_tools: [],
      disabled_tools: disabledTools,
      allowed_tools: [WEB_SEARCH_TOOL_ID],
    }
  }

  const strategy = parseToolSelectionStrategy(config.selection_strategy, manualFallback)
  const nextAllowedTools = allowedTools.length > 0
    ? unionToolIds(allowedTools, [WEB_SEARCH_TOOL_ID])
    : []

  if (strategy.mode === 'Manual') {
    const manualTools = unionToolIds(strategy.manualTools, [WEB_SEARCH_TOOL_ID])
    return {
      enabled: true,
      selection_strategy: { Manual: manualTools },
      max_tools: Math.max(Number(config.max_tools) || 1, manualTools.length),
      fixed_tools: unionToolIds(fixedTools, [WEB_SEARCH_TOOL_ID]),
      disabled_tools: disabledTools,
      allowed_tools: nextAllowedTools,
    }
  }

  return {
    enabled: true,
    selection_strategy: config.selection_strategy,
    max_tools: Math.max(1, Number(config.max_tools) || 1),
    fixed_tools: unionToolIds(fixedTools, [WEB_SEARCH_TOOL_ID]),
    disabled_tools: disabledTools,
    allowed_tools: nextAllowedTools,
  }
}
