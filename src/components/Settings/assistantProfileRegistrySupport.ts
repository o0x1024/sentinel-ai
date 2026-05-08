import {
  normalizeHarnessMaxContinuations,
  type AssistantProfileOption,
} from '@/components/Agent/assistantProfiles'
import type { UiToolConfigPayload } from '@/components/Agent/toolConfigRuntime'
import {
  TEAM_ORCHESTRATION_PRESET_METAS,
  TEAM_RECOVERY_PRESETS,
} from '@/components/Agent/teamOrchestrationSupport'

export const teamOrchestrationPresetOptions = TEAM_ORCHESTRATION_PRESET_METAS
export const teamRecoveryPresetOptions = TEAM_RECOVERY_PRESETS
export const toolSelectionStrategyOptions = ['Keyword', 'LLM', 'Hybrid', 'Manual', 'All']

export const parseToolIds = (raw: string) => {
  const seen = new Set<string>()
  const out: string[] = []
  for (const item of raw.split(/[\n,]/)) {
    const normalized = item.trim().replace(/::/g, '__')
    if (!normalized || seen.has(normalized)) continue
    seen.add(normalized)
    out.push(normalized)
  }
  return out
}

export const formatToolIds = (items: string[] | null | undefined) =>
  (Array.isArray(items) ? items : []).join('\n')

const normalizeToolIds = (items: string[] | null | undefined) => parseToolIds((items || []).join('\n'))

const parseToolSelectionStrategy = (
  strategy: unknown,
  fallbackManualTools: string[] = [],
) => {
  if (typeof strategy === 'string') {
    const normalizedStrategy = toolSelectionStrategyOptions.includes(strategy) ? strategy : 'Keyword'
    return {
      manualTools: normalizedStrategy === 'Manual' ? normalizeToolIds(fallbackManualTools) : [] as string[],
      strategy: normalizedStrategy,
    }
  }
  if (strategy && typeof strategy === 'object' && Array.isArray((strategy as any).Manual)) {
    return {
      manualTools: normalizeToolIds((strategy as any).Manual),
      strategy: 'Manual',
    }
  }
  return {
    manualTools: [] as string[],
    strategy: 'Keyword',
  }
}

export const profileToToolConfig = (profile: AssistantProfileOption): UiToolConfigPayload => ({
  enabled: profile.defaultToolsEnabled === true,
  selection_strategy: toolSelectionStrategyOptions.includes(profile.defaultToolSelectionStrategy || '')
    ? profile.defaultToolSelectionStrategy
    : 'Keyword',
  max_tools: Math.max(1, Math.floor(Number(profile.defaultMaxTools) || 1)),
  preselected_tools: normalizeToolIds(profile.defaultPreselectedTools),
  disabled_tools: normalizeToolIds(profile.defaultDisabledTools),
  manual_tools: normalizeToolIds(profile.defaultManualTools),
})

export const applyToolConfigToProfile = (
  profile: AssistantProfileOption,
  config: UiToolConfigPayload,
) => {
  const parsedStrategy = parseToolSelectionStrategy(
    config.selection_strategy,
    normalizeToolIds(config.manual_tools),
  )
  profile.defaultToolsEnabled = config.enabled === true
  profile.defaultToolSelectionStrategy = parsedStrategy.strategy
  profile.defaultMaxTools = Math.max(1, Math.floor(Number(config.max_tools) || 1))
  profile.defaultPreselectedTools = normalizeToolIds(config.preselected_tools)
  profile.defaultDisabledTools = normalizeToolIds(config.disabled_tools)
  profile.defaultManualTools = parsedStrategy.strategy === 'Manual'
    ? parsedStrategy.manualTools
    : []
}

export const createNextProfileIdentity = (profiles: AssistantProfileOption[]) => {
  const ids = new Set(profiles.map(profile => profile.id))
  let nextIndex = profiles.length + 1
  while (ids.has(`assistant.custom.${nextIndex}`)) {
    nextIndex += 1
  }
  return {
    id: `assistant.custom.${nextIndex}`,
    nextIndex,
  }
}

export const normalizeAssistantProfileDraft = (profile: AssistantProfileOption): AssistantProfileOption => ({
  ...profile,
  id: profile.id.trim(),
  label: profile.label.trim(),
  description: profile.description.trim(),
  defaultModel: profile.defaultModel?.trim() || null,
  defaultRagEnabled: profile.defaultRagEnabled === true,
  defaultWebSearchEnabled: profile.defaultWebSearchEnabled === true,
  defaultToolsEnabled: profile.defaultToolsEnabled === true,
  defaultTenthManEnabled: profile.defaultTenthManEnabled === true,
  defaultToolSelectionStrategy: toolSelectionStrategyOptions.includes(profile.defaultToolSelectionStrategy || '')
    ? profile.defaultToolSelectionStrategy
    : 'Keyword',
  defaultMaxTools: Math.max(1, Math.floor(Number(profile.defaultMaxTools) || 1)),
  defaultHarnessMaxContinuations: normalizeHarnessMaxContinuations(profile.defaultHarnessMaxContinuations),
  defaultPreselectedTools: normalizeToolIds(profile.defaultPreselectedTools),
  defaultDisabledTools: normalizeToolIds(profile.defaultDisabledTools),
  defaultManualTools: normalizeToolIds(profile.defaultManualTools),
  defaultTeamOrchestrationPresetId: profile.defaultTeamOrchestrationPresetId?.trim() || null,
  defaultTeamRecoveryPresetId: profile.defaultTeamRecoveryPresetId?.trim() || null,
  defaultTeamProfileId: profile.defaultTeamProfileId?.trim() || null,
  teamRole: ['assistant', 'orchestrator', 'specialist', 'monitor'].includes(profile.teamRole || '')
    ? profile.teamRole
    : 'assistant',
})
