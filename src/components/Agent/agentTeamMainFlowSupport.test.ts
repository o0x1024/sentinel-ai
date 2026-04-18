import { describe, expect, it, vi } from 'vitest'

import { appendTeamMessagesToMainFlow } from './agentTeamMainFlowSupport'

describe('agentTeamMainFlowSupport', () => {
  it('preserves dependency-ready metadata when mirroring system messages to the main flow', () => {
    const pushed: any[] = []

    appendTeamMessagesToMainFlow({
      messagesResp: [{
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
      }],
      teamMainFlowMessageIds: new Set<string>(),
      teamMirroredConversationMessageIds: new Set<string>(),
      teamPersistedAssistantSuppressionKeys: new Set<string>(),
      pushMainFlowMessage: (message) => {
        pushed.push(message)
      },
      insertMainFlowMessageAtPreferredIndex: vi.fn(),
      upsertTeamToolCallToMainFlow: vi.fn(),
      mirrorTeamMessageToConversation: vi.fn(),
      consumeTeamLocalHumanInputForPersistedMessage: vi.fn(),
      consumeTeamStreamTempForPersistedMessage: vi.fn(),
    })

    expect(pushed).toHaveLength(1)
    expect(pushed[0]?.metadata?.kind).toBe('team_dependency_ready')
    expect(pushed[0]?.metadata?.team_task_record_id).toBe('task-2')
    expect(pushed[0]?.metadata?.action_label).toBe('查看任务')
  })
})
