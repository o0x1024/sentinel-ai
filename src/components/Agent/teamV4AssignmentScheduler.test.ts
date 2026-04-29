import { describe, expect, it } from 'vitest'
import { runTeamV4AssignmentsWithDependencies } from './teamV4AssignmentScheduler'
import type { TeamV4SolverAssignment } from '@/types/teamRuntime'

const assignment = (id: string, dependsOn: string[] = []): TeamV4SolverAssignment => ({
  solver: {
    id: `solver-${id}`,
    run_id: 'run-1',
    profile_id: 'assistant.default',
    role_type: 'solver',
    name: `Solver ${id}`,
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
    assigned_agent_id: `solver-${id}`,
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
    actor_id: `solver-${id}`,
    task_id: id,
    role_type: 'solver',
    source_sequence: 0,
    policy_json: {},
    sections_json: [],
    token_estimate: 0,
    created_at: '2026-04-28T08:00:00Z',
  },
  harnessRun: {
    id: `harness-${id}`,
    run_id: 'run-1',
    actor_id: `solver-${id}`,
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
})
