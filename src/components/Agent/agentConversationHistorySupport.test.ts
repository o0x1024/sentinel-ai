import { describe, expect, it } from 'vitest'

import { buildConversationTimeline } from './agentConversationHistorySupport'

describe('agentConversationHistorySupport', () => {
  it('filters persisted agent task update system messages', () => {
    const timeline = buildConversationTimeline(
      [
        {
          id: 'msg-task-1',
          role: 'system',
          content: '已建立 2 个执行任务：读取 openapi.json、分析 API 结构',
          metadata: JSON.stringify({
            kind: 'agent_task_update',
            task_event_type: 'planned',
            task_count: 2,
            task_preview: '读取 openapi.json、分析 API 结构',
          }),
          timestamp: '2026-04-18T00:00:00Z',
        },
      ],
      {
        toolCallCompletedLabel: 'Tool call completed',
        shouldSuppressTeamMirrorNoiseMessage: () => false,
      },
    )

    expect(timeline).toHaveLength(0)
  })

  it('renders persisted tools_activated system messages with runtime hint', () => {
    const timeline = buildConversationTimeline(
      [
        {
          id: 'msg-1',
          role: 'system',
          content: 'Deferred tools activated',
          metadata: JSON.stringify({
            kind: 'tools_activated',
            tool_ids: ['file_read'],
            tools: ['tool_search', 'file_read'],
            query: 'read changed file',
            runtime_hint: 'recent file changes detected; prefer readback',
          }),
          timestamp: '2026-04-17T00:00:00Z',
        },
      ],
      {
        toolCallCompletedLabel: 'Tool call completed',
        shouldSuppressTeamMirrorNoiseMessage: () => false,
      },
    )

    expect(timeline).toHaveLength(1)
    expect(timeline[0].type).toBe('system')
    expect(timeline[0].content).toContain('Deferred tools activated: file_read')
    expect(timeline[0].content).toContain('Active toolset: tool_search, file_read')
    expect(timeline[0].content).toContain(
      'Reason: recent file changes detected; prefer readback',
    )
    expect(timeline[0].metadata?.kind).toBe('tools_activated')
  })

  it('strips legacy tool previews from persisted skill_loaded system messages', () => {
    const timeline = buildConversationTimeline(
      [
        {
          id: 'msg-skill-1',
          role: 'system',
          content: '',
          metadata: JSON.stringify({
            kind: 'skill_loaded',
            skill_id: 'agent-browser',
            skill_name: 'agent-browser',
            tools: ['skills', 'tasks', 'http_request'],
            tools_preview: 'skills, tasks, http_request',
          }),
          timestamp: '2026-04-20T00:00:00Z',
        },
      ],
      {
        toolCallCompletedLabel: 'Tool call completed',
        shouldSuppressTeamMirrorNoiseMessage: () => false,
      },
    )

    expect(timeline).toHaveLength(1)
    expect(timeline[0].type).toBe('system')
    expect(timeline[0].content).toBe('Skill loaded: agent-browser (agent-browser)')
    expect(timeline[0].metadata?.kind).toBe('skill_loaded')
    expect(timeline[0].metadata?.tools).toBeUndefined()
    expect(timeline[0].metadata?.tools_preview).toBeUndefined()
  })
})
