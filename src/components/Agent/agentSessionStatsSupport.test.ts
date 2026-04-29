import { describe, expect, it } from 'vitest'
import {
  buildAgentSessionStats,
  formatSessionDuration,
  formatTokenRate,
} from './agentSessionStatsSupport'

describe('agentSessionStatsSupport', () => {
  it('builds token and timing stats for a completed assistant turn', () => {
    expect(buildAgentSessionStats({
      startedAt: 1_000,
      endedAt: 5_000,
      firstResponseAt: 1_650,
      inputTokens: 1_200,
      outputTokens: 400,
    })).toEqual({
      duration_ms: 4_000,
      first_response_ms: 650,
      input_tokens: 1_200,
      output_tokens: 400,
      total_tokens: 1_600,
      tokens_per_second: 100,
    })
  })

  it('requires timing and usage values', () => {
    expect(buildAgentSessionStats({
      startedAt: 1_000,
      endedAt: 5_000,
      inputTokens: undefined,
      outputTokens: 400,
    })).toBeNull()
  })

  it('formats duration and token rate for compact display', () => {
    expect(formatSessionDuration(950)).toBe('950ms')
    expect(formatSessionDuration(1_250)).toBe('1.3s')
    expect(formatSessionDuration(65_000)).toBe('1m 5s')
    expect(formatTokenRate(8.456)).toBe('8.46')
    expect(formatTokenRate(28.456)).toBe('28.5')
  })
})
