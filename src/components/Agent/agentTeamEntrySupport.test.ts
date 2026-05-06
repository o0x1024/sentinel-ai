import { describe, expect, it } from 'vitest'

import { ensureConversationForTeamSession } from '@/components/Agent/agentTeamEntrySupport'

describe('agentTeamEntrySupport', () => {
  it('passes the current conversation binding when Team mode creates a new conversation', async () => {
    let capturedRequest: Record<string, unknown> | null = null

    const conversationId = await ensureConversationForTeamSession({
      conversationBinding: {
        schemaVersion: 4,
        profileId: 'assistant.default',
        contextMode: 'claude-like',
        runMode: 'team',
        workingDirectoryOverride: '/tmp/team',
        ragEnabled: true,
        webSearchEnabled: false,
        tenthManEnabled: false,
        selectedModel: 'anthropic/claude-4',
        toolsEnabled: true,
        toolConfig: null,
      },
      createConversation: async (request) => {
        capturedRequest = request
        return 'conv-team'
      },
      getConversationTitle: () => 'Team conversation',
      getDisplayTitle: () => 'Team conversation',
      onConversationReady: () => {},
      syncActiveTeamSession: async () => {},
    })

    expect(conversationId).toBe('conv-team')
    expect(capturedRequest).toMatchObject({
      service_name: 'default',
      title: 'Team conversation',
      conversation_binding: {
        runMode: 'team',
        selectedModel: 'anthropic/claude-4',
        workingDirectoryOverride: '/tmp/team',
      },
    })
  })
})
