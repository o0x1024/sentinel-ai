import { describe, expect, it } from 'vitest'

import { buildAgentTaskMetaItems } from './taskMetaPresentation'

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
})
