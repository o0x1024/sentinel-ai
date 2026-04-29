export const WEB_SEARCH_TOOL_ID = 'web_search'

export const LEGACY_SKILLS_TOOL_IDS = [
  'skills',
  'shell',
  'http_request',
  'spawn_agent',
  'wait_agents',
  'list_agents',
  'close_agent',
  'tenth_man_review',
  'memory',
  'tasks',
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

export type TeamToolPolicyRole = 'commander' | 'solver' | 'observer' | 'harness'

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
    fixed_tools: normalizeToolIdList(raw.fixed_tools ?? fallback?.fixed_tools),
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
  const fixedTools = normalizeToolIdList(config.fixed_tools)
  const manualFallback = normalizeToolIdList(config.manual_tools)
  const allowedTools = normalizeToolIdList(config.allowed_tools)
  const strategy = parseToolSelectionStrategy(config.selection_strategy, manualFallback)
  const runtimeSelectionStrategy = strategy.mode === 'Manual'
    ? { Manual: strategy.manualTools }
    : strategy.mode

  if (!webSearchEnabled) {
    return {
      enabled: config.enabled,
      selection_strategy: runtimeSelectionStrategy,
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
    selection_strategy: runtimeSelectionStrategy,
    max_tools: Math.max(1, Number(config.max_tools) || 1),
    fixed_tools: unionToolIds(fixedTools, [WEB_SEARCH_TOOL_ID]),
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
    normalizeToolIdList(baseRuntimeConfig.fixed_tools),
    baseManualTools,
    normalizeToolIdList(baseRuntimeConfig.allowed_tools),
  ).filter(toolId => !disabledTools.includes(toolId))
  const effectiveTools = roleTools.filter(toolId => agentToolScope.includes(toolId))

  return {
    enabled: baseRuntimeConfig.enabled && effectiveTools.length > 0,
    selection_strategy: { Manual: effectiveTools },
    max_tools: Math.max(Number(baseRuntimeConfig.max_tools) || 1, effectiveTools.length || 1),
    fixed_tools: [],
    disabled_tools: disabledTools,
    allowed_tools: effectiveTools,
  }
}
