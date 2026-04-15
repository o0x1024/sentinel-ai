import { describe, expect, it } from 'vitest'

import type { AssistantProfileOption } from '@/components/Agent/assistantProfiles'
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
  defaultFixedTools: [],
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
      fixed_tools: [],
      disabled_tools: [],
      manual_tools: ['browser__open', 'http_request'],
    })

    expect(profile.defaultToolSelectionStrategy).toBe('Manual')
    expect(profile.defaultManualTools).toEqual(['browser__open', 'http_request'])
    expect(profileToToolConfig(profile).manual_tools).toEqual(['browser__open', 'http_request'])
  })

  it('reads manual tools from the enum-shaped strategy payload', () => {
    const profile = createProfile()

    applyToolConfigToProfile(profile, {
      enabled: true,
      selection_strategy: { Manual: ['browser::open', 'browser::open', 'http_request'] },
      max_tools: 5,
      fixed_tools: [],
      disabled_tools: [],
      manual_tools: [],
    })

    expect(profile.defaultToolSelectionStrategy).toBe('Manual')
    expect(profile.defaultManualTools).toEqual(['browser__open', 'http_request'])
    expect(profileToToolConfig(profile).manual_tools).toEqual(['browser__open', 'http_request'])
  })
})
