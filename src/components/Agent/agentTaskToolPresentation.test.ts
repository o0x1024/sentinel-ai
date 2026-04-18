import { describe, expect, it } from 'vitest'

import { buildTaskToolCardData } from './agentTaskToolPresentation'

describe('agentTaskToolPresentation', () => {
  it('builds a planning card for add_items actions', () => {
    const card = buildTaskToolCardData({
      id: 'tool-1',
      type: 'tool_call',
      content: '',
      timestamp: 1,
      metadata: {
        tool_name: 'tasks',
        tool_args: {
          action: 'add_items',
          items: ['读取 openapi.json', '分析 API 结构'],
        },
        tool_result: JSON.stringify({
          success: true,
          list: {
            items: [
              { description: '读取 openapi.json', status: 'in_progress' },
              { description: '分析 API 结构', status: 'pending' },
            ],
          },
          message: 'Items added to tasks',
        }),
      },
    })

    expect(card?.title_key).toBe('agent.agentTaskPlannedTitle')
    expect(card?.item_count).toBe(2)
    expect(card?.preview).toContain('读取 openapi.json')
    expect(card?.detail).toContain('Items added to tasks')
  })

  it('builds a completion card for completed status updates', () => {
    const card = buildTaskToolCardData({
      id: 'tool-2',
      type: 'tool_call',
      content: '',
      timestamp: 1,
      metadata: {
        tool_name: 'tasks',
        tool_args: {
          action: 'update_status',
          item_index: 1,
          status: 'completed',
        },
        tool_result: JSON.stringify({
          success: true,
          message: 'Updated item 1 status to Completed',
        }),
      },
    })

    expect(card?.title_key).toBe('agent.agentTaskCompletedTitle')
    expect(card?.detail).toContain('Updated item 1 status to Completed')
  })
})
