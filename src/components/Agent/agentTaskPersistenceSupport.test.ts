import { describe, expect, it, vi } from 'vitest'

import { buildPersistedAgentTaskMessageRequest, persistAgentTaskMessage } from './agentTaskPersistenceSupport'

describe('agentTaskPersistenceSupport', () => {
  it('builds a persisted system message request for agent task updates', () => {
    const request = buildPersistedAgentTaskMessageRequest({
      conversationId: 'conv-1',
      message: {
        id: 'agent-task:planned:conv-1:1',
        type: 'system',
        content: '已建立 2 个执行任务：读取 openapi.json、分析 API 结构',
        timestamp: 1,
        metadata: {
          kind: 'agent_task_update',
          task_event_type: 'planned',
          task_count: 2,
          task_preview: '读取 openapi.json、分析 API 结构',
        },
      },
    })

    expect(request).toEqual({
      id: 'agent-task:planned:conv-1:1',
      conversation_id: 'conv-1',
      role: 'system',
      content: '已建立 2 个执行任务：读取 openapi.json、分析 API 结构',
      metadata: {
        kind: 'agent_task_update',
        task_event_type: 'planned',
        task_count: 2,
        task_preview: '读取 openapi.json、分析 API 结构',
        source: 'agent_tasks_update',
      },
    })
  })

  it('tracks persisted ids and suppresses duplicates', async () => {
    const persistedIds = new Set<string>()
    const persistMessage = vi.fn(async () => undefined)
    const message = {
      id: 'agent-task:planned:conv-1:1',
      type: 'system' as const,
      content: '已建立任务',
      timestamp: 1,
      metadata: {
        kind: 'agent_task_update',
        task_event_type: 'planned',
      },
    }

    const first = await persistAgentTaskMessage({
      conversationId: 'conv-1',
      message,
      persistedIds,
      persistMessage,
    })
    const second = await persistAgentTaskMessage({
      conversationId: 'conv-1',
      message,
      persistedIds,
      persistMessage,
    })

    expect(first).toBe(true)
    expect(second).toBe(false)
    expect(persistMessage).toHaveBeenCalledTimes(1)
    expect(persistedIds.has(message.id)).toBe(true)
  })
})
