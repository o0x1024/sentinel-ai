import { describe, expect, it } from 'vitest'

import {
  normalizeHarnessMaxContinuations,
  type AssistantProfileOption,
} from '@/components/Agent/assistantProfiles'
import { applyToolConfigToProfile, profileToToolConfig } from '@/components/Settings/assistantProfileRegistrySupport'

const createProfile = (): AssistantProfileOption => ({
  id: 'assistant.custom.1',
  label: 'Custom 1',
  description: 'test profile',
  defaultModel: null,
  defaultRagEnabled: false,
  defaultWebSearchEnabled: false,
  defaultToolsEnabled: true,
  defaultTenthManEnabled: false,
  defaultToolSelectionStrategy: 'Keyword',
  defaultMaxTools: 5,
  defaultHarnessMaxContinuations: 6,
  defaultPreselectedTools: [],
  defaultDisabledTools: [],
  defaultManualTools: [],
  defaultTeamOrchestrationPresetId: null,
  defaultTeamRecoveryPresetId: null,
  contextMode: 'claude-like',
  runMode: 'assistant',
})

describe('assistantProfileRegistrySupport', () => {
  it('preserves manual tools when selection strategy is the Manual string', () => {
    const profile = createProfile()

    applyToolConfigToProfile(profile, {
      enabled: true,
      selection_strategy: 'Manual',
      max_tools: 5,
      preselected_tools: [],
      disabled_tools: [],
      manual_tools: ['browser_shell__list', 'http_request'],
    })

    expect(profile.defaultToolSelectionStrategy).toBe('Manual')
    expect(profile.defaultManualTools).toEqual(['browser_shell__list', 'http_request'])
    expect(profileToToolConfig(profile).manual_tools).toEqual(['browser_shell__list', 'http_request'])
  })

  it('reads manual tools from the enum-shaped strategy payload', () => {
    const profile = createProfile()

    applyToolConfigToProfile(profile, {
      enabled: true,
      selection_strategy: { Manual: ['browser_shell::list', 'browser_shell::list', 'http_request'] },
      max_tools: 5,
      preselected_tools: [],
      disabled_tools: [],
      manual_tools: [],
    })

    expect(profile.defaultToolSelectionStrategy).toBe('Manual')
    expect(profile.defaultManualTools).toEqual(['browser_shell__list', 'http_request'])
    expect(profileToToolConfig(profile).manual_tools).toEqual(['browser_shell__list', 'http_request'])
  })

  it('normalizes removed Skills strategy to Keyword', () => {
    const profile = createProfile()

    applyToolConfigToProfile(profile, {
      enabled: true,
      selection_strategy: 'Skills',
      max_tools: 8,
      preselected_tools: [],
      disabled_tools: [],
      manual_tools: ['shell'],
    })

    expect(profile.defaultToolSelectionStrategy).toBe('Keyword')
    expect(profile.defaultManualTools).toEqual([])
    expect(profileToToolConfig(profile).selection_strategy).toBe('Keyword')
  })

  it('normalizes harness continuation limits', () => {
    expect(normalizeHarnessMaxContinuations(undefined)).toBe(6)
    expect(normalizeHarnessMaxContinuations(-1)).toBe(0)
    expect(normalizeHarnessMaxContinuations(99)).toBe(20)
  })
})
