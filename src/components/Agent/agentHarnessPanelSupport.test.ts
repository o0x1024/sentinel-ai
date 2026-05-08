import { describe, expect, it } from 'vitest'

import {
  collapsedHarnessRuns,
  hiddenHarnessRunCount,
  hiddenTimelineItemCount,
  resolveHarnessSelectedRunId,
  timelineItemsForDisplay,
} from './agentHarnessPanelSupport'

const run = (id: string, generation: number, state: string) => ({
  id,
  generation,
  state,
})

describe('agentHarnessPanelSupport', () => {
  it('keeps running runs and the latest historical run when collapsed', () => {
    const runs = [
      run('generation-3', 3, 'running'),
      run('generation-2-failed', 2, 'failed'),
      run('generation-2-succeeded', 2, 'succeeded'),
      run('generation-1', 1, 'succeeded'),
    ]

    expect(collapsedHarnessRuns(runs).map((item) => item.id)).toEqual([
      'generation-3',
      'generation-2-failed',
    ])
    expect(hiddenHarnessRunCount(runs, false)).toBe(2)
  })

  it('shows only the latest run when there is no active run', () => {
    const runs = [
      run('generation-3', 3, 'failed'),
      run('generation-2', 2, 'succeeded'),
    ]

    expect(collapsedHarnessRuns(runs).map((item) => item.id)).toEqual(['generation-3'])
  })

  it('moves selection back to a visible run when history is collapsed', () => {
    const runs = [
      run('generation-3', 3, 'running'),
      run('generation-2', 2, 'failed'),
      run('generation-1', 1, 'succeeded'),
    ]

    expect(resolveHarnessSelectedRunId(runs, 'generation-1', false)).toBe('generation-3')
    expect(resolveHarnessSelectedRunId(runs, 'generation-1', true)).toBe('generation-1')
  })

  it('can repin automatic selection to the newest visible run', () => {
    const runs = [
      run('generation-4-stalled', 4, 'stalled_without_ledger_progress'),
      run('generation-3-succeeded', 3, 'succeeded'),
    ]

    expect(resolveHarnessSelectedRunId(runs, 'generation-3-succeeded', true, true)).toBe('generation-3-succeeded')
    expect(resolveHarnessSelectedRunId(runs, 'generation-3-succeeded', true, false)).toBe('generation-4-stalled')
  })

  it('keeps stalled runs visible as the latest historical run', () => {
    const runs = [
      run('generation-4-stalled', 4, 'stalled_without_ledger_progress'),
      run('generation-3-succeeded', 3, 'succeeded'),
    ]

    expect(collapsedHarnessRuns(runs).map((item) => item.id)).toEqual(['generation-4-stalled'])
  })

  it('shows the latest timeline items first by default', () => {
    const items = ['event-1', 'event-2', 'event-3', 'event-4', 'event-5']

    expect(timelineItemsForDisplay(items, false)).toEqual(['event-5', 'event-4', 'event-3'])
    expect(timelineItemsForDisplay(items, true)).toEqual(['event-5', 'event-4', 'event-3', 'event-2', 'event-1'])
    expect(hiddenTimelineItemCount(items, false)).toBe(2)
  })
})
