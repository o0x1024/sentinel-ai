import { describe, expect, it } from 'vitest'

import {
  buildTeamTaskCreateRequest,
  buildTeamTaskKey,
} from './teamTaskCreateSupport'

describe('teamTaskCreateSupport', () => {
  it('builds a stable ascii task key from the title', () => {
    expect(buildTeamTaskKey({ title: 'Collect Context Facts' }, 123456)).toBe(
      'collect-context-facts',
    )
  })

  it('falls back to a time-based key when the title has no ascii slug', () => {
    expect(buildTeamTaskKey({ title: '收集上下文' }, 46655)).toBe('task-zzz')
  })

  it('normalizes the create request payload', () => {
    expect(buildTeamTaskCreateRequest({
      title: '  Deliver Summary  ',
      instruction: '  Merge the findings into one brief. ',
      depends_on: [' collect-context ', '', 'analyze-options'],
      owner_agent_id: ' lead-agent ',
      acceptance_criteria: '  Include three risks. ',
    }, 1000)).toEqual({
      task_key: 'deliver-summary',
      title: 'Deliver Summary',
      instruction: 'Merge the findings into one brief.',
      owner_agent_id: 'lead-agent',
      acceptance_criteria: 'Include three risks.',
      priority: null,
      metadata: {
        depends_on: ['collect-context', 'analyze-options'],
      },
    })
  })
})
