import { describe, expect, it } from 'vitest'

import {
  buildTeamTaskSections,
  hasUnresolvedTeamTaskDependencies,
  normalizeTeamTaskStatus,
  resolveTeamTaskDependencyTitles,
  resolveSatisfiedTeamTaskDependencies,
  resolveUnresolvedTeamTaskDependencies,
  teamBlackboardEntryTypeI18nKey,
  teamTaskSectionActionI18nKey,
  teamTaskSectionEmptyI18nKey,
  teamTaskStatusBadgeClass,
  teamTaskStatusI18nKey,
} from './teamWorkspacePresentation'

describe('teamWorkspacePresentation', () => {
  it('maps task statuses to normalized keys and i18n labels', () => {
    expect(normalizeTeamTaskStatus('ready_for_claim')).toBe('ready_for_claim')
    expect(teamTaskStatusI18nKey('ready_for_claim')).toBe(
      'agent.teamTaskStatus.readyForClaim',
    )
    expect(teamTaskStatusBadgeClass('blocked')).toBe('badge-warning')
  })

  it('maps blackboard entry types to translated keys', () => {
    expect(teamBlackboardEntryTypeI18nKey('artifact_ref')).toBe(
      'agent.teamBlackboardEntryType.artifactRef',
    )
    expect(teamBlackboardEntryTypeI18nKey('unknown-entry')).toBe(
      'agent.teamBlackboardEntryType.generic',
    )
  })

  it('resolves dependency titles by task key before falling back to raw ids', () => {
    const titles = resolveTeamTaskDependencyTitles(
      {
        id: 'task-3',
        session_id: 'session-1',
        task_id: 'deliver-summary',
        title: '汇总结论',
        instruction: '汇总输出',
        status: 'pending',
        assignee_agent_id: null,
        owner_agent_id: 'lead',
        claimed_by_agent_id: null,
        acceptance_criteria: null,
        depends_on: ['collect-context', 'task-2', 'missing-task'],
        attempt: 0,
        max_attempts: 1,
        last_error: null,
        started_at: null,
        completed_at: null,
        created_at: '2026-04-18T00:00:00Z',
        updated_at: '2026-04-18T00:00:00Z',
      },
      [
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
          updated_at: '2026-04-18T00:00:00Z',
        },
        {
          id: 'task-2',
          session_id: 'session-1',
          task_id: 'analyze-options',
          title: '分析方案',
          instruction: '分析风险',
          status: 'running',
          assignee_agent_id: 'agent-2',
          owner_agent_id: 'agent-2',
          claimed_by_agent_id: 'agent-2',
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
      ],
    )

    expect(titles).toEqual(['收集上下文', '分析方案', 'missing-task'])
  })

  it('tracks unresolved dependencies until they are completed', () => {
    const task = {
      id: 'task-3',
      session_id: 'session-1',
      task_id: 'deliver-summary',
      title: '汇总结论',
      instruction: '汇总输出',
      status: 'pending',
      assignee_agent_id: null,
      owner_agent_id: 'lead',
      claimed_by_agent_id: null,
      acceptance_criteria: null,
      depends_on: ['collect-context', 'analyze-options', 'missing-task'],
      attempt: 0,
      max_attempts: 1,
      last_error: null,
      started_at: null,
      completed_at: null,
      created_at: '2026-04-18T00:00:00Z',
      updated_at: '2026-04-18T00:00:00Z',
    }

    const tasks = [
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
        updated_at: '2026-04-18T00:00:00Z',
      },
      {
        id: 'task-2',
        session_id: 'session-1',
        task_id: 'analyze-options',
        title: '分析方案',
        instruction: '分析风险',
        status: 'running',
        assignee_agent_id: 'agent-2',
        owner_agent_id: 'agent-2',
        claimed_by_agent_id: 'agent-2',
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
    ]

    expect(resolveUnresolvedTeamTaskDependencies(task, tasks)).toEqual([
      '分析方案',
      'missing-task',
    ])
    expect(resolveSatisfiedTeamTaskDependencies(task, tasks)).toEqual([
      '收集上下文',
    ])
    expect(hasUnresolvedTeamTaskDependencies(task, tasks)).toBe(true)
  })

  it('groups tasks by collaboration-oriented sections', () => {
    const sections = buildTeamTaskSections([
      {
        id: 'task-1',
        session_id: 'session-1',
        task_id: 'collect-context',
        title: '收集上下文',
        instruction: '收集事实',
        status: 'ready_for_claim',
        assignee_agent_id: null,
        owner_agent_id: 'agent-1',
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
      },
      {
        id: 'task-2',
        session_id: 'session-1',
        task_id: 'analyze-options',
        title: '分析方案',
        instruction: '分析风险',
        status: 'running',
        assignee_agent_id: 'agent-2',
        owner_agent_id: 'agent-2',
        claimed_by_agent_id: 'agent-2',
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
        id: 'task-3',
        session_id: 'session-1',
        task_id: 'deliver-summary',
        title: '汇总结论',
        instruction: '汇总输出',
        status: 'pending',
        assignee_agent_id: null,
        owner_agent_id: 'lead',
        claimed_by_agent_id: null,
        acceptance_criteria: null,
        depends_on: ['analyze-options'],
        attempt: 0,
        max_attempts: 1,
        last_error: null,
        started_at: null,
        completed_at: null,
        created_at: '2026-04-18T00:00:00Z',
        updated_at: '2026-04-18T00:00:00Z',
      },
    ])

    expect(sections.map((section) => section.key)).toEqual([
      'executable',
      'active',
      'blocked',
      'review',
      'completed',
      'attention',
    ])
    expect(sections.find((section) => section.key === 'review')?.tasks).toEqual([])
    expect(sections.find((section) => section.key === 'attention')?.tasks).toEqual([])
  })

  it('exposes empty-state and action guidance keys for each section', () => {
    expect(teamTaskSectionEmptyI18nKey('executable')).toBe(
      'agent.teamTaskSectionEmpty.executable',
    )
    expect(teamTaskSectionActionI18nKey('attention')).toBe(
      'agent.teamTaskSectionAction.attention',
    )
  })
})
