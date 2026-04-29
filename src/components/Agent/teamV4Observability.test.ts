import { describe, expect, it } from 'vitest'
import { deriveTeamV4Observability } from './teamV4Observability'
import type { TeamV4Agent, TeamV4Event, TeamV4HarnessRun, TeamV4Task } from '@/types/teamRuntime'

const agent = (overrides: Partial<TeamV4Agent>): TeamV4Agent => ({
  id: 'agent-1',
  run_id: 'run-1',
  profile_id: 'assistant.default',
  role_type: 'solver',
  name: 'Solver 1',
  status: 'idle',
  model: 'model',
  context_mode: 'claude-like',
  tool_policy_json: {},
  metadata: {},
  created_at: '2026-04-28T08:00:00Z',
  updated_at: '2026-04-28T08:00:00Z',
  ...overrides,
})

const task = (overrides: Partial<TeamV4Task>): TeamV4Task => ({
  id: 'task-1',
  run_id: 'run-1',
  parent_task_id: null,
  task_key: 'root',
  title: 'Root Task',
  instruction: 'Do work',
  status: 'running',
  priority: 0,
  assigned_agent_id: 'agent-1',
  depends_on: [],
  acceptance_criteria: null,
  context_snapshot_id: null,
  metadata: {},
  created_at: '2026-04-28T08:00:00Z',
  updated_at: '2026-04-28T08:00:00Z',
  ...overrides,
})

const event = (overrides: Partial<TeamV4Event>): TeamV4Event => ({
  id: `event-${overrides.sequence || 1}`,
  run_id: 'run-1',
  sequence: 1,
  actor_id: 'agent-1',
  task_id: 'task-1',
  event_type: 'solver_execution_started',
  visibility: 'workspace',
  payload: {},
  created_at: '2026-04-28T08:00:00Z',
  ...overrides,
})

const harness = (overrides: Partial<TeamV4HarnessRun>): TeamV4HarnessRun => ({
  id: 'harness-1',
  run_id: 'run-1',
  actor_id: 'agent-1',
  task_id: 'task-1',
  status: 'running',
  lease_expires_at: null,
  last_heartbeat_at: '2026-04-28T08:01:00Z',
  checkpoint_sequence: 1,
  metadata: {},
  created_at: '2026-04-28T08:00:00Z',
  updated_at: '2026-04-28T08:01:00Z',
  ...overrides,
})

describe('teamV4Observability', () => {
  it('maps solver tool events into live activity and waiting_tool agent status', () => {
    const state = deriveTeamV4Observability({
      agents: [agent({})],
      events: [
        event({ sequence: 1, event_type: 'solver_execution_started' }),
        event({
          id: 'event-2',
          sequence: 2,
          event_type: 'solver_tool_started',
          payload: {
            toolName: 'http_request',
          },
        }),
      ],
      harnessRuns: [harness({})],
      runState: 'running',
      tasks: [task({})],
    })

    expect(state.phase).toBe('tool_running')
    expect(state.headline).toBe('Solver 1 called http_request')
    expect(state.toolEvents).toHaveLength(1)
    expect(state.agentStates[0].status).toBe('waiting_tool')
  })

  it('keeps failed solver events visible as the run headline', () => {
    const state = deriveTeamV4Observability({
      agents: [agent({})],
      events: [
        event({
          sequence: 1,
          event_type: 'solver_execution_failed',
          payload: {
            error: 'Connection refused',
          },
        }),
      ],
      harnessRuns: [],
      runState: 'failed',
      tasks: [task({ status: 'failed' })],
    })

    expect(state.phase).toBe('failed')
    expect(state.headline).toBe('Solver 1 failed Root Task')
    expect(state.detail).toBe('Connection refused')
    expect(state.agentStates[0].status).toBe('failed')
  })
})
