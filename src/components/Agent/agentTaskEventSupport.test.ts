import { describe, expect, it } from 'vitest'

import { buildAgentTaskSystemMessages } from './agentTaskEventSupport'

describe('agentTaskEventSupport', () => {
  it('emits a planned message on the first task batch', () => {
    const messages = buildAgentTaskSystemMessages({
      executionId: 'exec-1',
      previousTasks: [],
      nextTasks: [
        {
          id: 'task-1',
          content: '读取 openapi.json',
          status: 'pending',
          created_at: 1,
          updated_at: 1,
        },
        {
          id: 'task-2',
          content: '分析 API 结构',
          status: 'pending',
          created_at: 1,
          updated_at: 1,
        },
      ],
      timestamp: 100,
    })

    expect(messages).toHaveLength(1)
    expect(messages[0]?.metadata?.kind).toBe('agent_task_update')
    expect(messages[0]?.metadata?.task_event_type).toBe('planned')
    expect(messages[0]?.content).toContain('已建立 2 个执行任务')
  })

  it('emits started and completed messages for status transitions', () => {
    const messages = buildAgentTaskSystemMessages({
      executionId: 'exec-1',
      previousTasks: [
        {
          id: 'task-1',
          content: '读取 openapi.json',
          active_form: '正在读取 openapi.json',
          status: 'pending',
          created_at: 1,
          updated_at: 1,
        },
        {
          id: 'task-2',
          content: '分析 API 结构',
          status: 'in_progress',
          active_form: '正在分析 API 结构',
          created_at: 1,
          updated_at: 2,
        },
      ],
      nextTasks: [
        {
          id: 'task-1',
          content: '读取 openapi.json',
          active_form: '正在读取 openapi.json',
          status: 'in_progress',
          created_at: 1,
          updated_at: 3,
        },
        {
          id: 'task-2',
          content: '分析 API 结构',
          status: 'completed',
          active_form: '正在分析 API 结构',
          created_at: 1,
          updated_at: 3,
        },
      ],
      timestamp: 200,
    })

    expect(messages).toHaveLength(2)
    expect(messages[0]?.metadata?.task_event_type).toBe('started')
    expect(messages[0]?.content).toContain('开始处理任务')
    expect(messages[1]?.metadata?.task_event_type).toBe('completed')
    expect(messages[1]?.content).toContain('已完成任务')
  })
})
