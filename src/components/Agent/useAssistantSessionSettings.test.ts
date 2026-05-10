import { describe, expect, it } from 'vitest'

import { buildBaseAssistantConversationBinding } from './useAssistantSessionSettings'

describe('buildBaseAssistantConversationBinding', () => {
  it('keeps only profile and working directory from the current new-conversation context', () => {
    const binding = buildBaseAssistantConversationBinding({
      workingDirectoryOverride: ' /tmp/current-workspace ',
      profile: {
        id: 'assistant.security',
        label: 'Security',
        description: 'Security analyst',
        contextMode: 'sentinel-like',
        runMode: 'assistant',
        defaultRagEnabled: true,
        defaultWebSearchEnabled: false,
        defaultTenthManEnabled: true,
        defaultHarnessMaxContinuations: 9,
        defaultToolsEnabled: true,
        defaultManualTools: ['shell'],
        defaultPreselectedTools: ['web_search'],
        defaultDisabledTools: ['file_write'],
      },
    })

    expect(binding).toMatchObject({
      schemaVersion: 4,
      profileId: 'assistant.security',
      teamProfileId: '',
      contextMode: 'sentinel-like',
      runMode: 'assistant',
      workingDirectoryOverride: '/tmp/current-workspace',
      ragEnabled: true,
      webSearchEnabled: false,
      tenthManEnabled: true,
      harnessMaxContinuations: 9,
      browserShellDirectWriteEnabled: false,
      browserShellSessionId: null,
      selectedModel: null,
      toolConfig: null,
    })
    expect('toolsEnabled' in binding).toBe(false)
  })
})
