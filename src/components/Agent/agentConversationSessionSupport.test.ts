import { describe, expect, it } from 'vitest'

import { pickLatestConversation } from '@/components/Agent/agentConversationSessionSupport'

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
})
