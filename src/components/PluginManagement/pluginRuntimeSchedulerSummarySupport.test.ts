import { describe, expect, it } from 'vitest'
import {
  getPendingPressureLevel,
  getRejectedTrendLevel,
  getRuntimeSummaryChipClass,
} from './pluginRuntimeSchedulerSummarySupport'

describe('pluginRuntimeSchedulerSummarySupport', () => {
  it('classifies pending pressure by queue saturation ratio', () => {
    expect(getPendingPressureLevel({ pendingCount: 2, configuredMaxQueueDepth: 20 })).toBe(
      'neutral'
    )
    expect(getPendingPressureLevel({ pendingCount: 15, configuredMaxQueueDepth: 20 })).toBe(
      'warning'
    )
    expect(getPendingPressureLevel({ pendingCount: 19, configuredMaxQueueDepth: 20 })).toBe('error')
  })

  it('classifies rejected trend by recent cumulative growth', () => {
    expect(getRejectedTrendLevel([0, 0, 0, 0])).toBe('neutral')
    expect(getRejectedTrendLevel([0, 0, 1, 1])).toBe('warning')
    expect(getRejectedTrendLevel([0, 1, 2, 3])).toBe('error')
    expect(getRejectedTrendLevel([5])).toBe('neutral')
  })

  it('returns chip classes for each summary level', () => {
    expect(getRuntimeSummaryChipClass('neutral')).toContain('bg-base-200/70')
    expect(getRuntimeSummaryChipClass('warning')).toContain('text-warning')
    expect(getRuntimeSummaryChipClass('error')).toContain('text-error')
  })
})
