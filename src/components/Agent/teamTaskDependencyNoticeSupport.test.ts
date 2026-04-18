import { describe, expect, it } from 'vitest'

import {
  buildTeamDependencyReadyMessageRequest,
  buildTeamDependencyReadyNotices,
} from './teamTaskDependencyNoticeSupport'

describe('teamTaskDependencyNoticeSupport', () => {
  it('emits a system notice when a blocked task becomes executable', () => {
    const notices = buildTeamDependencyReadyNotices({
      sessionId: 'session-1',
      previousTasks: [
        {
          id: 'task-1',
          session_id: 'session-1',
          task_id: 'collect-context',
          title: '收集上下文',
          instruction: '收集事实',
          status: 'running',
          assignee_agent_id: 'agent-1',
          owner_agent_id: 'agent-1',
          claimed_by_agent_id: 'agent-1',
          acceptance_criteria: null,
          depends_on: [],
          attempt: 1,
          max_attempts: 1,
          last_error: null,
          started_at: null,
          completed_at: null,
          created_at: '2026-04-18T00:00:00Z',
          updated_at: '2026-04-18T00:00:00Z',
        },
        {
          id: 'task-2',
          session_id: 'session-1',
          task_id: 'deliver-summary',
          title: '汇总结论',
          instruction: '汇总输出',
          status: 'pending',
          assignee_agent_id: null,
          owner_agent_id: 'lead',
          claimed_by_agent_id: null,
          acceptance_criteria: null,
          depends_on: ['collect-context'],
          attempt: 0,
          max_attempts: 1,
          last_error: null,
          started_at: null,
          completed_at: null,
          created_at: '2026-04-18T00:00:00Z',
          updated_at: '2026-04-18T00:00:00Z',
        },
      ],
      nextTasks: [
        {
          id: 'task-1',
          session_id: 'session-1',
          task_id: 'collect-context',
          title: '收集上下文',
          instruction: '收集事实',
          status: 'completed',
          assignee_agent_id: 'agent-1',
          owner_agent_id: 'agent-1',
          claimed_by_agent_id: 'agent-1',
          acceptance_criteria: null,
          depends_on: [],
          attempt: 1,
          max_attempts: 1,
          last_error: null,
          started_at: null,
          completed_at: null,
          created_at: '2026-04-18T00:00:00Z',
          updated_at: '2026-04-18T00:01:00Z',
        },
        {
          id: 'task-2',
          session_id: 'session-1',
          task_id: 'deliver-summary',
          title: '汇总结论',
          instruction: '汇总输出',
          status: 'pending',
          assignee_agent_id: null,
          owner_agent_id: 'lead',
          claimed_by_agent_id: null,
          acceptance_criteria: null,
          depends_on: ['collect-context'],
          attempt: 0,
          max_attempts: 1,
          last_error: null,
          started_at: null,
          completed_at: null,
          created_at: '2026-04-18T00:00:00Z',
          updated_at: '2026-04-18T00:01:00Z',
        },
      ],
    })

    expect(notices).toHaveLength(1)
    expect(notices[0]?.content).toContain('任务现在可开始执行：汇总结论')
    expect(notices[0]?.content).toContain('收集上下文')
  })

  it('does not emit duplicate notices when the message id already exists', () => {
    const notices = buildTeamDependencyReadyNotices({
      sessionId: 'session-1',
      existingMessages: [{
        id: 'team_dependency_ready:session-1:task-2:2026-04-18T00:01:00Z',
        session_id: 'session-1',
        role: 'system',
        content: 'existing',
        timestamp: '2026-04-18T00:01:00Z',
      }],
      previousTasks: [
        {
          id: 'task-2',
          session_id: 'session-1',
          task_id: 'deliver-summary',
          title: '汇总结论',
          instruction: '汇总输出',
          status: 'pending',
          assignee_agent_id: null,
          owner_agent_id: 'lead',
          claimed_by_agent_id: null,
          acceptance_criteria: null,
          depends_on: ['collect-context'],
          attempt: 0,
          max_attempts: 1,
          last_error: null,
          started_at: null,
          completed_at: null,
          created_at: '2026-04-18T00:00:00Z',
          updated_at: '2026-04-18T00:00:00Z',
        },
      ],
      nextTasks: [
        {
          id: 'task-2',
          session_id: 'session-1',
          task_id: 'deliver-summary',
          title: '汇总结论',
          instruction: '汇总输出',
          status: 'pending',
          assignee_agent_id: null,
          owner_agent_id: 'lead',
          claimed_by_agent_id: null,
          acceptance_criteria: null,
          depends_on: [],
          attempt: 0,
          max_attempts: 1,
          last_error: null,
          started_at: null,
          completed_at: null,
          created_at: '2026-04-18T00:00:00Z',
          updated_at: '2026-04-18T00:01:00Z',
        },
      ],
    })

    expect(notices).toEqual([])
  })

  it('does not emit duplicate notices when a persisted message carries the same notice_id', () => {
    const notices = buildTeamDependencyReadyNotices({
      sessionId: 'session-1',
      existingMessages: [{
        id: 'db-message-1',
        session_id: 'session-1',
        role: 'system',
        content: 'existing',
        metadata: {
          kind: 'team_dependency_ready',
          notice_id: 'team_dependency_ready:session-1:task-2:2026-04-18T00:01:00Z',
        },
        timestamp: '2026-04-18T00:01:00Z',
      }],
      previousTasks: [
        {
          id: 'task-2',
          session_id: 'session-1',
          task_id: 'deliver-summary',
          title: '汇总结论',
          instruction: '汇总输出',
          status: 'pending',
          assignee_agent_id: null,
          owner_agent_id: 'lead',
          claimed_by_agent_id: null,
          acceptance_criteria: null,
          depends_on: ['collect-context'],
          attempt: 0,
          max_attempts: 1,
          last_error: null,
          started_at: null,
          completed_at: null,
          created_at: '2026-04-18T00:00:00Z',
          updated_at: '2026-04-18T00:00:00Z',
        },
      ],
      nextTasks: [
        {
          id: 'task-2',
          session_id: 'session-1',
          task_id: 'deliver-summary',
          title: '汇总结论',
          instruction: '汇总输出',
          status: 'pending',
          assignee_agent_id: null,
          owner_agent_id: 'lead',
          claimed_by_agent_id: null,
          acceptance_criteria: null,
          depends_on: [],
          attempt: 0,
          max_attempts: 1,
          last_error: null,
          started_at: null,
          completed_at: null,
          created_at: '2026-04-18T00:00:00Z',
          updated_at: '2026-04-18T00:01:00Z',
        },
      ],
    })

    expect(notices).toEqual([])
  })

  it('builds a persisted system-message request with metadata intact', () => {
    const [notice] = buildTeamDependencyReadyNotices({
      sessionId: 'session-1',
      previousTasks: [{
        id: 'task-2',
        session_id: 'session-1',
        task_id: 'deliver-summary',
        title: '汇总结论',
        instruction: '汇总输出',
        status: 'pending',
        assignee_agent_id: null,
        owner_agent_id: 'lead',
        claimed_by_agent_id: null,
        acceptance_criteria: null,
        depends_on: ['collect-context'],
        attempt: 0,
        max_attempts: 1,
        last_error: null,
        started_at: null,
        completed_at: null,
        created_at: '2026-04-18T00:00:00Z',
        updated_at: '2026-04-18T00:00:00Z',
      }],
      nextTasks: [{
        id: 'task-2',
        session_id: 'session-1',
        task_id: 'deliver-summary',
        title: '汇总结论',
        instruction: '汇总输出',
        status: 'pending',
        assignee_agent_id: null,
        owner_agent_id: 'lead',
        claimed_by_agent_id: null,
        acceptance_criteria: null,
        depends_on: [],
        attempt: 0,
        max_attempts: 1,
        last_error: null,
        started_at: null,
        completed_at: null,
        created_at: '2026-04-18T00:00:00Z',
        updated_at: '2026-04-18T00:01:00Z',
      }],
    })

    const request = buildTeamDependencyReadyMessageRequest(notice!)
    expect(request.message_type).toBe('system')
    expect(request.payload.metadata.notice_id).toBe(notice?.metadata?.notice_id)
    expect(request.payload.metadata.kind).toBe('team_dependency_ready')
  })
})
