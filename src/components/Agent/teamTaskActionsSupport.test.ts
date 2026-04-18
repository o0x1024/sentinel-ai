import { describe, expect, it } from 'vitest'

import type { TeamTask } from '@/types/agentTeam'
import {
  buildTeamTaskActionToastMessage,
  canTeamTaskBlock,
  canTeamTaskClaim,
  canTeamTaskComplete,
  canTeamTaskFail,
  canTeamTaskRelease,
  getTeamTaskClaimAgentId,
  getTeamTaskReleaseAgentId,
} from './teamTaskActionsSupport'

const buildTask = (overrides: Partial<TeamTask> = {}): TeamTask => ({
  id: 'task-1',
  session_id: 'session-1',
  task_id: 'collect-context',
  title: '收集上下文',
  instruction: '收集事实',
  status: 'pending',
  assignee_agent_id: null,
  owner_agent_id: 'agent-owner',
  claimed_by_agent_id: null,
  acceptance_criteria: null,
  depends_on: [],
  attempt: 0,
  max_attempts: 1,
  last_error: null,
  started_at: null,
  completed_at: null,
  created_at: '2026-04-18T00:00:00Z',
  updated_at: '2026-04-18T00:00:00Z',
  ...overrides,
})

describe('teamTaskActionsSupport', () => {
  it('prefers the owner as the claim actor', () => {
    const task = buildTask({ owner_agent_id: 'owner-1', assignee_agent_id: 'agent-2' })
    expect(getTeamTaskClaimAgentId(task)).toBe('owner-1')
    expect(canTeamTaskClaim(task)).toBe(true)
  })

  it('requires an active handler for release actions', () => {
    const task = buildTask({
      status: 'claimed',
      owner_agent_id: 'owner-1',
      claimed_by_agent_id: 'agent-3',
    })
    expect(getTeamTaskReleaseAgentId(task)).toBe('agent-3')
    expect(canTeamTaskRelease(task)).toBe(true)
  })

  it('builds a toast message by concatenating message, reason, and next step', () => {
    expect(buildTeamTaskActionToastMessage({
      message: '任务暂不可认领：汇总结论',
      reason: '任务已被其他成员占用。',
      next_step: '请先查看任务最新状态。',
    })).toBe('任务暂不可认领：汇总结论 任务已被其他成员占用。 请先查看任务最新状态。')
  })

  it('allows manual status updates for active tasks', () => {
    const task = buildTask({ status: 'running' })
    expect(canTeamTaskComplete(task)).toBe(true)
    expect(canTeamTaskFail(task)).toBe(true)
    expect(canTeamTaskBlock(task)).toBe(true)
  })
})
