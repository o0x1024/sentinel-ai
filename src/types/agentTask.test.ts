import { describe, expect, it } from 'vitest'

import {
  calculateAgentTaskProgress,
  getAgentTaskDisplayText,
  getRootAgentTasks,
  mapTaskRuntimeItemsToAgentTasks,
} from './agentTask'

describe('agentTask', () => {
  it('maps runtime tasks into task objects and preserves hierarchy', () => {
    const tasks = mapTaskRuntimeItemsToAgentTasks([
      {
        id: 'parent',
        content: '父任务',
        status: 'in_progress',
        active_form: '正在处理父任务',
        created_at: 1,
        updated_at: 2,
      },
      {
        id: 'child',
        content: '子任务',
        status: 'pending',
        created_at: 1,
        updated_at: 1,
        metadata: {
          parent_id: 'parent',
        },
      },
    ], 'exec-1')

    expect(tasks).toHaveLength(2)
    expect(getRootAgentTasks(tasks)).toHaveLength(1)
    expect(getAgentTaskDisplayText(tasks[0]!)).toBe('正在处理父任务')
    expect(tasks[1]?.metadata?.parent_id).toBe('parent')
    expect(tasks[0]?.metadata?.source_execution_id).toBe('exec-1')
  })

  it('calculates progress from completed tasks', () => {
    const tasks = mapTaskRuntimeItemsToAgentTasks([
      {
        id: 'task-1',
        content: '任务 1',
        status: 'completed',
        created_at: 1,
        updated_at: 1,
      },
      {
        id: 'task-2',
        content: '任务 2',
        status: 'pending',
        created_at: 1,
        updated_at: 1,
      },
    ])

    expect(calculateAgentTaskProgress(tasks)).toBe(50)
  })
})
