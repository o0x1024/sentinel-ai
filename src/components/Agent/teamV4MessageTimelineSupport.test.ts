import { describe, expect, it } from 'vitest'
import type { AgentMessage } from '@/types/agent'
import type { TeamV4Agent, TeamV4Event, TeamV4Task } from '@/types/teamRuntime'
import {
  appendTeamV4TimelineMessages,
  buildTeamV4TimelineMessages,
  shouldIncludeTeamV4TimelineEventInMainFlow,
} from './teamV4MessageTimelineSupport'

const agent: TeamV4Agent = {
  id: 'agent-1',
  run_id: 'run-1',
  profile_id: 'assistant.default',
  role_type: 'specialist',
  name: 'Specialist 1',
  status: 'running',
  model: null,
  context_mode: null,
  tool_policy_json: {},
  metadata: {},
  created_at: '2026-05-09T00:00:00Z',
  updated_at: '2026-05-09T00:00:00Z',
}

const task: TeamV4Task = {
  id: 'task-1',
  run_id: 'run-1',
  parent_task_id: null,
  task_key: 'audit',
  title: 'Audit crypto path',
  instruction: 'Audit it',
  status: 'running',
  priority: 1,
  assigned_agent_id: 'agent-1',
  depends_on: [],
  acceptance_criteria: null,
  context_snapshot_id: null,
  metadata: {},
  created_at: '2026-05-09T00:00:00Z',
  updated_at: '2026-05-09T00:00:00Z',
}

const event = (overrides: Partial<TeamV4Event>): TeamV4Event => ({
  id: 'event-1',
  run_id: 'run-1',
  sequence: 1,
  actor_id: 'agent-1',
  task_id: 'task-1',
  event_type: 'specialist_execution_started',
  visibility: 'workspace',
  payload: {},
  created_at: '2026-05-09T00:00:01Z',
  ...overrides,
})

