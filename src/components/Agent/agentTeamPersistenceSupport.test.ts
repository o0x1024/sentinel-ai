import { describe, expect, it, vi } from 'vitest'

import { mirrorTeamMessageToConversation } from './agentTeamPersistenceSupport'

describe('agentTeamPersistenceSupport', () => {
  it('persists dependency-ready notices with CTA metadata intact', async () => {
    const persistMessage = vi.fn(async () => {})

    await mirrorTeamMessageToConversation({
      msg: {
        id: 'db-message-1',
        session_id: 'session-1',
        role: 'system',
        content: '任务现在可开始执行：汇总结论。',
        metadata: {
          kind: 'team_dependency_ready',
          task_record_id: 'task-2',
          task_key: 'deliver-summary',
          task_title: '汇总结论',
          action_label: '查看任务',
        },
        timestamp: '2026-04-18T00:01:00Z',
      },
      conversationId: 'conv-1',
      activeTeamSessionId: 'session-1',
      teamMirroredConversationMessageIds: new Set<string>(),
      shouldSuppressTeamMirrorNoiseMessage: () => false,
      persistMessage,
    })

    expect(persistMessage).toHaveBeenCalledTimes(1)
    expect(persistMessage).toHaveBeenCalledWith(expect.objectContaining({
      metadata: expect.objectContaining({
        kind: 'team_dependency_ready',
        action_label: '查看任务',
      }),
    }))
  })
})
