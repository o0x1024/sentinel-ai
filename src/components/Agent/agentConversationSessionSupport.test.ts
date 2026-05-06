import { describe, expect, it } from 'vitest'

import {
  createConversationSession,
  pickLatestConversation,
} from '@/components/Agent/agentConversationSessionSupport'

describe('agentConversationSessionSupport', () => {
  it('picks the most recently created conversation first', () => {
    const latest = pickLatestConversation([
      {
        id: 'older-created-newer-updated',
        title: 'older',
        created_at: '2026-04-10T10:00:00.000Z',
        updated_at: '2026-04-15T10:00:00.000Z',
      },
      {
        id: 'newer-created',
        title: 'newer',
        created_at: '2026-04-12T10:00:00.000Z',
        updated_at: '2026-04-12T11:00:00.000Z',
      },
    ])

    expect(latest?.id).toBe('newer-created')
  })

  it('passes the current conversation binding when creating a new conversation', async () => {
    let capturedRequest: Record<string, unknown> | null = null

    await createConversationSession({
      conversationBinding: {
        schemaVersion: 4,
        profileId: 'assistant.default',
        contextMode: 'codex-like',
        runMode: 'assistant',
        workingDirectoryOverride: '/tmp/workspace',
        ragEnabled: true,
        webSearchEnabled: true,
        tenthManEnabled: false,
        selectedModel: 'openai/gpt-5.5',
        toolsEnabled: true,
        toolConfig: null,
      },
      createConversation: async (request) => {
        capturedRequest = request
        return 'conv-new'
      },
      getConversationTitle: () => 'New conversation',
      getDisplayTitle: () => 'New conversation',
      onConversationCreated: () => {},
    })

    expect(capturedRequest).toMatchObject({
      service_name: 'default',
      title: 'New conversation',
      conversation_binding: {
        contextMode: 'codex-like',
        selectedModel: 'openai/gpt-5.5',
        webSearchEnabled: true,
      },
    })
  })
})
