import { describe, expect, it } from 'vitest'

import { mapPersistedAgentTasks } from './agentTaskHistorySupport'

describe('agentTaskHistorySupport', () => {
  it('maps persisted rows into agent tasks', () => {
    const tasks = mapPersistedAgentTasks([
      {
        id: 'exec-1_0',
        execution_id: 'exec-1',
        item_index: 0,
        content: '读取 openapi.json',
        status: 'in_progress',
        result: null,
        created_at_ms: 100,
        updated_at_ms: 120,
      },
      {
        id: 'exec-1_1',
        execution_id: 'exec-1',
        item_index: 1,
        content: '分析 API 结构',
        status: 'completed',
        result: 'done',
        created_at_ms: 100,
        updated_at_ms: 140,
      },
    ])

    expect(tasks).toHaveLength(2)
    expect(tasks[0]?.status).toBe('in_progress')
    expect(tasks[0]?.metadata?.step_index).toBe(1)
    expect(tasks[1]?.status).toBe('completed')
    expect(tasks[1]?.metadata?.reason).toBe('done')
  })
})