describe('teamV4MessageTimelineSupport', () => {
  it('filters low-signal planning events out of the main flow', () => {
    expect(
      shouldIncludeTeamV4TimelineEventInMainFlow(
        event({
          event_type: 'specialist_scheduler_started',
        }),
      ),
    ).toBe(false)
    expect(
      shouldIncludeTeamV4TimelineEventInMainFlow(
        event({
          event_type: 'specialist_execution_started',
        }),
      ),
    ).toBe(false)
    expect(
      shouldIncludeTeamV4TimelineEventInMainFlow(
        event({
          event_type: 'specialist_execution_completed',
        }),
      ),
    ).toBe(false)
  })

  it('keeps high-signal tool and failure events in the main flow', () => {
    expect(
      shouldIncludeTeamV4TimelineEventInMainFlow(
        event({
          event_type: 'specialist_tool_started',
        }),
      ),
    ).toBe(true)
    expect(
      shouldIncludeTeamV4TimelineEventInMainFlow(
        event({
          event_type: 'orchestrator_recovery_decision',
        }),
      ),
    ).toBe(true)
    expect(
      shouldIncludeTeamV4TimelineEventInMainFlow(
        event({
          event_type: 'specialist_execution_failed',
        }),
      ),
    ).toBe(true)
  })

  it('converts specialist tool events into main-flow messages', () => {
    const messages = buildTeamV4TimelineMessages({
      agents: [agent],
      tasks: [task],
      events: [
        event({
          id: 'event-tool',
          event_type: 'specialist_tool_started',
          payload: { toolName: 'grep', toolCallId: 'tool-1' },
        }),
      ],
    })

    expect(messages).toHaveLength(1)
    expect(messages[0].type).toBe('tool_call')
    expect(messages[0].content).toContain('grep')
    expect(messages[0].metadata?.kind).toBe('team_v4_timeline')
    expect(messages[0].metadata?.team_member_name).toBe('Specialist 1')
  })

  it('merges specialist tool start and result into one reusable tool block', () => {
    const timeline = buildTeamV4TimelineMessages({
      agents: [agent],
      tasks: [task],
      events: [
        event({
          id: 'event-tool-start',
          event_type: 'specialist_tool_started',
          payload: {
            toolCallId: 'tool-1',
            toolName: 'shell',
            arguments: '{"command":"pwd"}',
          },
        }),
        event({
          id: 'event-tool-result',
          sequence: 2,
          created_at: '2026-05-09T00:00:03Z',
          event_type: 'specialist_tool_result',
          payload: {
            toolCallId: 'tool-1',
            toolName: 'shell',
            arguments: '{"command":"pwd"}',
            result:
              '[{"type":"text","text":"{\\"stdout\\":\\"/workspace\\\\n\\",\\"stderr\\":\\"\\"}"}]',
            success: true,
          },
        }),
      ],
    })

    const merged = appendTeamV4TimelineMessages([], timeline)
    expect(merged).toHaveLength(1)
    expect(merged[0].id).toBe('teamv4:tool:run-1:tool-1')
    expect(merged[0].type).toBe('tool_call')
    expect(merged[0].content).toBe('工具调用完成: shell')
    expect(merged[0].metadata?.status).toBe('completed')
    expect(merged[0].metadata?.tool_name).toBe('shell')
    expect(merged[0].metadata?.tool_args).toEqual({ command: 'pwd' })
    expect(merged[0].metadata?.tool_result).toContain('stdout')
  })

  it('drops scheduler-only events from timeline message generation', () => {
    const messages = buildTeamV4TimelineMessages({
      agents: [agent],
      tasks: [task],
      events: [
        event({
          id: 'event-scheduler',
          event_type: 'specialist_scheduler_started',
        }),
      ],
    })

    expect(messages).toEqual([])
  })

  it('deduplicates timeline messages by id when appending', () => {
    const existing: AgentMessage[] = [
      {
        id: 'teamv4:event:event-1',
        type: 'error',
        content: 'old',
        timestamp: 1,
        metadata: { kind: 'team_v4_timeline' },
      },
    ]
    const timeline = buildTeamV4TimelineMessages({
      agents: [agent],
      tasks: [task],
      events: [
        event({ id: 'event-1', event_type: 'specialist_execution_failed' }),
        event({
          id: 'event-2',
          event_type: 'orchestrator_recovery_decision',
          sequence: 2,
          created_at: '2026-05-09T00:00:02Z',
        }),
      ],
    })

    const merged = appendTeamV4TimelineMessages(existing, timeline)
    expect(merged.map((message) => message.id)).toEqual([
      'teamv4:event:event-1',
      'teamv4:event:event-2',
    ])
  })

  it('updates an existing timeline message in place when a duplicate id arrives', () => {
    const existing: AgentMessage[] = [
      {
        id: 'teamv4:tool:run-1:tool-1',
        type: 'tool_call',
        content: '正在调用工具: shell',
        timestamp: 1,
        metadata: {
          kind: 'team_v4_timeline',
          status: 'running',
          tool_name: 'shell',
          tool_call_id: 'tool-1',
        },
      },
    ]

    const merged = appendTeamV4TimelineMessages(existing, [
      {
        id: 'teamv4:tool:run-1:tool-1',
        type: 'tool_call',
        content: '工具调用完成: shell',
        timestamp: 999,
        metadata: {
          kind: 'team_v4_timeline',
          status: 'completed',
          tool_name: 'shell',
          tool_call_id: 'tool-1',
          tool_result: '{"stdout":"ok"}',
        },
      },
    ])

    expect(merged).toHaveLength(1)
    expect(merged[0].timestamp).toBe(1)
    expect(merged[0].content).toBe('工具调用完成: shell')
    expect(merged[0].metadata?.status).toBe('completed')
    expect(merged[0].metadata?.tool_result).toBe('{"stdout":"ok"}')
  })
})
