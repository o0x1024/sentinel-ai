export const WEB_SEARCH_TOOL_ID = 'web_search'
export const TENTH_MAN_REVIEW_TOOL_ID = 'tenth_man_review'

export interface UiToolConfigPayload {
  enabled: boolean
  selection_strategy: any
  max_tools: number
  preselected_tools: string[]
  disabled_tools: string[]
  manual_tools?: string[]
  allowed_tools?: string[]
}

export type TeamToolPolicyRole = 'orchestrator' | 'specialist' | 'monitor' | 'harness'
export type ParsedToolSelectionStrategy = {
  mode: string
  manualTools: string[]
}

const TOOL_SELECTION_STRATEGIES = new Set(['Keyword', 'LLM', 'Hybrid', 'Manual', 'All', 'Deferred'])

const dedupeToolIds = (items: string[]) => {
  const seen = new Set<string>()
  const out: string[] = []

  for (const item of items) {
    let normalized = item.trim().replace(/::/g, '__')
    if (normalized === 'interactive_shell') normalized = 'shell'
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
): ParsedToolSelectionStrategy => {
  if (strategyRaw && typeof strategyRaw === 'object' && !Array.isArray(strategyRaw)) {
    const strategyObj = strategyRaw as Record<string, unknown>
    if (Array.isArray(strategyObj.Manual)) {
      return { mode: 'Manual', manualTools: normalizeToolIdList(strategyObj.Manual) }
    }
  }

  if (typeof strategyRaw === 'string') {
    const mode = strategyRaw.trim() || 'Keyword'
    if (mode === 'Manual') {
      return { mode, manualTools: normalizeToolIdList(fallbackManualTools) }
    }
    if (!TOOL_SELECTION_STRATEGIES.has(mode)) {
      return { mode: 'Keyword', manualTools: [] as string[] }
    }
    return { mode, manualTools: [] as string[] }
  }

  return { mode: 'Keyword', manualTools: [] as string[] }
}

export const normalizeUiToolConfigPayload = (
  configRaw: Partial<UiToolConfigPayload> | Record<string, unknown> | null | undefined,
  fallback?: UiToolConfigPayload,
): UiToolConfigPayload => {
  const raw = (configRaw && typeof configRaw === 'object')
    ? configRaw as Record<string, unknown>
    : {}
  const fallbackManualTools = normalizeToolIdList(raw.manual_tools)
  const strategy = parseToolSelectionStrategy(
    raw.selection_strategy ?? fallback?.selection_strategy,
    fallbackManualTools,
  )
  const normalizedSelectionStrategy = strategy.mode === 'Manual'
    ? { Manual: strategy.manualTools }
    : strategy.mode

  return {
    enabled: typeof raw.enabled === 'boolean' ? raw.enabled : fallback?.enabled === true,
    selection_strategy: normalizedSelectionStrategy,
    max_tools: Math.max(1, Math.floor(
      Number(raw.max_tools)
      || Number(fallback?.max_tools)
      || 1,
    )),
    preselected_tools: normalizeToolIdList(raw.preselected_tools ?? fallback?.preselected_tools),
    disabled_tools: normalizeToolIdList(raw.disabled_tools ?? fallback?.disabled_tools),
    allowed_tools: normalizeToolIdList(raw.allowed_tools ?? fallback?.allowed_tools),
  }
}

const unionToolIds = (...groups: Array<string[] | undefined>) => {
  return dedupeToolIds(groups.flatMap((group) => group || []))
}

const readTeamRoleToolPolicy = (
  toolPolicyMatrix: Record<string, any>,
  role: TeamToolPolicyRole,
) => {
  const rolePolicy = toolPolicyMatrix[role]
  if (!rolePolicy || typeof rolePolicy !== 'object' || Array.isArray(rolePolicy)) {
    throw new Error(`Team tool policy matrix requires a ${role} policy.`)
  }
  return rolePolicy as Record<string, any>
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
  const preselectedTools = normalizeToolIdList(config.preselected_tools)
  const manualFallback = normalizeToolIdList(config.manual_tools)
  const allowedTools = normalizeToolIdList(config.allowed_tools)
  const strategy = parseToolSelectionStrategy(config.selection_strategy, manualFallback)
  const runtimeSelectionStrategy = strategy.mode === 'Manual'
    ? { Manual: strategy.manualTools }
    : strategy.mode
  const manualAllowedTools = strategy.mode === 'Manual'
    ? unionToolIds(strategy.manualTools, preselectedTools)
    : allowedTools

  if (!webSearchEnabled) {
    return {
      enabled: config.enabled,
      selection_strategy: runtimeSelectionStrategy,
      max_tools: Math.max(1, Number(config.max_tools) || 1),
      preselected_tools: preselectedTools,
      disabled_tools: disabledTools,
      allowed_tools: manualAllowedTools,
    }
  }

  if (!config.enabled) {
    return {
      enabled: true,
      selection_strategy: { Manual: [WEB_SEARCH_TOOL_ID] },
      max_tools: 1,
      preselected_tools: [],
      disabled_tools: disabledTools,
      allowed_tools: [WEB_SEARCH_TOOL_ID],
    }
  }

  const nextAllowedTools = allowedTools.length > 0
    ? unionToolIds(allowedTools, [WEB_SEARCH_TOOL_ID])
    : []

  if (strategy.mode === 'Manual') {
    const manualTools = unionToolIds(strategy.manualTools, [WEB_SEARCH_TOOL_ID])
    return {
      enabled: true,
      selection_strategy: { Manual: manualTools },
      max_tools: Math.max(Number(config.max_tools) || 1, manualTools.length),
      preselected_tools: unionToolIds(preselectedTools, [WEB_SEARCH_TOOL_ID]),
      disabled_tools: disabledTools,
      allowed_tools: unionToolIds(manualAllowedTools, [WEB_SEARCH_TOOL_ID]),
    }
  }

  return {
    enabled: true,
    selection_strategy: runtimeSelectionStrategy,
    max_tools: Math.max(1, Number(config.max_tools) || 1),
    preselected_tools: unionToolIds(preselectedTools, [WEB_SEARCH_TOOL_ID]),
    disabled_tools: disabledTools,
    allowed_tools: nextAllowedTools,
  }
}

export const buildRuntimeToolConfigForTeamRole = (
  config: UiToolConfigPayload,
  toolPolicyMatrix: Record<string, any>,
  role: TeamToolPolicyRole,
  options?: {
    webSearchEnabled?: boolean
  },
) => {
  const baseRuntimeConfig = buildRuntimeToolConfigForExecution(config, options)
  const rolePolicy = readTeamRoleToolPolicy(toolPolicyMatrix, role)
  const roleTools = normalizeToolIdList(rolePolicy.tools)
  const disabledTools = normalizeToolIdList(baseRuntimeConfig.disabled_tools)
  const baseStrategy = baseRuntimeConfig.selection_strategy
  const baseManualTools = baseStrategy
    && typeof baseStrategy === 'object'
    && !Array.isArray(baseStrategy)
    && Array.isArray(baseStrategy.Manual)
      ? normalizeToolIdList(baseStrategy.Manual)
      : []
  const agentToolScope = unionToolIds(
    normalizeToolIdList(baseRuntimeConfig.preselected_tools),
    baseManualTools,
    normalizeToolIdList(baseRuntimeConfig.allowed_tools),
  ).filter(toolId => !disabledTools.includes(toolId))
  const hasExplicitAgentToolScope = agentToolScope.length > 0
  const effectiveTools = hasExplicitAgentToolScope
    ? roleTools.filter(toolId => agentToolScope.includes(toolId))
    : roleTools.filter(toolId => !disabledTools.includes(toolId))

  return {
    enabled: baseRuntimeConfig.enabled && effectiveTools.length > 0,
    selection_strategy: { Manual: effectiveTools },
    max_tools: Math.max(Number(baseRuntimeConfig.max_tools) || 1, effectiveTools.length || 1),
    preselected_tools: [],
    disabled_tools: disabledTools,
    allowed_tools: effectiveTools,
  }
}

export const runtimeToolConfigAllowsTool = (
  runtimeConfigRaw: unknown,
  toolId: string,
): boolean => {
  if (!runtimeConfigRaw || typeof runtimeConfigRaw !== 'object' || Array.isArray(runtimeConfigRaw)) {
    return false
  }

  const runtimeConfig = runtimeConfigRaw as Record<string, unknown>
  if (runtimeConfig.enabled !== true) {
    return false
  }

  const normalizedToolId = normalizeToolIdList([toolId])[0]
  if (!normalizedToolId) {
    return false
  }

  const disabledTools = normalizeToolIdList(runtimeConfig.disabled_tools)
  if (disabledTools.includes(normalizedToolId)) {
    return false
  }

  const allowedTools = normalizeToolIdList(runtimeConfig.allowed_tools)
  if (allowedTools.length > 0) {
    return allowedTools.includes(normalizedToolId)
  }

  const preselectedTools = normalizeToolIdList(runtimeConfig.preselected_tools)
  const strategy = parseToolSelectionStrategy(
    runtimeConfig.selection_strategy,
    normalizeToolIdList(runtimeConfig.manual_tools),
  )
  if (strategy.mode === 'Manual') {
    return unionToolIds(strategy.manualTools, preselectedTools).includes(normalizedToolId)
  }

  return true
}
