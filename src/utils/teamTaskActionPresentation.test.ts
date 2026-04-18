import { describe, expect, it } from 'vitest'

import {
  buildTeamTaskActionFailure,
  buildTeamTaskActionSuccess,
} from './teamTaskActionPresentation'

describe('teamTaskActionPresentation', () => {
  it('builds success messages from the task title when available', () => {
    const result = buildTeamTaskActionSuccess('claim', {
      taskId: 'task-record-1',
      taskKey: 'deliver-summary',
      title: '汇总结论',
      status: 'claimed',
    })

    expect(result.success).toBe(true)
    expect(result.message).toBe('任务已认领：汇总结论')
    expect(result.next_step).toContain('及时更新结果')
  })

  it('falls back to task key when the title is missing', () => {
    const result = buildTeamTaskActionSuccess('create', {
      taskId: 'task-record-2',
      taskKey: 'collect-context',
      title: '',
      status: 'pending',
    })

    expect(result.message).toBe('任务已创建：collect-context')
  })

  it('maps common backend errors into user-facing reasons', () => {
    const result = buildTeamTaskActionFailure('release', {
      taskId: 'task-record-3',
      title: '整理接口清单',
      error: 'Task is not currently claimed by this agent',
    })

    expect(result.success).toBe(false)
    expect(result.message).toBe('任务暂不可释放：整理接口清单')
    expect(result.reason).toBe('当前负责人不是该成员，无法直接释放。')
    expect(result.next_step).toContain('当前负责人')
  })

  it('builds completion success messages with dependency follow-up guidance', () => {
    const result = buildTeamTaskActionSuccess('complete', {
      taskId: 'task-record-4',
      title: '汇总结论',
      status: 'completed',
    })

    expect(result.message).toBe('任务已完成：汇总结论')
    expect(result.next_step).toContain('解除依赖')
  })
})
