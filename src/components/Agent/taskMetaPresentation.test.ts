import { describe, expect, it } from 'vitest'

import { buildAgentTaskMetaItems, buildTeamTaskMetaItems } from './taskMetaPresentation'

describe('taskMetaPresentation', () => {
  it('builds compact single-agent task meta rows', () => {
    const items = buildAgentTaskMetaItems({
      id: 'task-1',
      title: 'Read file',
      status: 'blocked',
      created_at: 1,
      updated_at: 1000,
      metadata: {
        source_label: 'tasks tool',
        reason: 'Waiting for readback',
      },
    })

    expect(items).toEqual([
      {
        key: 'source',
        labelKey: 'agent.taskMetaSourceLabel',
        value: 'tasks tool',
      },
      {
        key: 'reason',
        labelKey: 'agent.taskMetaReasonLabel',
        value: 'Waiting for readback',
        tone: 'warning',
      },
    ])
  })

  it('builds team task meta rows from shared task facts', () => {
    const items = buildTeamTaskMetaItems(
      {
        id: 'task-2',
        session_id: 'session-1',
        task_id: 'deliver-summary',
        title: '汇总结论',
        instruction: '汇总输出',
        status: 'running',
        assignee_agent_id: 'agent-2',
        owner_agent_id: 'agent-1',
        claimed_by_agent_id: 'agent-2',
        acceptance_criteria: '给出三条结论',
        depends_on: ['collect-context'],
        attempt: 1,
        max_attempts: 3,
        last_error: null,
        started_at: null,
        completed_at: null,
        created_at: '2026-04-18T00:00:00Z',
        updated_at: '2026-04-18T01:00:00Z',
      },
      {
        resolveAgentName: (id) => (id === 'agent-1' ? 'Lead' : 'Analyst'),
        dependencyPreview: () => '收集上下文',
        dependencySatisfiedPreview: () => '',
        dependencyBlockerPreview: () => '分析方案',
      },
    )

    expect(items).toEqual([
      {
        key: 'owner',
        labelKey: 'agent.teamTaskOwnerLabel',
        value: 'Lead',
      },
      {
        key: 'handler',
        labelKey: 'agent.teamTaskCurrentHandlerLabel',
        value: 'Analyst',
      },
      {
        key: 'dependencies',
        labelKey: 'agent.teamTaskDependencyLabel',
        value: '收集上下文',
      },
      {
        key: 'dependencies_blocked',
        labelKey: 'agent.teamTaskBlockingDependencyLabel',
        value: '分析方案',
        tone: 'warning',
      },
      {
        key: 'acceptance',
        labelKey: 'agent.teamTaskAcceptanceCriteriaLabel',
        value: '给出三条结论',
      },
      {
        key: 'attempts',
        labelKey: 'agent.teamTaskAttemptsLabel',
        value: '1/3',
      },
    ])
  })
})
