import { describe, expect, it } from 'vitest'
import { deriveTeamV4Observability } from './teamV4Observability'
import type { TeamV4Agent, TeamV4Event, TeamV4HarnessRun, TeamV4Task } from '@/types/teamRuntime'

const agent = (overrides: Partial<TeamV4Agent>): TeamV4Agent => ({
  id: 'agent-1',
  run_id: 'run-1',
  profile_id: 'assistant.default',
  role_type: 'specialist',
  name: 'Specialist 1',
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
  event_type: 'specialist_execution_started',
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
  it('maps specialist tool events into live activity and waiting_tool agent status', () => {
    const state = deriveTeamV4Observability({
      agents: [agent({})],
      events: [
        event({ sequence: 1, event_type: 'specialist_execution_started' }),
        event({
          id: 'event-2',
          sequence: 2,
          event_type: 'specialist_tool_started',
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
    expect(state.headline).toBe('Specialist 1 called http_request')
    expect(state.toolEvents).toHaveLength(1)
    expect(state.agentStates[0].status).toBe('waiting_tool')
  })

  it('keeps failed specialist events visible as the run headline', () => {
    const state = deriveTeamV4Observability({
      agents: [agent({})],
      events: [
        event({
          sequence: 1,
          event_type: 'specialist_execution_failed',
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
    expect(state.headline).toBe('Specialist 1 failed Root Task')
    expect(state.detail).toBe('Connection refused')
    expect(state.agentStates[0].status).toBe('failed')
  })

  it('uses terminal harness status ahead of stale running events for agent state', () => {
    const state = deriveTeamV4Observability({
      agents: [agent({})],
      events: [
        event({ sequence: 1, event_type: 'specialist_execution_started' }),
      ],
      harnessRuns: [
        harness({
          status: 'completed',
          updated_at: '2026-04-28T08:03:00Z',
        }),
      ],
      runState: 'running',
      tasks: [task({ status: 'completed' })],
    })

    expect(state.agentStates[0].status).toBe('completed')
    expect(state.agentStates[0].detail).toContain('Harness completed')
  })

  it('maps harness terminal events into observable phases', () => {
    const state = deriveTeamV4Observability({
      agents: [agent({})],
      events: [
        event({
          sequence: 2,
          event_type: 'harness_failed',
          payload: {
            error: 'task ledger incomplete',
          },
        }),
      ],
      harnessRuns: [harness({ status: 'failed' })],
      runState: 'failed',
      tasks: [task({ status: 'failed' })],
    })

    expect(state.phase).toBe('failed')
    expect(state.headline).toBe('Specialist 1 harness failed')
    expect(state.detail).toBe('task ledger incomplete')
  })

  it('maps expired harness events to warning activity and expired agent status', () => {
    const state = deriveTeamV4Observability({
      agents: [agent({})],
      events: [
        event({
          sequence: 3,
          event_type: 'harness_expired',
          payload: {
            harness_run_id: 'harness-1',
          },
        }),
      ],
      harnessRuns: [harness({ status: 'expired' })],
      runState: 'running',
      tasks: [task({ status: 'running' })],
    })

    expect(state.phase).toBe('expired')
    expect(state.agentStates[0].status).toBe('expired')
    expect(state.activeEvent?.severity).toBe('warning')
  })
})
