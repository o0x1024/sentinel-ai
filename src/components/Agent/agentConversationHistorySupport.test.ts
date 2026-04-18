import { describe, expect, it } from 'vitest'

import { buildConversationTimeline } from './agentConversationHistorySupport'

describe('agentConversationHistorySupport', () => {
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
})
