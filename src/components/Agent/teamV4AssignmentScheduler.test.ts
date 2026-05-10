import { describe, expect, it } from 'vitest'
import { runTeamV4AssignmentsWithDependencies } from './teamV4AssignmentScheduler'
import type { TeamV4SpecialistAssignment } from '@/types/teamRuntime'

const assignment = (id: string, dependsOn: string[] = []): TeamV4SpecialistAssignment => ({
  specialist: {
    id: `specialist-${id}`,
    run_id: 'run-1',
    profile_id: 'assistant.default',
    role_type: 'specialist',
    name: `Specialist ${id}`,
    status: 'idle',
    model: null,
    context_mode: 'claude-like',
    tool_policy_json: {},
    metadata: {},
    created_at: '2026-04-28T08:00:00Z',
    updated_at: '2026-04-28T08:00:00Z',
  },
  task: {
    id,
    run_id: 'run-1',
    parent_task_id: null,
    task_key: id,
    title: id,
    instruction: id,
    status: 'ready',
    priority: 0,
    assigned_agent_id: `specialist-${id}`,
    depends_on: dependsOn,
    acceptance_criteria: null,
    context_snapshot_id: null,
    metadata: {},
    created_at: '2026-04-28T08:00:00Z',
    updated_at: '2026-04-28T08:00:00Z',
  },
  contextSnapshot: {
    id: `context-${id}`,
    run_id: 'run-1',
    actor_id: `specialist-${id}`,
    task_id: id,
    role_type: 'specialist',
    source_sequence: 0,
    policy_json: {},
    sections_json: [],
    token_estimate: 0,
    created_at: '2026-04-28T08:00:00Z',
  },
  harnessRun: {
    id: `harness-${id}`,
    run_id: 'run-1',
    actor_id: `specialist-${id}`,
    task_id: id,
    status: 'running',
    lease_expires_at: null,
    last_heartbeat_at: null,
    checkpoint_sequence: 0,
    metadata: {},
    created_at: '2026-04-28T08:00:00Z',
    updated_at: '2026-04-28T08:00:00Z',
  },
})

const assignmentForSpecialist = (
  id: string,
  specialistId: string,
  dependsOn: string[] = [],
): TeamV4SpecialistAssignment => ({
  ...assignment(id, dependsOn),
  specialist: {
    ...assignment(id, dependsOn).specialist,
    id: specialistId,
    name: specialistId,
  },
  task: {
    ...assignment(id, dependsOn).task,
    assigned_agent_id: specialistId,
  },
  contextSnapshot: {
    ...assignment(id, dependsOn).contextSnapshot,
    actor_id: specialistId,
  },
  harnessRun: {
    ...assignment(id, dependsOn).harnessRun,
    actor_id: specialistId,
  },
})

describe('teamV4AssignmentScheduler', () => {
  it('waits for dependency completion before launching dependent tasks', async () => {
    const order: string[] = []
    await runTeamV4AssignmentsWithDependencies(
      [assignment('collect'), assignment('verify', ['collect'])],
      2,
      async (item) => {
        order.push(item.task.id)
        return item.task.id
      },
    )

    expect(order).toEqual(['collect', 'verify'])
  })

  it('rejects unknown dependencies', async () => {
    await expect(runTeamV4AssignmentsWithDependencies(
      [assignment('verify', ['missing'])],
      1,
      async (item) => item.task.id,
    )).rejects.toThrow('unknown dependency')
  })

  it('never runs two assignments concurrently on the same specialist', async () => {
    const timeline: string[] = []
    let active = 0
    let peak = 0
    await runTeamV4AssignmentsWithDependencies(
      [
        assignmentForSpecialist('collect', 'specialist-shared'),
        assignmentForSpecialist('analyze', 'specialist-shared'),
      ],
      2,
      async (item) => {
        timeline.push(`start:${item.task.id}`)
        active += 1
        peak = Math.max(peak, active)
        await new Promise((resolve) => setTimeout(resolve, 10))
        active -= 1
        timeline.push(`finish:${item.task.id}`)
        return item.task.id
      },
    )

    expect(peak).toBe(1)
    expect(timeline).toEqual([
      'start:collect',
      'finish:collect',
      'start:analyze',
      'finish:analyze',
    ])
  })
})
