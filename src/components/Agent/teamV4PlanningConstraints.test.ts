import { describe, expect, it } from 'vitest'
import {
  formatTeamV4PlanningConstraintIssues,
  validateTeamV4PlannedTasks,
  type TeamV4PlannedTask,
} from './teamV4PlanningConstraints'

const task = (overrides: Partial<TeamV4PlannedTask>): TeamV4PlannedTask => ({
  key: 'collect',
  title: 'Collect context',
  instruction: 'Collect context',
  acceptanceCriteria: 'Context collected',
  specialistId: 'specialist-1',
  requiredTools: ['file_read'],
  dependsOnTaskKeys: [],
  distinctFromTaskKeys: [],
  priority: 0,
  ...overrides,
})

describe('teamV4PlanningConstraints', () => {
  it('rejects distinct specialist conflicts', () => {
    const issues = validateTeamV4PlannedTasks({
      plannedTasks: [
        task({ key: 'logic', title: 'Analyze logic', distinctFromTaskKeys: ['traditional'] }),
        task({ key: 'traditional', title: 'Analyze traditional' }),
      ],
      availableSpecialistIds: new Set(['specialist-1', 'specialist-2']),
      maxTasksPerSpecialist: 2,
    })

    expect(issues).toHaveLength(1)
    expect(issues[0].code).toBe('distinct_specialist_conflict')
    expect(formatTeamV4PlanningConstraintIssues(issues)[0]).toContain('must use different specialists')
  })

  it('rejects plans that overload one specialist beyond capacity', () => {
    const issues = validateTeamV4PlannedTasks({
      plannedTasks: [
        task({ key: 'collect', specialistId: 'specialist-1' }),
        task({ key: 'search', specialistId: 'specialist-1', title: 'Search patterns' }),
      ],
      availableSpecialistIds: new Set(['specialist-1']),
      maxTasksPerSpecialist: 1,
    })

    expect(issues).toHaveLength(1)
    expect(issues[0].code).toBe('specialist_capacity_exceeded')
    expect(issues[0].taskKeys).toEqual(['collect', 'search'])
  })

  it('accepts valid distinct specialist lanes within capacity', () => {
    const issues = validateTeamV4PlannedTasks({
      plannedTasks: [
        task({
          key: 'logic',
          specialistId: 'specialist-1',
          distinctFromTaskKeys: ['traditional'],
        }),
        task({
          key: 'traditional',
          specialistId: 'specialist-2',
        }),
      ],
      availableSpecialistIds: new Set(['specialist-1', 'specialist-2']),
      maxTasksPerSpecialist: 1,
    })

    expect(issues).toEqual([])
  })
})
